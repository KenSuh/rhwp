//! 문단 텍스트·오프셋 접근과 필드 끝점 경계 규칙.

use super::{FieldEndpoint, Paragraph};
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
    pub fn with_text(text: String, char_offsets: Vec<u32>) -> Self {
        Self {
            text,
            char_offsets,
            ..Self::default()
        }
    }

    /// 텍스트 인덱스는 유지하고 컨트롤 갭에 따른 원본 위치만 갱신한다.
    pub fn set_char_offsets(&mut self, offsets: Vec<u32>) {
        self.char_offsets = offsets;
    }

    /// 텍스트 전체 교체. 내부 끝점은 제거하고 저장 시 명시적 거부를 유도한다.
    pub fn replace_text(&mut self, text: String) {
        let old_len = self.text.chars().count();
        self.shift_hwpx_field_markers(0..old_len, text.chars().count());
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
    }

    /// 문자 범위 교체. 삭제와 삽입의 경계 규칙은 보통 편집과 동일하다.
    pub fn replace_text_range(&mut self, range: Range<usize>, text: &str) {
        self.delete_text_at(range.start, range.len());
        self.insert_text_at(range.start, text);
    }

    // 삭제 [s,e): m <= s 유지, m >= e 왼쪽 이동, s < m < e 제거.
    // 삽입 p: begin == p 는 앞에 남고 end == p 는 뒤로 이동해 새 글자를 포함한다.
    pub(super) fn shift_hwpx_field_markers(&mut self, deleted: Range<usize>, inserted: usize) {
        self.hwpx_field_markers.retain_mut(|marker| {
            if deleted.start < marker.char_idx && marker.char_idx < deleted.end {
                return false;
            }
            if marker.char_idx >= deleted.end {
                marker.char_idx -= deleted.len();
            }
            let shifts = match marker.endpoint {
                FieldEndpoint::Begin => marker.char_idx > deleted.start,
                FieldEndpoint::End => marker.char_idx >= deleted.start,
            };
            if shifts {
                marker.char_idx += inserted;
            }
            true
        });
    }
}
