//! 문단 텍스트·오프셋 접근과 필드 끝점 경계 규칙.

use super::{
    FieldEndpoint, FieldId, FieldRange, FieldRanges, HwpxFieldMarker, Paragraph, ParagraphEdit,
};
use crate::model::control::Control;
use std::ops::Range;

impl Paragraph {
    /// 문단 텍스트의 읽기 전용 뷰.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 텍스트 문자별 원본 UTF-16 위치의 읽기 전용 뷰.
    pub fn char_offsets(&self) -> &[u32] {
        &self.char_offsets
    }

    /// 파싱·새 문단 구성용. 기존 끝점을 가진 문단은 이 생성자로 변경하지 않는다.
    pub fn with_text(text: String, char_offsets: Vec<u32>, markers: Vec<HwpxFieldMarker>) -> Self {
        Self {
            text,
            char_offsets,
            hwpx_field_markers: Self::ordered_markers(markers),
            ..Self::default()
        }
    }

    /// Parser boundary: consumes the metadata owner and requires endpoint state.
    pub fn with_parsed_text(
        mut self,
        text: String,
        offsets: Vec<u32>,
        markers: Vec<HwpxFieldMarker>,
    ) -> Self {
        self.text = text;
        self.char_offsets = offsets;
        self.hwpx_field_markers = Self::ordered_markers(markers);
        self
    }

    fn ordered_markers(mut markers: Vec<HwpxFieldMarker>) -> Vec<HwpxFieldMarker> {
        markers.sort_by_key(|m| (m.char_idx, m.order));
        markers
    }

    pub fn field_markers(&self) -> &[HwpxFieldMarker] {
        &self.hwpx_field_markers
    }

    /// Same-paragraph ranges are a view, never another position store.
    pub fn field_ranges(&self) -> FieldRanges {
        FieldRanges(
            self.hwpx_field_markers
                .iter()
                .filter_map(|end| {
                    if end.endpoint != FieldEndpoint::End {
                        return None;
                    }
                    let begin = self
                        .hwpx_field_markers
                        .iter()
                        .find(|m| m.id == end.id && m.endpoint == FieldEndpoint::Begin)?;
                    let control_idx = match &end.id {
                        FieldId::Binary(ci) => *ci,
                        FieldId::Hwpx(id) => self.controls.iter().position(|ctrl| {
                            matches!(ctrl, Control::Field(f) if f.endpoint_id.as_ref() == Some(id))
                        })?,
                    };
                    Some(FieldRange {
                        start_char_idx: begin.char_idx,
                        end_char_idx: end.char_idx,
                        control_idx,
                    })
                })
                .collect(),
        )
    }

    /// Binary parsing keeps raw controls/offsets; only its range boundary converts.
    pub fn markers_from_binary_ranges(ranges: &[FieldRange]) -> Vec<HwpxFieldMarker> {
        let mut markers = Vec::with_capacity(ranges.len() * 2);
        let mut positions: Vec<_> = ranges
            .iter()
            .flat_map(|r| [r.start_char_idx, r.end_char_idx])
            .collect();
        positions.sort_unstable();
        positions.dedup();
        for pos in positions {
            let mut starts: Vec<_> = ranges.iter().filter(|r| r.start_char_idx == pos).collect();
            starts.sort_by_key(|r| r.control_idx);
            let mut starts = starts.into_iter().peekable();
            let mut order = 0;
            for end in ranges.iter().filter(|r| r.end_char_idx == pos) {
                while starts
                    .peek()
                    .is_some_and(|r| r.control_idx <= end.control_idx)
                {
                    if let Some(begin) = starts.next() {
                        markers.push(HwpxFieldMarker {
                            id: FieldId::Binary(begin.control_idx),
                            char_idx: pos,
                            endpoint: FieldEndpoint::Begin,
                            order,
                        });
                        order += 1;
                    }
                }
                markers.push(HwpxFieldMarker {
                    id: FieldId::Binary(end.control_idx),
                    char_idx: pos,
                    endpoint: FieldEndpoint::End,
                    order,
                });
                order += 1;
            }
            for begin in starts {
                markers.push(HwpxFieldMarker {
                    id: FieldId::Binary(begin.control_idx),
                    char_idx: pos,
                    endpoint: FieldEndpoint::Begin,
                    order,
                });
                order += 1;
            }
        }
        markers
    }

    pub fn edit_checkpoint(&self) -> ParagraphEdit {
        ParagraphEdit {
            before: Box::new(self.clone()),
            deleted_chars: 0,
        }
    }

    /// Consumes an inverse and returns its exact inverse for redo.
    pub(crate) fn undo_edit(&mut self, edit: ParagraphEdit) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        *self = *edit.before;
        inverse
    }

    fn field_id(&self, control_idx: usize) -> Option<FieldId> {
        Some(match self.controls.get(control_idx)? {
            Control::Field(f) => match &f.endpoint_id {
                Some(id) => FieldId::Hwpx(id.clone()),
                None => FieldId::Binary(control_idx),
            },
            _ => return None,
        })
    }

    pub(crate) fn remove_field_range(&mut self, index: usize) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        if let Some(id) = self
            .field_ranges()
            .get(index)
            .and_then(|range| self.field_id(range.control_idx))
        {
            self.hwpx_field_markers.retain(|marker| marker.id != id);
        }
        inverse
    }

    /// 텍스트 인덱스는 유지하고 컨트롤 갭에 따른 원본 위치만 갱신한다.
    pub(crate) fn set_char_offsets(&mut self, offsets: Vec<u32>) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        self.char_offsets = offsets;
        inverse
    }

    /// 텍스트 전체 교체. 내부 끝점은 제거하고 저장 시 명시적 거부를 유도한다.
    pub(crate) fn replace_text(&mut self, text: String) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        let old_len = self.text.chars().count();
        self.shift_hwpx_field_markers(0..old_len, text.chars().count(), 0);
        self.text = text;
        let mut position = 0;
        self.char_offsets = self
            .text
            .chars()
            .map(|ch| {
                let start = position;
                position += if ch == '\t' {
                    8
                } else {
                    u32::try_from(ch.len_utf16()).unwrap_or(2)
                };
                start
            })
            .collect();
        self.char_count = position + 1;
        inverse
    }

    /// 문자 범위 교체. 삭제와 삽입의 경계 규칙은 보통 편집과 동일하다.
    pub(crate) fn replace_text_range(&mut self, range: Range<usize>, text: &str) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        self.delete_text_at(range.start, range.len());
        self.insert_text_at(range.start, text);
        inverse
    }

    /// Replaces only sequence elements strictly between the selected endpoints.
    pub(crate) fn replace_field_content(
        &mut self,
        control_idx: usize,
        text: &str,
    ) -> Option<ParagraphEdit> {
        let key = self.field_id(control_idx)?;
        let begin = self
            .hwpx_field_markers
            .iter()
            .position(|m| m.id == key && m.endpoint == FieldEndpoint::Begin)?;
        let end = self
            .hwpx_field_markers
            .iter()
            .position(|m| m.id == key && m.endpoint == FieldEndpoint::End)?;
        if begin >= end {
            return None;
        }
        let inverse = self.edit_checkpoint();
        let start_pos = self.hwpx_field_markers[begin].char_idx;
        let end_pos = self.hwpx_field_markers[end].char_idx;
        let mut markers = self.hwpx_field_markers.clone();
        markers.drain(begin + 1..end);
        for (slot, marker) in markers.iter_mut().enumerate() {
            if slot > begin {
                marker.char_idx = marker.char_idx - (end_pos - start_pos) + text.chars().count();
            }
        }
        self.delete_text_at(start_pos, end_pos - start_pos);
        self.insert_text_at(start_pos, text);
        self.hwpx_field_markers = markers;
        if matches!(key, FieldId::Hwpx(_)) {
            self.rebuild_hwpx_field_offsets();
        }
        Some(inverse)
    }

    pub(super) fn rebuild_hwpx_field_offsets(&mut self) {
        let non_fields = self
            .controls
            .iter()
            .filter(|c| !matches!(c, Control::Field(_)))
            .count();
        let mut offset = u32::try_from(non_fields)
            .unwrap_or(u32::MAX)
            .saturating_mul(8);
        self.char_offsets.clear();
        for (i, ch) in self.text.chars().enumerate() {
            offset += u32::try_from(
                self.hwpx_field_markers
                    .iter()
                    .filter(|m| m.char_idx == i)
                    .count(),
            )
            .unwrap_or(u32::MAX)
            .saturating_mul(8);
            self.char_offsets.push(offset);
            offset += if ch == '\t' {
                8
            } else {
                u32::try_from(ch.len_utf16()).unwrap_or(2)
            };
        }
        offset += u32::try_from(
            self.hwpx_field_markers
                .iter()
                .filter(|m| m.char_idx == self.char_offsets.len())
                .count(),
        )
        .unwrap_or(u32::MAX)
        .saturating_mul(8);
        self.char_count = offset + 1;
    }

    // 삭제 [s,e): m <= s 유지, m >= e 왼쪽 이동, s < m < e 제거.
    // Insert p: text precedes the FIRST marker at p. Thus [E1][B2] extends
    // field 1, and a begin (including [B][E] of an empty field) follows typing.
    pub(super) fn field_insertion_slot(&self, position: usize) -> usize {
        let first = self
            .hwpx_field_markers
            .partition_point(|m| m.char_idx < position);
        // Binary HWP's legacy cursor is after leading FIELD_BEGIN controls.
        // Translate that cursor to a sequence slot without a second position store.
        first
            + self.hwpx_field_markers[first..]
                .iter()
                .take_while(|m| {
                    m.char_idx == position
                        && m.endpoint == FieldEndpoint::Begin
                        && matches!(m.id, FieldId::Binary(_))
                })
                .count()
    }

    pub(super) fn shift_hwpx_field_markers(
        &mut self,
        deleted: Range<usize>,
        inserted: usize,
        insertion_slot: usize,
    ) {
        self.hwpx_field_markers
            .retain(|marker| !(deleted.start < marker.char_idx && marker.char_idx < deleted.end));
        for (slot, marker) in self.hwpx_field_markers.iter_mut().enumerate() {
            if marker.char_idx >= deleted.end {
                marker.char_idx -= deleted.len();
            }
            if marker.char_idx > deleted.start
                || (marker.char_idx == deleted.start && slot >= insertion_slot)
            {
                marker.char_idx += inserted;
            }
        }
        // Deletion may collapse several positions. Preserve sequence order at
        // that position rather than re-sorting by their former local order.
        for (order, marker) in self.hwpx_field_markers.iter_mut().enumerate() {
            marker.order = order;
        }
    }
}
