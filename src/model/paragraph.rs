//! 문단 (Paragraph, CharRun, LineSeg, RangeTag)

use super::control::Control;

mod text;

/// 문단 (HWPTAG_PARA_HEADER + 하위 레코드)
///
/// 텍스트와 원본 오프셋은 끝점 소유자만 변경한다.
/// ```compile_fail,E0616
/// let mut paragraph = rhwp::model::paragraph::Paragraph::default();
/// paragraph.text = String::new();
/// ```
/// ```compile_fail,E0616
/// let mut paragraph = rhwp::model::paragraph::Paragraph::default();
/// paragraph.char_offsets = Vec::new();
/// ```
///
/// ```compile_fail
/// use rhwp::model::paragraph::Paragraph;
/// let paragraph = Paragraph::with_text("text".into(), vec![0,1,2,3]);
/// ```
///
/// ```compile_fail
/// use rhwp::model::paragraph::Paragraph;
/// let mut paragraph = Paragraph::default();
/// paragraph.field_ranges = Vec::new();
/// ```
///
/// ```compile_fail
/// use rhwp::model::paragraph::Paragraph;
/// let mut paragraph = Paragraph::default();
/// paragraph.field_markers().clear();
/// ```
///
/// ```compile_fail
/// let mut paragraph = rhwp::model::paragraph::Paragraph::default();
/// paragraph.field_ranges().push(rhwp::model::paragraph::FieldRange {
///     start_char_idx: 0, end_char_idx: 0, control_idx: 0,
/// });
/// ```
#[derive(Debug, Default, Clone)]
pub struct Paragraph {
    /// 문자 수 (제어 문자 포함)
    pub char_count: u32,
    /// 컨트롤 마스크
    pub control_mask: u32,
    /// 문단 모양 ID 참조
    pub para_shape_id: u16,
    /// 문단 스타일 ID 참조
    pub style_id: u8,
    /// 단 나누기 종류
    pub column_type: ColumnBreakType,
    /// 원본 break_type 바이트 (라운드트립 보존용, 0이면 column_type에서 재구성)
    pub raw_break_type: u8,
    /// 문단 텍스트 (UTF-16에서 변환된 문자열)
    text: String,
    /// 텍스트 문자별 UTF-16 코드 유닛 위치 (LineSeg/CharShapeRef 위치와 매핑용)
    /// char_offsets[i] = text[i]에 해당하는 원본 UTF-16 코드 유닛 인덱스
    char_offsets: Vec<u32>,
    /// 글자 모양 변경 위치 목록
    pub char_shapes: Vec<CharShapeRef>,
    /// 줄 레이아웃 정보
    pub line_segs: Vec<LineSeg>,
    /// 영역 태그 정보
    pub range_tags: Vec<RangeTag>,
    /// Text and zero-width field endpoints form one sequence, ordered by (char_idx, order).
    hwpx_field_markers: Vec<HwpxFieldMarker>,
    /// 컨트롤 목록 (표, 그림, 각주 등)
    pub controls: Vec<Control>,
    /// 각 컨트롤에 대응하는 CTRL_DATA 레코드 (라운드트립 보존용)
    /// controls[i]에 대응하는 CTRL_DATA가 있으면 ctrl_data_records[i] = Some(data)
    pub ctrl_data_records: Vec<Option<Vec<u8>>>,
    /// char_count의 최상위 비트 (bit 31) 보존 (라운드트립용)
    pub char_count_msb: bool,
    /// PARA_HEADER 레코드의 12바이트 이후 추가 바이트 (라운드트립 보존용)
    /// numCharShapes, numRangeTags, numLineSegs, instanceId 등
    pub raw_header_extra: Vec<u8>,
    /// 원본에 PARA_TEXT 레코드가 존재했는지 (라운드트립 보존용)
    pub has_para_text: bool,
    /// TAB 확장 데이터 (라운드트립 보존용)
    /// 각 탭 문자의 7 code unit (탭 너비, 종류 등) — text 내 '\t' 순서와 1:1 대응
    pub tab_extended: Vec<[u16; 7]>,
    /// 문단 번호 시작 방식 오버라이드
    /// None = 앞 번호 목록에 이어 (기본)
    /// Some(NumberingRestart) = 이전 번호 이어 / 새 번호 시작
    pub numbering_restart: Option<NumberingRestart>,
}

/// 문단 번호 시작 방식
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberingRestart {
    /// 이전 번호 목록에 이어 (다른 번호 체계 후 복귀 시 이전 카운터 복원)
    ContinuePrevious,
    /// 새 번호 목록 시작 (지정 값부터)
    NewStart(u32),
}

/// 문단 텍스트 내 제어 문자 종류
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CtrlChar {
    /// 구역 정의/단 정의
    SectionColumnDef,
    /// 필드 시작
    FieldBegin,
    /// 필드 끝
    FieldEnd,
    /// 탭
    Tab,
    /// 줄 끝 (line break)
    LineBreak,
    /// 그리기 개체/표
    DrawTableObject,
    /// 문단 끝 (para break)
    ParaBreak,
    /// 숨은 설명
    HiddenComment,
    /// 머리말/꼬리말
    HeaderFooter,
    /// 각주/미주
    FootnoteEndnote,
    /// 자동 번호
    AutoNumber,
    /// 페이지 컨트롤
    PageControl,
    /// 책갈피
    Bookmark,
    /// 덧말/글자겹침
    Ruby,
    /// 하이픈
    Hyphen,
    /// 묶음 빈칸
    NonBreakingSpace,
    /// 고정폭 빈칸
    FixedWidthSpace,
    /// 일반 문자
    Char(char),
}

/// 단 나누기 종류
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ColumnBreakType {
    #[default]
    None,
    /// 구역 나누기
    Section,
    /// 다단 나누기
    MultiColumn,
    /// 쪽 나누기
    Page,
    /// 단 나누기
    Column,
}

/// 글자 모양 참조 (문단 내 위치별 글자 모양)
#[derive(Debug, Clone, Default)]
pub struct CharShapeRef {
    /// 글자 모양이 바뀌는 시작 위치
    pub start_pos: u32,
    /// 글자 모양 ID
    pub char_shape_id: u32,
}

/// 줄 레이아웃 정보 (HWPTAG_PARA_LINE_SEG)
#[derive(Debug, Clone, Default)]
pub struct LineSeg {
    /// 텍스트 시작 위치
    pub text_start: u32,
    /// 줄의 세로 위치
    pub vertical_pos: i32,
    /// 줄의 높이
    pub line_height: i32,
    /// 텍스트 부분의 높이
    pub text_height: i32,
    /// 베이스라인까지 거리
    pub baseline_distance: i32,
    /// 줄간격
    pub line_spacing: i32,
    /// 컬럼에서의 시작 위치
    pub column_start: i32,
    /// 세그먼트 폭
    pub segment_width: i32,
    /// 태그 플래그
    pub tag: u32,
}

impl LineSeg {
    /// 페이지의 첫 줄인지 여부
    pub fn is_first_line_of_page(&self) -> bool {
        self.tag & 0x01 != 0
    }

    /// 컬럼의 첫 줄인지 여부
    pub fn is_first_line_of_column(&self) -> bool {
        self.tag & 0x02 != 0
    }
}

/// 영역 태그 (HWPTAG_PARA_RANGE_TAG)
#[derive(Debug, Clone, Default)]
pub struct RangeTag {
    /// 영역 시작
    pub start: u32,
    /// 영역 끝
    pub end: u32,
    /// 태그 (상위 8비트: 종류, 하위 24비트: 데이터)
    pub tag: u32,
}

/// 필드 텍스트 범위 (0x03 FIELD_BEGIN ~ 0x04 FIELD_END 사이 텍스트)
#[derive(Debug, Clone, Default)]
pub struct FieldRange {
    /// text 문자열 내 시작 인덱스 (포함)
    pub start_char_idx: usize,
    /// text 문자열 내 끝 인덱스 (미포함)
    pub end_char_idx: usize,
    /// controls[] 배열 내 인덱스 (해당 Field 컨트롤 참조)
    pub control_idx: usize,
}

/// Immutable derived ranges; edits must go through the paragraph owner.
#[derive(Debug)]
pub struct FieldRanges(Vec<FieldRange>);

impl std::ops::Deref for FieldRanges {
    type Target = [FieldRange];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a FieldRanges {
    type Item = &'a FieldRange;
    type IntoIter = std::slice::Iter<'a, FieldRange>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldEndpoint {
    Begin,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldId {
    Hwpx(String),
    Binary(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HwpxFieldMarker {
    pub id: FieldId,
    pub char_idx: usize,
    pub endpoint: FieldEndpoint,
    pub order: usize,
}

/// Exact inverse of a paragraph-owner edit. The complete prior state includes
/// text, endpoint sequence slots, raw offsets, controls, and formatting.
#[derive(Debug, Clone)]
pub struct ParagraphEdit {
    before: Box<Paragraph>,
    pub deleted_chars: usize,
}

#[derive(Debug)]
pub struct ParagraphSplit {
    pub paragraph: Paragraph,
    pub inverse: ParagraphEdit,
}

#[derive(Debug)]
pub struct ParagraphMerge {
    pub offset: usize,
    pub inverse: ParagraphEdit,
}

impl Paragraph {
    fn control_anchor_positions(&self) -> Vec<usize> {
        let mut anchors = Vec::new();
        let mut previous_end = 0;
        for (index, ch) in self.text.chars().enumerate() {
            let offset = self.char_offsets[index];
            let field_gap = self
                .hwpx_field_markers
                .iter()
                .filter(|marker| marker.char_idx == index)
                .count()
                * 8;
            let gap = usize::try_from(offset.saturating_sub(previous_end))
                .unwrap_or(0)
                .saturating_sub(field_gap);
            anchors.extend(std::iter::repeat_n(index, gap / 8));
            previous_end = offset
                + if ch == '\t' {
                    8
                } else {
                    u32::try_from(ch.len_utf16()).unwrap_or(2)
                };
        }
        let count = self
            .controls
            .iter()
            .filter(|control| !matches!(control, Control::Field(_)))
            .count();
        anchors.resize(count, self.text.chars().count());
        anchors
    }

    fn control_in_selection(
        control: &Control,
        start: (usize, usize),
        end: (usize, usize),
        position: (usize, usize),
    ) -> bool {
        !matches!(control, Control::SectionDef(_) | Control::ColumnDef(_))
            && start <= position
            && position < end
    }

    fn selected_fields(
        source: &[Paragraph],
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<(usize, usize, FieldId)> {
        let mut included = Vec::new();
        for (pi, para) in source.iter().enumerate() {
            for (ci, control) in para.controls.iter().enumerate() {
                let Control::Field(field) = control else {
                    continue;
                };
                let id = field
                    .endpoint_id
                    .as_ref()
                    .map_or(FieldId::Binary(ci), |id| FieldId::Hwpx(id.clone()));
                let mut begin = None;
                let mut finish = None;
                for (index, p) in source.iter().enumerate() {
                    for m in p.field_markers() {
                        if m.id == id && (!matches!(id, FieldId::Binary(_)) || index == pi) {
                            let endpoint = Some((m.endpoint, (index, m.char_idx)));
                            match m.endpoint {
                                FieldEndpoint::Begin => begin = endpoint,
                                FieldEndpoint::End => finish = endpoint,
                            }
                        }
                    }
                }
                if matches!(
                    (begin, finish),
                    (Some((_, b)), Some((_, e))) if start <= b && b <= e && e <= end && b < end
                ) {
                    included.push((pi, ci, id));
                }
            }
        }
        included
    }

    fn rebuild_offsets_with_control_anchors(&mut self, kept_non_field_anchors: &[usize]) {
        let text_len = self.text.chars().count();
        let mut offset = 0u32;
        self.char_offsets.clear();
        for (i, ch) in self.text.chars().enumerate() {
            let gap_slots = kept_non_field_anchors.iter().filter(|&&a| a == i).count()
                + self
                    .hwpx_field_markers
                    .iter()
                    .filter(|m| m.char_idx == i)
                    .count();
            offset += u32::try_from(gap_slots)
                .unwrap_or(u32::MAX)
                .saturating_mul(8);
            self.char_offsets.push(offset);
            offset += if ch == '\t' {
                8
            } else {
                u32::try_from(ch.len_utf16()).unwrap_or(2)
            };
        }
        let trailing_slots = kept_non_field_anchors
            .iter()
            .filter(|&&a| a >= text_len)
            .count()
            + self
                .hwpx_field_markers
                .iter()
                .filter(|m| m.char_idx == self.char_offsets.len())
                .count();
        offset += u32::try_from(trailing_slots)
            .unwrap_or(u32::MAX)
            .saturating_mul(8);
        self.char_count = offset + 1;
    }

    pub fn clipboard_fragment(
        source: &[Paragraph],
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<Paragraph> {
        let included = Self::selected_fields(source, start, end);
        source
            .iter()
            .enumerate()
            .take(end.0 + 1)
            .skip(start.0)
            .map(|(pi, original)| {
                let first = if pi == start.0 { start.1 } else { 0 };
                let last = if pi == end.0 {
                    end.1
                } else {
                    original.text.chars().count()
                };
                let mut para = original.clone();
                let styles: Vec<_> = (first..last)
                    .map(|i| original.char_shape_id_at(i).unwrap_or(0))
                    .collect();
                para.delete_text_at(last, original.text.chars().count() - last);
                para.delete_text_at(0, first);
                let mut control_map = Vec::new();
                let mut kept_anchors = Vec::new();
                para.controls.clear();
                para.ctrl_data_records.clear();
                let anchors = original.control_anchor_positions();
                let mut non_field = 0;
                for (ci, control) in original.controls.iter().enumerate() {
                    let selected = if matches!(control, Control::Field(_)) {
                        included.iter().any(|(p, c, _)| (*p, *c) == (pi, ci))
                    } else {
                        let anchor = anchors[non_field];
                        non_field += 1;
                        let keep = Self::control_in_selection(control, start, end, (pi, anchor));
                        if keep {
                            kept_anchors
                                .push(anchor.saturating_sub(first).min(para.text.chars().count()));
                        }
                        keep
                    };
                    if !selected {
                        continue;
                    }
                    control_map.push((ci, para.controls.len()));
                    para.controls.push(control.clone());
                    para.ctrl_data_records
                        .push(original.ctrl_data_records.get(ci).cloned().flatten());
                }
                para.hwpx_field_markers = original
                    .field_markers()
                    .iter()
                    .filter_map(|m| {
                        let belongs = included.iter().any(|(p, _, id)| {
                            *id == m.id && (!matches!(id, FieldId::Binary(_)) || *p == pi)
                        });
                        if !belongs || m.char_idx < first || m.char_idx > last {
                            return None;
                        }
                        let mut marker = m.clone();
                        marker.char_idx -= first;
                        if let FieldId::Binary(ci) = &mut marker.id {
                            *ci = control_map.iter().find(|(old, _)| old == ci)?.1;
                        }
                        Some(marker)
                    })
                    .collect();
                para.rebuild_offsets_with_control_anchors(&kept_anchors);
                para.char_shapes.clear();
                for (i, id) in styles.into_iter().enumerate() {
                    if para
                        .char_shapes
                        .last()
                        .is_none_or(|shape| shape.char_shape_id != id)
                    {
                        para.char_shapes.push(CharShapeRef {
                            start_pos: para.char_offsets[i],
                            char_shape_id: id,
                        });
                    }
                }
                para
            })
            .collect()
    }

    fn delete_paragraph_selection_slice(
        &mut self,
        pi: usize,
        start: (usize, usize),
        end: (usize, usize),
        removed_fields: &[(usize, usize, FieldId)],
    ) -> bool {
        let text_len = self.text.chars().count();
        let first = if pi == start.0 {
            start.1.min(text_len)
        } else {
            0
        };
        let last = if pi == end.0 {
            end.1.min(text_len)
        } else {
            text_len
        };
        let anchors = self.control_anchor_positions();
        let mut non_field = 0;
        let mut kept_controls = Vec::with_capacity(self.controls.len());
        let mut kept_records = Vec::with_capacity(self.controls.len());
        let mut control_map = Vec::with_capacity(self.controls.len());
        let mut removed_anchors = Vec::new();
        let mut control_removed = false;

        for (ci, control) in self.controls.iter().enumerate() {
            let remove = if matches!(control, Control::Field(_)) {
                removed_fields.iter().any(|(p, c, _)| (*p, *c) == (pi, ci))
            } else {
                let anchor = anchors[non_field];
                non_field += 1;
                let selected = Self::control_in_selection(control, start, end, (pi, anchor));
                if selected {
                    removed_anchors.push(anchor);
                }
                selected
            };
            if remove {
                control_removed = true;
                continue;
            }
            control_map.push((ci, kept_controls.len()));
            kept_controls.push(control.clone());
            kept_records.push(self.ctrl_data_records.get(ci).cloned().flatten());
        }
        self.controls = kept_controls;
        self.ctrl_data_records = kept_records;

        let mut removed_marker_positions = Vec::new();
        let mut kept_markers = Vec::with_capacity(self.hwpx_field_markers.len());
        for mut marker in self.hwpx_field_markers.drain(..) {
            let remove = removed_fields.iter().any(|(p, _, id)| {
                *id == marker.id && (!matches!(id, FieldId::Binary(_)) || *p == pi)
            });
            if remove {
                removed_marker_positions.push(marker.char_idx);
                continue;
            }
            if let FieldId::Binary(ci) = &mut marker.id {
                if let Some((_, new_ci)) = control_map.iter().find(|(old, _)| old == ci) {
                    *ci = *new_ci;
                }
            }
            kept_markers.push(marker);
        }
        self.hwpx_field_markers = kept_markers;

        for pos in removed_anchors
            .iter()
            .copied()
            .chain(removed_marker_positions.iter().copied())
        {
            let idx = pos.min(self.char_offsets.len());
            let threshold = self.char_offsets.get(idx).copied().unwrap_or(u32::MAX);
            for offset in &mut self.char_offsets[idx..] {
                *offset = offset.saturating_sub(8);
            }
            for cs in &mut self.char_shapes {
                if cs.start_pos >= threshold {
                    cs.start_pos = cs.start_pos.saturating_sub(8);
                }
            }
            for ls in &mut self.line_segs {
                if ls.text_start >= threshold {
                    ls.text_start = ls.text_start.saturating_sub(8);
                }
            }
            for rt in &mut self.range_tags {
                if rt.start >= threshold {
                    rt.start = rt.start.saturating_sub(8);
                }
                if rt.end >= threshold {
                    rt.end = rt.end.saturating_sub(8);
                }
            }
            self.char_count = self.char_count.saturating_sub(8);
        }

        let deleted_chars = self
            .delete_text_at(first, last.saturating_sub(first))
            .deleted_chars;

        if self.text.is_empty() && self.controls.is_empty() && self.hwpx_field_markers.is_empty() {
            self.has_para_text = false;
            self.control_mask = 0;
        }

        control_removed || !removed_marker_positions.is_empty() || deleted_chars > 0
    }

    pub(crate) fn delete_selection_range(
        paragraphs: &mut Vec<Paragraph>,
        raw_start: (usize, usize),
        raw_end: (usize, usize),
    ) -> bool {
        if raw_start.0 >= paragraphs.len()
            || raw_end.0 >= paragraphs.len()
            || raw_start.0 > raw_end.0
        {
            return false;
        }
        let start = (
            raw_start.0,
            raw_start.1.min(paragraphs[raw_start.0].text.chars().count()),
        );
        let end = (
            raw_end.0,
            raw_end.1.min(paragraphs[raw_end.0].text.chars().count()),
        );
        if start >= end {
            return false;
        }
        let removed_fields = Self::selected_fields(paragraphs, start, end);
        if start.0 == end.0 {
            return paragraphs[start.0].delete_paragraph_selection_slice(
                start.0,
                start,
                end,
                &removed_fields,
            );
        }
        paragraphs[end.0].delete_paragraph_selection_slice(end.0, start, end, &removed_fields);
        for mid in (start.0 + 1..end.0).rev() {
            paragraphs.remove(mid);
        }
        paragraphs[start.0].delete_paragraph_selection_slice(start.0, start, end, &removed_fields);
        if start.0 + 1 < paragraphs.len() {
            let next = paragraphs.remove(start.0 + 1);
            paragraphs[start.0].merge_from(&next);
        }
        true
    }

    pub(crate) fn remap_clipboard_fields(
        fragment: &mut [Paragraph],
        used: &mut std::collections::HashSet<String>,
        next: &mut u32,
    ) -> Result<(), crate::error::HwpError> {
        for para in fragment.iter() {
            for control in &para.controls {
                if let Control::Field(field) = control {
                    used.insert(field.field_id.to_string());
                    used.extend(field.endpoint_id.iter().cloned());
                }
            }
        }
        let mut identities = Vec::new();
        for (pi, para) in fragment.iter_mut().enumerate() {
            for (ci, control) in para.controls.iter_mut().enumerate() {
                let Control::Field(field) = control else {
                    continue;
                };
                loop {
                    *next = next.checked_add(1).ok_or_else(|| {
                        crate::error::HwpError::InvalidField("CLIPBOARD_FIELD_ID_EXHAUSTED".into())
                    })?;
                    if used.insert(next.to_string()) {
                        break;
                    }
                }
                let new = next.to_string();
                let old = field
                    .endpoint_id
                    .replace(new.clone())
                    .map_or(FieldId::Binary(ci), FieldId::Hwpx);
                identities.push((pi, old, new.clone()));
                field.field_id = *next;
                if let Some(hwpx) = &mut field.hwpx {
                    for (key, value) in &mut hwpx.begin_attributes {
                        if key == "id" || key == "fieldid" {
                            *value = new.clone();
                        }
                    }
                    if let Some(end) = &mut hwpx.end_attributes {
                        for (key, value) in end {
                            if key == "beginIDRef" || key == "fieldid" {
                                *value = new.clone();
                            }
                        }
                    }
                }
            }
        }
        for (pi, para) in fragment.iter_mut().enumerate() {
            for marker in &mut para.hwpx_field_markers {
                if let Some((_, _, id)) = identities.iter().find(|(p, old, _)| {
                    *old == marker.id && (!matches!(old, FieldId::Binary(_)) || *p == pi)
                }) {
                    marker.id = FieldId::Hwpx(id.clone());
                }
            }
        }
        for para in fragment {
            for control in &mut para.controls {
                match control {
                    Control::Table(table) => {
                        for cell in &mut table.cells {
                            Self::remap_clipboard_fields(&mut cell.paragraphs, used, next)?;
                        }
                        if let Some(caption) = &mut table.caption {
                            Self::remap_clipboard_fields(&mut caption.paragraphs, used, next)?;
                        }
                    }
                    Control::Shape(shape) => Self::remap_clipboard_shape(shape, used, next)?,
                    Control::Picture(picture) => {
                        if let Some(caption) = &mut picture.caption {
                            Self::remap_clipboard_fields(&mut caption.paragraphs, used, next)?;
                        }
                    }
                    Control::Header(header) => {
                        Self::remap_clipboard_fields(&mut header.paragraphs, used, next)?
                    }
                    Control::Footer(footer) => {
                        Self::remap_clipboard_fields(&mut footer.paragraphs, used, next)?
                    }
                    Control::Footnote(note) => {
                        Self::remap_clipboard_fields(&mut note.paragraphs, used, next)?
                    }
                    Control::Endnote(note) => {
                        Self::remap_clipboard_fields(&mut note.paragraphs, used, next)?
                    }
                    Control::HiddenComment(comment) => {
                        Self::remap_clipboard_fields(&mut comment.paragraphs, used, next)?
                    }
                    Control::SectionDef(_)
                    | Control::ColumnDef(_)
                    | Control::AutoNumber(_)
                    | Control::NewNumber(_)
                    | Control::PageNumberPos(_)
                    | Control::Bookmark(_)
                    | Control::Hyperlink(_)
                    | Control::Ruby(_)
                    | Control::CharOverlap(_)
                    | Control::PageHide(_)
                    | Control::Equation(_)
                    | Control::Field(_)
                    | Control::Form(_)
                    | Control::Unknown(_) => {}
                }
            }
        }
        Ok(())
    }

    fn remap_clipboard_shape(
        shape: &mut crate::model::shape::ShapeObject,
        used: &mut std::collections::HashSet<String>,
        next: &mut u32,
    ) -> Result<(), crate::error::HwpError> {
        if let Some(drawing) = shape.drawing_mut() {
            if let Some(text_box) = &mut drawing.text_box {
                Self::remap_clipboard_fields(&mut text_box.paragraphs, used, next)?;
            }
            if let Some(caption) = &mut drawing.caption {
                Self::remap_clipboard_fields(&mut caption.paragraphs, used, next)?;
            }
        }
        if let crate::model::shape::ShapeObject::Group(group) = shape {
            for child in &mut group.children {
                Self::remap_clipboard_shape(child, used, next)?;
            }
            if let Some(caption) = &mut group.caption {
                Self::remap_clipboard_fields(&mut caption.paragraphs, used, next)?;
            }
        }
        Ok(())
    }

    fn reserve_clipboard_ids(
        paragraphs: &[Paragraph],
        used: &mut std::collections::HashSet<String>,
    ) {
        for para in paragraphs {
            for marker in para.field_markers() {
                if let FieldId::Hwpx(id) = &marker.id {
                    used.insert(id.clone());
                }
            }
            for control in &para.controls {
                match control {
                    Control::Field(field) => {
                        used.insert(field.field_id.to_string());
                        used.extend(field.endpoint_id.iter().cloned());
                    }
                    Control::Table(table) => {
                        for cell in &table.cells {
                            Self::reserve_clipboard_ids(&cell.paragraphs, used);
                        }
                        if let Some(caption) = &table.caption {
                            Self::reserve_clipboard_ids(&caption.paragraphs, used);
                        }
                    }
                    Control::Shape(shape) => Self::reserve_clipboard_shape_ids(shape, used),
                    Control::Picture(picture) => {
                        if let Some(caption) = &picture.caption {
                            Self::reserve_clipboard_ids(&caption.paragraphs, used);
                        }
                    }
                    Control::Header(header) => {
                        Self::reserve_clipboard_ids(&header.paragraphs, used)
                    }
                    Control::Footer(footer) => {
                        Self::reserve_clipboard_ids(&footer.paragraphs, used)
                    }
                    Control::Footnote(note) => Self::reserve_clipboard_ids(&note.paragraphs, used),
                    Control::Endnote(note) => Self::reserve_clipboard_ids(&note.paragraphs, used),
                    Control::HiddenComment(comment) => {
                        Self::reserve_clipboard_ids(&comment.paragraphs, used)
                    }
                    Control::SectionDef(_)
                    | Control::ColumnDef(_)
                    | Control::AutoNumber(_)
                    | Control::NewNumber(_)
                    | Control::PageNumberPos(_)
                    | Control::Bookmark(_)
                    | Control::Hyperlink(_)
                    | Control::Ruby(_)
                    | Control::CharOverlap(_)
                    | Control::PageHide(_)
                    | Control::Equation(_)
                    | Control::Form(_)
                    | Control::Unknown(_) => {}
                }
            }
        }
    }

    fn reserve_clipboard_shape_ids(
        shape: &crate::model::shape::ShapeObject,
        used: &mut std::collections::HashSet<String>,
    ) {
        if let Some(drawing) = shape.drawing() {
            if let Some(text_box) = &drawing.text_box {
                Self::reserve_clipboard_ids(&text_box.paragraphs, used);
            }
            if let Some(caption) = &drawing.caption {
                Self::reserve_clipboard_ids(&caption.paragraphs, used);
            }
        }
        if let crate::model::shape::ShapeObject::Group(group) = shape {
            for child in &group.children {
                Self::reserve_clipboard_shape_ids(child, used);
            }
            if let Some(caption) = &group.caption {
                Self::reserve_clipboard_ids(&caption.paragraphs, used);
            }
        }
    }

    pub(crate) fn paste_fragment(
        target: &mut Vec<Paragraph>,
        paragraph: usize,
        offset: usize,
        fragment: &[Paragraph],
    ) -> Result<(usize, usize), crate::error::HwpError> {
        let owner = target.get(paragraph).ok_or_else(|| {
            crate::error::HwpError::RenderError("CLIPBOARD_PARAGRAPH_RANGE".into())
        })?;
        if offset > owner.text.chars().count() {
            return Err(crate::error::HwpError::RenderError(
                "CLIPBOARD_CHARACTER_RANGE".into(),
            ));
        }
        let mut fragment = fragment.to_vec();
        let mut used = std::collections::HashSet::new();
        let mut next = 0u32;
        Self::reserve_clipboard_ids(target, &mut used);
        Self::reserve_clipboard_ids(&fragment, &mut used);
        Self::remap_clipboard_fields(&mut fragment, &mut used, &mut next)?;
        let Some(first) = fragment.first() else {
            return Ok((paragraph, offset));
        };
        let last = paragraph + fragment.len() - 1;
        let slot = target[paragraph].field_insertion_slot(offset);
        let markers = target[paragraph].hwpx_field_markers.clone();
        let mut right = target[paragraph].split_at(offset).paragraph;
        target[paragraph].hwpx_field_markers = markers[..slot].to_vec();
        right.hwpx_field_markers = markers[slot..]
            .iter()
            .cloned()
            .map(|mut marker| {
                marker.char_idx -= offset;
                marker
            })
            .collect();
        target[paragraph].merge_from(first);
        for (index, para) in fragment.into_iter().enumerate().skip(1) {
            target.insert(paragraph + index, para);
        }
        let end = target[last].merge_from(&right).offset;
        for para in &mut target[paragraph..=last] {
            para.rebuild_hwpx_field_offsets();
        }
        Ok((last, end))
    }

    /// 빈 문단을 생성한다 (문단 끝 마커만 포함).
    ///
    /// 표 셀 생성 등에서 최소한의 유효한 문단이 필요할 때 사용한다.
    pub fn new_empty() -> Self {
        Paragraph {
            char_count: 1, // 끝 마커(0x000D) 포함
            line_segs: vec![LineSeg {
                text_start: 0,
                line_height: 1000,
                text_height: 1000,
                baseline_distance: 850,
                line_spacing: 600,
                tag: 0x00060000, // HWP 기본 플래그
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// 문자의 UTF-16 코드 유닛 수를 반환한다.
    fn char_utf16_len(c: char) -> u32 {
        if (c as u32) > 0xFFFF {
            2
        } else {
            1
        }
    }

    /// char_offset 위치에 텍스트를 삽입한다.
    ///
    /// char_offset은 Rust 문자(char) 인덱스이다 (바이트 인덱스가 아님).
    /// 삽입 후 char_offsets, char_shapes, line_segs, char_count가 자동 갱신된다.
    ///
    /// char_offset이 text.chars().count()를 초과하면 인라인 컨트롤 뒤의
    /// 위치로 간주하여 올바른 UTF-16 위치에 삽입한다.
    pub(crate) fn insert_text_at(&mut self, char_offset: usize, new_text: &str) -> ParagraphEdit {
        let inverse = self.edit_checkpoint();
        if new_text.is_empty() {
            return inverse;
        }

        let utf16_width = |ch: char| {
            if ch == '\t' {
                8
            } else {
                Self::char_utf16_len(ch)
            }
        };
        let text_chars: Vec<char> = self.text.chars().collect();
        let text_len = text_chars.len();

        // char_offset > text_len: 인라인 컨트롤 뒤의 위치
        // navigable_text_len이 text_len보다 클 수 있음 (인라인 컨트롤 포함)
        // 이 경우 char_offset을 text_len으로 clamp하되, UTF-16 위치는
        // 마지막 문자 + 후행 컨트롤 갭을 포함한 값으로 계산
        let effective_char_offset = char_offset.min(text_len);
        let insertion_slot = self.field_insertion_slot(effective_char_offset);

        // 바이트 삽입 위치 계산
        let byte_offset: usize = text_chars[..effective_char_offset]
            .iter()
            .map(|c| c.len_utf8())
            .sum();

        // 삽입 지점의 UTF-16 위치 결정
        let utf16_insert_pos: u32 = if char_offset > text_len && !self.char_offsets.is_empty() {
            // 텍스트 끝 이후 (인라인 컨트롤 뒤): 마지막 문자의 UTF-16 위치 + 폭 + 후행 갭
            let last_idx = self.char_offsets.len() - 1;
            let last_char_end = self.char_offsets[last_idx] + utf16_width(text_chars[last_idx]);
            // 후행 컨트롤 수 = char_offset - text_len
            let trailing_ctrl_count = (char_offset - text_len) as u32;
            last_char_end + trailing_ctrl_count * 8
        } else if effective_char_offset < self.char_offsets.len() {
            let next_pos = self.char_offsets[effective_char_offset];
            let previous_end = effective_char_offset.checked_sub(1).map_or(0, |idx| {
                self.char_offsets[idx] + utf16_width(text_chars[idx])
            });
            let field_end_gap = self
                .hwpx_field_markers
                .iter()
                .enumerate()
                .filter(|(slot, marker)| {
                    marker.char_idx == effective_char_offset && *slot >= insertion_slot
                })
                .count();
            let field_end_gap = u32::try_from(field_end_gap)
                .unwrap_or(u32::MAX)
                .saturating_mul(8);
            // 끝 경계 삽입은 필드 안에 속하므로 끝 컨트롤 갭보다 앞에 둔다.
            if next_pos.saturating_sub(previous_end) >= field_end_gap {
                next_pos - field_end_gap
            } else {
                next_pos
            }
        } else if !self.char_offsets.is_empty() {
            let last_idx = self.char_offsets.len() - 1;
            self.char_offsets[last_idx] + utf16_width(text_chars[last_idx])
        } else {
            // 텍스트가 비어있을 때: 기존 컨트롤 뒤에 삽입 (각 컨트롤 = 8 code units)
            let begins = self
                .hwpx_field_markers
                .iter()
                .filter(|m| m.endpoint == FieldEndpoint::Begin)
                .count();
            let prefix = self
                .hwpx_field_markers
                .iter()
                .take(insertion_slot)
                .filter(|m| m.char_idx == effective_char_offset)
                .count();
            u32::try_from(self.controls.len().saturating_sub(begins) + prefix)
                .unwrap_or(u32::MAX)
                .saturating_mul(8)
        };
        let char_offset = effective_char_offset;

        // 새 텍스트의 UTF-16 총 길이
        let new_chars: Vec<char> = new_text.chars().collect();
        let utf16_delta: u32 = new_chars.iter().map(|c| utf16_width(*c)).sum();

        // 1. 텍스트 삽입
        self.text.insert_str(byte_offset, new_text);

        // 2. char_offsets 재구축
        // 삽입 지점 이후의 기존 오프셋을 시프트
        for offset in self.char_offsets[char_offset..].iter_mut() {
            *offset += utf16_delta;
        }
        // 새 문자들의 오프셋 삽입
        let mut new_offsets = Vec::with_capacity(new_chars.len());
        let mut pos = utf16_insert_pos;
        for c in &new_chars {
            new_offsets.push(pos);
            pos += utf16_width(*c);
        }
        // char_offset 위치에 새 오프셋 삽입
        let mut updated_offsets = Vec::with_capacity(self.char_offsets.len() + new_offsets.len());
        updated_offsets.extend_from_slice(&self.char_offsets[..char_offset]);
        updated_offsets.extend_from_slice(&new_offsets);
        updated_offsets.extend_from_slice(&self.char_offsets[char_offset..]);
        self.char_offsets = updated_offsets;

        // 3. char_shapes: 삽입 지점 이후의 start_pos를 시프트
        // 문단 시작(pos 0)에 삽입할 때 첫 번째 스타일(start_pos=0)은 유지
        for cs in &mut self.char_shapes {
            if cs.start_pos > utf16_insert_pos {
                cs.start_pos += utf16_delta;
            } else if cs.start_pos == utf16_insert_pos && cs.start_pos > 0 {
                cs.start_pos += utf16_delta;
            }
        }

        // 4. line_segs: 삽입 지점 이후의 text_start를 시프트
        for ls in &mut self.line_segs {
            if ls.text_start > utf16_insert_pos {
                ls.text_start += utf16_delta;
            } else if ls.text_start == utf16_insert_pos && ls.text_start > 0 {
                ls.text_start += utf16_delta;
            }
        }

        // 5. range_tags: 삽입 지점 이후의 start/end를 시프트
        for rt in &mut self.range_tags {
            if rt.start >= utf16_insert_pos {
                rt.start += utf16_delta;
            }
            if rt.end >= utf16_insert_pos {
                rt.end += utf16_delta;
            }
        }

        let inserted_len = new_chars.len();

        self.shift_hwpx_field_markers(char_offset..char_offset, inserted_len, insertion_slot);

        // 6. char_count 갱신
        self.char_count += utf16_delta;
        inverse
    }

    /// char_offset 위치에서 count개의 문자를 삭제한다.
    ///
    /// char_offset은 Rust 문자(char) 인덱스이다 (바이트 인덱스가 아님).
    /// 삭제 후 char_offsets, char_shapes, line_segs, char_count가 자동 갱신된다.
    /// 반환값: 실제 삭제된 문자 수.
    pub(crate) fn delete_text_at(&mut self, char_offset: usize, count: usize) -> ParagraphEdit {
        let mut inverse = self.edit_checkpoint();
        if count == 0 {
            return inverse;
        }

        let text_chars: Vec<char> = self.text.chars().collect();
        let text_len = text_chars.len();

        if char_offset >= text_len {
            return inverse;
        }

        // 실제 삭제할 문자 수 (범위 클램핑)
        let actual_count = count.min(text_len - char_offset);
        let del_end = char_offset + actual_count;

        // 바이트 범위 계산
        let byte_start: usize = text_chars[..char_offset].iter().map(|c| c.len_utf8()).sum();
        let byte_end: usize = text_chars[..del_end].iter().map(|c| c.len_utf8()).sum();

        let mut utf16_delta = 0;
        let deleted_spans: Vec<_> = self.char_offsets[char_offset..del_end]
            .iter()
            .zip(&text_chars[char_offset..del_end])
            .map(|(&start, &ch)| {
                let removed_before = utf16_delta;
                let width = if ch == '\t' {
                    8
                } else {
                    Self::char_utf16_len(ch)
                };
                utf16_delta += width;
                (start, removed_before, width)
            })
            .collect();
        // 텍스트만 지우므로 컨트롤 갭은 남기고 각 위치 앞에서 지운 UTF-16만 차감한다.
        let shift_position = |position: u32| {
            let count = deleted_spans.partition_point(|(start, _, _)| *start < position);
            if count == 0 {
                position
            } else {
                let (start, removed_before, width) = deleted_spans[count - 1];
                position - removed_before - (position - start).min(width)
            }
        };

        // 1. 텍스트 삭제
        self.text.drain(byte_start..byte_end);

        // 2. char_offsets: 삭제 범위 제거 + 이후 엔트리 시프트
        let mut updated_offsets =
            Vec::with_capacity(self.char_offsets.len().saturating_sub(actual_count));
        updated_offsets.extend_from_slice(&self.char_offsets[..char_offset]);
        for &offset in &self.char_offsets[del_end..] {
            updated_offsets.push(offset - utf16_delta);
        }
        self.char_offsets = updated_offsets;

        // 3. char_shapes: 삭제 범위 이후 → utf16_delta만큼 감소
        for cs in &mut self.char_shapes {
            cs.start_pos = shift_position(cs.start_pos);
        }

        // 4. line_segs: 삭제 범위 이후 → utf16_delta만큼 감소
        for ls in &mut self.line_segs {
            ls.text_start = shift_position(ls.text_start);
        }

        // 5. range_tags: 삭제 범위에 따라 축소/조정
        for rt in &mut self.range_tags {
            rt.start = shift_position(rt.start);
            rt.end = shift_position(rt.end);
        }

        self.shift_hwpx_field_markers(char_offset..del_end, 0, 0);

        // 6. char_count 갱신
        self.char_count -= utf16_delta;

        inverse.deleted_chars = actual_count;
        inverse
    }

    /// char_offset 위치에서 문단을 분할한다.
    ///
    /// 현재 문단은 char_offset 이전까지만 유지되고,
    /// char_offset 이후의 텍스트와 메타데이터로 새 문단을 생성하여 반환한다.
    pub(crate) fn split_at(&mut self, char_offset: usize) -> ParagraphSplit {
        let inverse = self.edit_checkpoint();
        let text_chars: Vec<char> = self.text.chars().collect();
        let text_len = text_chars.len();
        let split_pos = char_offset.min(text_len);

        // 분할 지점의 UTF-16 위치
        let utf16_split: u32 = if split_pos < self.char_offsets.len() {
            self.char_offsets[split_pos]
        } else if !self.char_offsets.is_empty() {
            let last = self.char_offsets.len() - 1;
            self.char_offsets[last] + Self::char_utf16_len(text_chars[last])
        } else {
            split_pos as u32
        };

        // === 새 문단 구성 ===

        // 1. 텍스트 분할
        let byte_offset: usize = text_chars[..split_pos].iter().map(|c| c.len_utf8()).sum();
        let new_text = self.text[byte_offset..].to_string();
        self.text.truncate(byte_offset);

        // 2. char_offsets 분할
        let new_char_offsets: Vec<u32> = self.char_offsets[split_pos..]
            .iter()
            .map(|&off| off - utf16_split)
            .collect();
        self.char_offsets.truncate(split_pos);

        // 3. char_shapes 분할
        let mut new_char_shapes: Vec<CharShapeRef> = Vec::new();
        // 분할 지점에서의 활성 스타일 찾기
        let mut active_style_id: u32 = self
            .char_shapes
            .first()
            .map(|cs| cs.char_shape_id)
            .unwrap_or(0);
        for cs in &self.char_shapes {
            if cs.start_pos <= utf16_split {
                active_style_id = cs.char_shape_id;
            }
        }

        // 분할 지점 이후의 char_shapes를 새 문단으로 이동 (위치 조정)
        let mut has_zero_pos = false;
        for cs in &self.char_shapes {
            if cs.start_pos >= utf16_split {
                let new_pos = cs.start_pos - utf16_split;
                if new_pos == 0 {
                    has_zero_pos = true;
                }
                new_char_shapes.push(CharShapeRef {
                    start_pos: new_pos,
                    char_shape_id: cs.char_shape_id,
                });
            }
        }
        // 새 문단의 시작(pos 0)에 스타일이 없으면 활성 스타일 추가
        if !has_zero_pos {
            new_char_shapes.insert(
                0,
                CharShapeRef {
                    start_pos: 0,
                    char_shape_id: active_style_id,
                },
            );
        }

        // 원래 문단의 char_shapes: 분할 지점 이후 제거
        self.char_shapes.retain(|cs| cs.start_pos < utf16_split);
        if self.char_shapes.is_empty() {
            self.char_shapes.push(CharShapeRef {
                start_pos: 0,
                char_shape_id: active_style_id,
            });
        }

        // 4. line_segs: 원본 치수를 보존하여 리플로우 시 올바른 줄간격 유지
        //    split_at 후 reflow_line_segs()가 첫 번째 LineSeg의 치수를 참조하므로,
        //    원본 HWP의 줄높이/텍스트높이 등을 유지해야 한다.
        let orig_line_seg = self.line_segs.first().cloned();
        let (lh, th, bd, ls, sw, tag) = match orig_line_seg {
            Some(ref o) if o.line_height > 0 => (
                o.line_height,
                o.text_height,
                o.baseline_distance,
                o.line_spacing,
                o.segment_width,
                o.tag,
            ),
            _ => (400, 400, 320, 0, 0, 0x00060000),
        };
        let new_line_segs = vec![LineSeg {
            text_start: 0,
            line_height: lh,
            text_height: th,
            baseline_distance: bd,
            line_spacing: ls,
            segment_width: sw,
            tag,
            ..Default::default()
        }];
        self.line_segs = vec![LineSeg {
            text_start: 0,
            line_height: lh,
            text_height: th,
            baseline_distance: bd,
            line_spacing: ls,
            segment_width: sw,
            tag,
            ..Default::default()
        }];

        // 5. range_tags 분할
        let mut new_range_tags: Vec<RangeTag> = Vec::new();
        let mut kept_range_tags: Vec<RangeTag> = Vec::new();
        for rt in &self.range_tags {
            if rt.start >= utf16_split {
                // 완전히 새 문단 쪽
                new_range_tags.push(RangeTag {
                    start: rt.start - utf16_split,
                    end: rt.end - utf16_split,
                    tag: rt.tag,
                });
            } else if rt.end <= utf16_split {
                // 완전히 원래 문단 쪽
                kept_range_tags.push(rt.clone());
            }
            // 경계에 걸치는 태그는 양쪽에서 제거 (단순화)
        }
        self.range_tags = kept_range_tags;

        let marker_slot = self
            .hwpx_field_markers
            .iter()
            .position(|m| {
                m.char_idx > split_pos
                    || (m.char_idx == split_pos && m.endpoint == FieldEndpoint::Begin)
            })
            .unwrap_or(self.hwpx_field_markers.len());
        let new_field_markers = self
            .hwpx_field_markers
            .drain(marker_slot..)
            .map(|mut marker| {
                marker.char_idx -= split_pos;
                marker
            })
            .collect();

        // 6. char_count 갱신
        //    원본 문단에 남은 controls는 각각 8 code unit을 차지하므로 반영 필요
        let new_text_char_count = new_text.chars().count() as u32;
        let ctrl_code_units: u32 = self.controls.len() as u32 * 8;
        self.char_count = split_pos as u32 + ctrl_code_units + 1; // +1 for paragraph end marker
        self.char_count = self.char_count.max(
            self.char_offsets
                .last()
                .zip(self.text.chars().last())
                .map_or(1, |(&offset, ch)| offset + Self::char_utf16_len(ch) + 1),
        );
        let new_char_count = (new_text_char_count + 1).max(
            new_char_offsets
                .last()
                .zip(new_text.chars().last())
                .map_or(1, |(&offset, ch)| offset + Self::char_utf16_len(ch) + 1),
        );

        // 7. has_para_text: 빈 문단(텍스트 없고 컨트롤 없음)이면 PARA_TEXT 불필요
        //    HWP 프로그램은 cc=1(빈 문단)에 PARA_TEXT가 있으면 파일 손상으로 판단
        if self.text.is_empty() && self.controls.is_empty() {
            self.has_para_text = false;
        }
        let new_has_para_text = !new_text.is_empty(); // 새 문단은 controls가 없으므로 텍스트 유무로 판단

        ParagraphSplit {
            paragraph: Paragraph {
                text: new_text,
                char_offsets: new_char_offsets,
                char_shapes: new_char_shapes,
                line_segs: new_line_segs,
                range_tags: new_range_tags,
                hwpx_field_markers: new_field_markers,
                char_count: new_char_count,
                para_shape_id: self.para_shape_id,
                style_id: self.style_id,
                column_type: ColumnBreakType::None,
                raw_break_type: 0,
                control_mask: 0,
                controls: Vec::new(),
                ctrl_data_records: Vec::new(),
                char_count_msb: false,
                raw_header_extra: self.raw_header_extra.clone(),
                has_para_text: new_has_para_text,
                tab_extended: Vec::new(),
                numbering_restart: None,
            },
            inverse,
        }
    }

    /// 다른 문단의 텍스트와 메타데이터를 현재 문단 끝에 결합한다.
    ///
    /// 병합 후 other 문단의 내용은 현재 문단에 포함된다.
    /// 반환값: 병합 지점의 char offset (원래 텍스트의 길이).
    pub(crate) fn merge_from(&mut self, other: &Paragraph) -> ParagraphMerge {
        let inverse = self.edit_checkpoint();
        let self_text_len = self.text.chars().count();
        let self_markers_before = self.hwpx_field_markers.len();
        let self_non_fields_before = self
            .controls
            .iter()
            .filter(|c| !matches!(c, Control::Field(_)))
            .count();
        let marker_order_offset = self
            .hwpx_field_markers
            .iter()
            .map(|m| m.order + 1)
            .max()
            .unwrap_or(0)
            .max(self.controls.len());
        self.hwpx_field_markers
            .extend(other.hwpx_field_markers.iter().cloned().map(|mut marker| {
                marker.char_idx += self_text_len;
                marker.order += marker_order_offset;
                if let FieldId::Binary(ci) = &mut marker.id {
                    *ci += self.controls.len();
                }
                marker
            }));
        let ctrl_offset = self.controls.len();
        self.controls.extend(other.controls.iter().cloned());
        self.ctrl_data_records.resize(ctrl_offset, None);
        self.ctrl_data_records.extend(
            (0..other.controls.len()).map(|ci| other.ctrl_data_records.get(ci).cloned().flatten()),
        );
        self.control_mask |= other.control_mask;
        self.has_para_text |= other.has_para_text;
        if other.text.is_empty() {
            return ParagraphMerge {
                offset: self_text_len,
                inverse,
            };
        }

        // 현재 문단 끝의 UTF-16 위치
        let utf16_end: u32 = if !self.char_offsets.is_empty() {
            let last = self.char_offsets.len() - 1;
            let text_chars: Vec<char> = self.text.chars().collect();
            self.char_offsets[last] + Self::char_utf16_len(text_chars[last])
        } else if self_non_fields_before > 0 {
            u32::try_from(self_non_fields_before + self_markers_before)
                .unwrap_or(0)
                .saturating_mul(8)
        } else {
            0
        };

        // 1. 텍스트 결합
        self.text.push_str(&other.text);

        // 2. char_offsets 결합 (other의 오프셋에 utf16_end 추가)
        for &off in &other.char_offsets {
            self.char_offsets.push(off + utf16_end);
        }

        // 3. char_shapes 결합 (other의 start_pos에 utf16_end 추가)
        for cs in &other.char_shapes {
            let new_pos = cs.start_pos + utf16_end;
            // 중복 위치에 같은 스타일이면 스킵
            if self
                .char_shapes
                .last()
                .map(|last| last.start_pos == new_pos && last.char_shape_id == cs.char_shape_id)
                .unwrap_or(false)
            {
                continue;
            }
            self.char_shapes.push(CharShapeRef {
                start_pos: new_pos,
                char_shape_id: cs.char_shape_id,
            });
        }

        // 4. line_segs: 원본 치수를 보존하여 리플로우 시 올바른 줄간격 유지
        let orig_line_seg = self.line_segs.first().cloned();
        let (lh, th, bd, ls, sw, tag) = match orig_line_seg {
            Some(ref o) if o.line_height > 0 => (
                o.line_height,
                o.text_height,
                o.baseline_distance,
                o.line_spacing,
                o.segment_width,
                o.tag,
            ),
            _ => (400, 400, 320, 0, 0, 0x00060000),
        };
        self.line_segs = vec![LineSeg {
            text_start: 0,
            line_height: lh,
            text_height: th,
            baseline_distance: bd,
            line_spacing: ls,
            segment_width: sw,
            tag,
            ..Default::default()
        }];

        // 5. range_tags 결합 (other의 start/end에 utf16_end 추가)
        for rt in &other.range_tags {
            self.range_tags.push(RangeTag {
                start: rt.start + utf16_end,
                end: rt.end + utf16_end,
                tag: rt.tag,
            });
        }

        // 6. char_count 갱신 (-1 because merge removes one paragraph end)
        self.char_count = (self_text_len + other.text.chars().count()) as u32 + 1;
        self.char_count = self.char_count.max(
            self.char_offsets
                .last()
                .zip(self.text.chars().last())
                .map_or(1, |(&offset, ch)| offset + Self::char_utf16_len(ch) + 1),
        );

        ParagraphMerge {
            offset: self_text_len,
            inverse,
        }
    }

    /// 주어진 문자 오프셋(char index)에 해당하는 CharShapeRef의 char_shape_id를 반환한다.
    ///
    /// char_shapes가 비어있으면 None을 반환한다.
    /// char_offset에 해당하는 UTF-16 위치를 찾아서 가장 적절한 CharShapeRef를 선택한다.
    pub fn char_shape_id_at(&self, char_offset: usize) -> Option<u32> {
        if self.char_shapes.is_empty() {
            return None;
        }

        // char_offset → UTF-16 위치 변환
        let utf16_pos = if char_offset < self.char_offsets.len() {
            self.char_offsets[char_offset]
        } else if !self.char_offsets.is_empty() {
            // 문단 끝 위치
            let last = *self.char_offsets.last().unwrap();
            let last_char = self.text.chars().nth(self.char_offsets.len() - 1);
            last + last_char
                .map(|c| if (c as u32) > 0xFFFF { 2 } else { 1 })
                .unwrap_or(1)
        } else {
            0
        };

        // utf16_pos 이하인 가장 큰 start_pos를 가진 CharShapeRef 찾기
        let mut result_id = self.char_shapes[0].char_shape_id;
        for csr in &self.char_shapes {
            if csr.start_pos <= utf16_pos {
                result_id = csr.char_shape_id;
            } else {
                break;
            }
        }
        Some(result_id)
    }

    /// [start_char_offset, end_char_offset) 범위에 new_char_shape_id를 적용한다.
    ///
    /// CharShapeRef 배열을 분할/교체하여 지정 범위만 새 ID로 변경한다.
    /// 범위 경계에서 기존 CharShapeRef가 부분적으로 겹치면 분할한다.
    /// 적용 후 연속 동일 ID는 병합한다.
    pub(crate) fn apply_char_shape_range(
        &mut self,
        start_char_offset: usize,
        end_char_offset: usize,
        new_char_shape_id: u32,
    ) {
        if start_char_offset >= end_char_offset || self.char_offsets.is_empty() {
            return;
        }
        if self.char_shapes.is_empty() {
            self.char_shapes.push(CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            });
        }

        // char offset → UTF-16 위치 변환
        let utf16_start = if start_char_offset < self.char_offsets.len() {
            self.char_offsets[start_char_offset]
        } else {
            return;
        };
        let utf16_end = if end_char_offset < self.char_offsets.len() {
            self.char_offsets[end_char_offset]
        } else if !self.char_offsets.is_empty() {
            let last = *self.char_offsets.last().unwrap();
            let last_char = self.text.chars().nth(self.char_offsets.len() - 1);
            last + last_char
                .map(|c| if (c as u32) > 0xFFFF { 2 } else { 1 })
                .unwrap_or(1)
        } else {
            return;
        };

        if utf16_start >= utf16_end {
            return;
        }

        // 문단 내 텍스트가 차지하는 UTF-16 영역의 끝 위치 (복원 범위 제한용)
        // 컨트롤이 있으면 char_offsets가 0이 아닌 위치에서 시작하므로
        // 단순 텍스트 길이가 아닌 마지막 문자의 UTF-16 끝 위치를 사용해야 한다.
        let text_utf16_end: u32 = if !self.char_offsets.is_empty() {
            let last_idx = self.char_offsets.len() - 1;
            let last_char = self.text.chars().nth(last_idx);
            self.char_offsets[last_idx]
                + last_char
                    .map(|c| if (c as u32) > 0xFFFF { 2 } else { 1 })
                    .unwrap_or(1)
        } else {
            self.text
                .chars()
                .map(|c| if (c as u32) > 0xFFFF { 2u32 } else { 1u32 })
                .sum()
        };

        // 새 CharShapeRef 배열을 구축
        let mut new_refs: Vec<CharShapeRef> = Vec::new();

        for (i, csr) in self.char_shapes.iter().enumerate() {
            let seg_start = csr.start_pos;
            // 다음 CharShapeRef의 start_pos 또는 문단 끝
            let seg_end = if i + 1 < self.char_shapes.len() {
                self.char_shapes[i + 1].start_pos
            } else {
                u32::MAX
            };

            if seg_end <= utf16_start || seg_start >= utf16_end {
                // 범위와 겹치지 않음 — 그대로 유지
                new_refs.push(csr.clone());
            } else {
                // 겹침 발생
                // 범위 앞부분 (seg_start < utf16_start)
                if seg_start < utf16_start {
                    new_refs.push(CharShapeRef {
                        start_pos: seg_start,
                        char_shape_id: csr.char_shape_id,
                    });
                }

                // 새 ID 삽입 (범위 시작점)
                let insert_start = utf16_start.max(seg_start);
                // 이미 같은 위치에 new_char_shape_id가 있는지 확인
                let already_inserted = new_refs
                    .last()
                    .map(|r| r.start_pos == insert_start && r.char_shape_id == new_char_shape_id)
                    .unwrap_or(false);
                if !already_inserted {
                    // 이전 ref가 같은 start_pos인데 다른 ID이면 교체
                    if let Some(last) = new_refs.last_mut() {
                        if last.start_pos == insert_start {
                            last.char_shape_id = new_char_shape_id;
                        } else {
                            new_refs.push(CharShapeRef {
                                start_pos: insert_start,
                                char_shape_id: new_char_shape_id,
                            });
                        }
                    } else {
                        new_refs.push(CharShapeRef {
                            start_pos: insert_start,
                            char_shape_id: new_char_shape_id,
                        });
                    }
                }

                // 범위 뒷부분 복원 (utf16_end < seg_end, 텍스트 범위 내일 때만)
                if utf16_end < seg_end && utf16_end < text_utf16_end {
                    new_refs.push(CharShapeRef {
                        start_pos: utf16_end,
                        char_shape_id: csr.char_shape_id,
                    });
                }
            }
        }

        // 연속 동일 ID 병합
        let mut merged: Vec<CharShapeRef> = Vec::new();
        for r in new_refs {
            if let Some(last) = merged.last() {
                if last.char_shape_id == r.char_shape_id {
                    continue; // 동일 ID 연속 → 뒤의 것 제거
                }
            }
            merged.push(r);
        }

        self.char_shapes = merged;
    }
}

#[cfg(test)]
mod tests;
