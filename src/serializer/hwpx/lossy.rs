//! HWPX 저장 시 emit되지 않아 손실되는 컨트롤/콘텐츠의 분류(단일 진실원).
//!
//! HWPX serializer 는 컨트롤 중 일부만, 또 emit 하는 컨트롤도 본문 geometry/텍스트만 쓰고
//! 부가 콘텐츠(글상자 텍스트·객체 캡션·구역 설정 등)는 버린다(silent loss). 파서는 전 컨트롤을
//! 모델에 보존하므로 **저장 직전**에 무엇이 손실될지 감지할 수 있다. 본 모듈은 그 분류를 한 곳에
//! 모아 두 drop site(본문 `render_controls_xml`, 표 셀 `write_cell_control_run`)와 섹션 진입
//! (`write_section`)이 동일 분류를 쓴다.
//!
//! ## surface 별 emit 집합 차이(중요)
//! - 본문(Body): `write_control_xml` 이 6종(Table/Picture/Shape/PageHide/PageNumberPos/Form) emit.
//! - 표 셀(Cell): `write_cell_control` 이 4종(Table/Picture/Shape/Form)만 emit — **PageHide/
//!   PageNumberPos 는 셀에서 drop** 된다. 따라서 분류는 `LossySurface` 에 따라 달라진다.
//!
//! ## 부분 emit 손실(geometry 는 쓰지만 콘텐츠는 버림)
//! - 도형(Line/Rectangle 등)·그림·표·묶음의 **캡션**(`caption`)은 어떤 writer 도 emit 하지 않는다.
//! - 도형 **글상자 텍스트**(`drawing.text_box.paragraphs`)도 emit 되지 않는다.
//! - `SectionDef` 는 `page_def`(용지/여백)만 `render_page_pr` 로 재-emit 되고, 바탕쪽/쪽 테두리/
//!   감추기 플래그/시작 번호/개요 번호 등은 버려진다(섹션 단위 `section_def_has_unemitted_content`).
//!
//! ## drift 가드
//! `classify_hwpx_lossy`·`shape_has_unemittable` 는 `_` 와일드카드 없는 exhaustive match 다.
//! `Control`/`ShapeObject` enum 변경 시 컴파일 에러로 "새 컨트롤이 분류 없이 silent drop"을 막는다.
//!
//! ## fail-safe 원칙
//! 재-emit 여부가 불확실하면 None 이 아니라 Some(lossy)(over-warn > silent-loss). 단 false-positive
//! (실제 저장되는 콘텐츠를 오경고)는 경고 피로를 부르므로, 콘텐츠가 실재할 때만(caption.is_some,
//! text_box.paragraphs 비어있지 않음 등) 경고한다.

use crate::model::control::Control;
use crate::model::document::SectionDef;
use crate::model::shape::{DrawingObjAttr, ShapeObject};

/// 손실이 일어나는 직렬화 표면. 셀은 본문보다 emit 집합이 좁다(PageHide/PageNumberPos drop).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossySurface {
    /// 본문 문단(`write_control_xml`, 6종 emit).
    Body,
    /// 표 셀 문단(`write_cell_control`, 4종 emit — PageHide/PageNumberPos drop).
    Cell,
}

/// HWPX 저장 시 손실되는 컨트롤/콘텐츠의 사용자-노출 종류.
///
/// 머신-안정 문자열(`as_str`)로 직렬화되며, 한국어 라벨 매핑은 TS(UI) 레이어가 담당한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossyKind {
    /// 누름틀(필드 컨트롤 — ClickHere/MailMerge/CrossRef 등)
    Field,
    /// 책갈피
    Bookmark,
    /// 수식
    Equation,
    /// 자동 번호
    AutoNumber,
    /// 새 번호 지정
    NewNumber,
    /// 각주
    Footnote,
    /// 미주
    Endnote,
    /// 하이퍼링크(컨트롤 형)
    Hyperlink,
    /// 덧말
    Ruby,
    /// 글자 겹침
    CharOverlap,
    /// 숨은 설명
    HiddenComment,
    /// 다단(단 정의)
    ColumnDef,
    /// 머리말
    Header,
    /// 꼬리말
    Footer,
    /// 감추기(쪽 숨김) — 표 셀에서만 손실(본문은 emit)
    PageHide,
    /// 쪽 번호 위치 — 표 셀에서만 손실(본문은 emit)
    PageNumberPos,
    /// 그리기 개체(타원/호/다각형/곡선 등 미지원 서브타입 — 통째로 손실)
    Shape,
    /// 글상자 텍스트(도형 안 텍스트 — geometry 는 저장되나 텍스트는 손실)
    TextBox,
    /// 객체 캡션(표/그림/도형의 캡션 — 어떤 writer 도 emit 안 함)
    Caption,
    /// 구역 설정(바탕쪽·쪽 테두리·감추기·시작 번호 등 page_def 외 SectionDef 손실)
    SectionSettings,
    /// 알 수 없는 컨트롤
    Unknown,
}

impl LossyKind {
    /// 머신-안정 식별 문자열(UI 라벨 매핑 키). enum variant 이름과 1:1.
    pub fn as_str(&self) -> &'static str {
        match self {
            LossyKind::Field => "Field",
            LossyKind::Bookmark => "Bookmark",
            LossyKind::Equation => "Equation",
            LossyKind::AutoNumber => "AutoNumber",
            LossyKind::NewNumber => "NewNumber",
            LossyKind::Footnote => "Footnote",
            LossyKind::Endnote => "Endnote",
            LossyKind::Hyperlink => "Hyperlink",
            LossyKind::Ruby => "Ruby",
            LossyKind::CharOverlap => "CharOverlap",
            LossyKind::HiddenComment => "HiddenComment",
            LossyKind::ColumnDef => "ColumnDef",
            LossyKind::Header => "Header",
            LossyKind::Footer => "Footer",
            LossyKind::PageHide => "PageHide",
            LossyKind::PageNumberPos => "PageNumberPos",
            LossyKind::Shape => "Shape",
            LossyKind::TextBox => "TextBox",
            LossyKind::Caption => "Caption",
            LossyKind::SectionSettings => "SectionSettings",
            LossyKind::Unknown => "Unknown",
        }
    }
}

/// 한 컨트롤이 저장 시 손실되는 위치 + 종류.
///
/// `section_index`/`para_index` 는 향후 손실 위치 네비게이션(MAY)용 best-effort 좌표다.
/// 표 셀처럼 중첩된 위치의 `para_index` 는 셀-로컬 문단 인덱스(근사)일 수 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LossyDrop {
    pub kind: LossyKind,
    pub section_index: usize,
    pub para_index: usize,
}

/// 한 컨트롤이 주어진 표면(`surface`)에서 저장 시 손실되면 `Some(kind)`, 안전하면 `None`.
///
/// ⚠️ exhaustive match(`_` 금지) — `Control` variant 변경 시 컴파일 에러로 drift 차단.
///
/// 한 컨트롤이 여러 손실(예: 미지원 도형 + 캡션)을 가질 수 있으나, 분류는 가장 심한 1종만
/// 반환한다(전체 손실 > 콘텐츠 일부 손실). UI 는 종류별 집계로 표시한다.
pub fn classify_hwpx_lossy(ctrl: &Control, surface: LossySurface) -> Option<LossyKind> {
    match ctrl {
        // ── 본문/셀 공통 emit(geometry/구조). 단 캡션은 어떤 writer 도 emit 안 함 ──
        Control::Table(t) => {
            if t.caption.is_some() {
                Some(LossyKind::Caption)
            } else {
                None
            }
        }
        Control::Picture(p) => {
            if p.caption.is_some() {
                Some(LossyKind::Caption)
            } else {
                None
            }
        }
        Control::Form(_) => None,
        // ── Shape: 미지원 서브타입(통째) > 글상자 텍스트 > 캡션 ──
        Control::Shape(shape) => classify_shape(shape),
        // ── PageHide/PageNumberPos: 본문은 emit, 셀은 drop ──
        Control::PageHide(_) => match surface {
            LossySurface::Body => None,
            LossySurface::Cell => Some(LossyKind::PageHide),
        },
        Control::PageNumberPos(_) => match surface {
            LossySurface::Body => None,
            LossySurface::Cell => Some(LossyKind::PageNumberPos),
        },
        // ── SectionDef: page_def 는 render_page_pr 로 재-emit. page_def 외 손실은 섹션 단위
        //    (section_def_has_unemitted_content)로 따로 감지하므로 여기선 None(매-문단 중복 방지) ──
        Control::SectionDef(_) => None,
        // ── 재-emit 경로 없음 = 실손실 ──
        Control::ColumnDef(_) => Some(LossyKind::ColumnDef),
        Control::Header(_) => Some(LossyKind::Header),
        Control::Footer(_) => Some(LossyKind::Footer),
        Control::Footnote(_) => Some(LossyKind::Footnote),
        Control::Endnote(_) => Some(LossyKind::Endnote),
        Control::AutoNumber(_) => Some(LossyKind::AutoNumber),
        Control::NewNumber(_) => Some(LossyKind::NewNumber),
        Control::Bookmark(_) => Some(LossyKind::Bookmark),
        Control::Hyperlink(_) => Some(LossyKind::Hyperlink),
        Control::Ruby(_) => Some(LossyKind::Ruby),
        Control::CharOverlap(_) => Some(LossyKind::CharOverlap),
        Control::HiddenComment(_) => Some(LossyKind::HiddenComment),
        Control::Equation(_) => Some(LossyKind::Equation),
        Control::Field(_) => Some(LossyKind::Field),
        Control::Unknown(_) => Some(LossyKind::Unknown),
    }
}

/// 도형 컨트롤의 손실 분류. 미지원 서브타입(전체 손실) > 글상자 텍스트 > 캡션 순으로 심각도.
fn classify_shape(shape: &ShapeObject) -> Option<LossyKind> {
    if shape_has_unemittable(shape) {
        return Some(LossyKind::Shape);
    }
    if shape_has_text_box(shape) {
        return Some(LossyKind::TextBox);
    }
    if shape_has_caption(shape) {
        return Some(LossyKind::Caption);
    }
    None
}

/// 그리기 개체(또는 묶음 내 자식)에 serializer 가 통째로 버리는 미지원 서브타입이 있으면 true.
///
/// serializer(`write_shape_xml`/`write_cell_shape`) emit: Line/Rectangle/Picture/Group(자식 재귀).
/// drop: Ellipse/Arc/Polygon/Curve.
///
/// ⚠️ exhaustive match(`_` 금지) — `ShapeObject` variant 변경 시 컴파일 에러로 drift 차단.
fn shape_has_unemittable(shape: &ShapeObject) -> bool {
    match shape {
        ShapeObject::Line(_) | ShapeObject::Rectangle(_) | ShapeObject::Picture(_) => false,
        ShapeObject::Group(group) => group.children.iter().any(shape_has_unemittable),
        ShapeObject::Ellipse(_)
        | ShapeObject::Arc(_)
        | ShapeObject::Polygon(_)
        | ShapeObject::Curve(_) => true,
    }
}

/// 도형(또는 묶음 내 자식)에 emit 되지 않는 글상자 텍스트가 있으면 true.
/// serializer 는 도형 geometry(sz/pos/outMargin)만 쓰고 `drawing.text_box` 는 버린다.
fn shape_has_text_box(shape: &ShapeObject) -> bool {
    match shape {
        ShapeObject::Line(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Rectangle(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Ellipse(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Arc(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Polygon(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Curve(s) => drawing_has_text_box(&s.drawing),
        ShapeObject::Picture(_) => false,
        ShapeObject::Group(group) => group.children.iter().any(shape_has_text_box),
    }
}

/// 도형(또는 묶음 내 자식)에 emit 되지 않는 캡션이 있으면 true.
/// 어떤 도형/그림/표 writer 도 캡션을 emit 하지 않는다.
fn shape_has_caption(shape: &ShapeObject) -> bool {
    match shape {
        ShapeObject::Line(s) => s.drawing.caption.is_some(),
        ShapeObject::Rectangle(s) => s.drawing.caption.is_some(),
        ShapeObject::Ellipse(s) => s.drawing.caption.is_some(),
        ShapeObject::Arc(s) => s.drawing.caption.is_some(),
        ShapeObject::Polygon(s) => s.drawing.caption.is_some(),
        ShapeObject::Curve(s) => s.drawing.caption.is_some(),
        ShapeObject::Picture(p) => p.caption.is_some(),
        ShapeObject::Group(group) => {
            group.caption.is_some() || group.children.iter().any(shape_has_caption)
        }
    }
}

fn drawing_has_text_box(d: &DrawingObjAttr) -> bool {
    d.text_box
        .as_ref()
        .map_or(false, |tb| !tb.paragraphs.is_empty())
}

/// `SectionDef` 가 `page_def`(용지/여백) 외에 emit 되지 않는 의미 있는 콘텐츠를 가지면 true.
///
/// serializer 는 `render_page_pr` 로 page_def 만 재-emit 하고, 나머지 secPr 은 정적 템플릿
/// (`empty_section0.xml`)의 하드코딩 값으로 나간다. 따라서 SectionDef 의 page_def 외 필드 중
/// 템플릿 기본값과 다른 것은 저장 시 리셋되어 손실된다.
///
/// false-positive(경고 피로)를 피하려고, 0/HORIZONTAL/8000 등 **템플릿 기본값과 명확히 다른**
/// 값일 때만 손실로 본다(0=미지정 모호성이 있는 default_tab_spacing 은 0 과 템플릿 8000 둘 다 제외).
/// 템플릿 기본값(empty_section0.xml): textDirection="HORIZONTAL"(0), tabStop="8000",
/// startNum page/pic/tbl/equation="0", pageStartsOn="BOTH"(0).
///
/// 1차 page_border_fill·각주/미주 모양은 구조체 기본값 비교 모호성으로 보수적으로 제외(알려진 잔여).
pub fn section_def_has_unemitted_content(sd: &SectionDef) -> bool {
    !sd.master_pages.is_empty()
        || !sd.extra_page_border_fills.is_empty()
        || sd.hide_header
        || sd.hide_footer
        || sd.hide_master_page
        || sd.hide_border
        || sd.hide_fill
        || sd.hide_empty_line
        || sd.page_num != 0
        || sd.page_num_type != 0
        || sd.picture_num != 0
        || sd.table_num != 0
        || sd.equation_num != 0
        || sd.text_direction != 0
        || sd.outline_numbering_id != 0
        // 탭 간격: 0(미지정)도 템플릿 8000 도 아닌 실제 커스텀 값일 때만.
        || (sd.default_tab_spacing != 0 && sd.default_tab_spacing != 8000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::control::{Bookmark, Equation, Field, PageHide, PageNumberPos};
    use crate::model::image::Picture;
    use crate::model::shape::{
        Caption, EllipseShape, GroupShape, LineShape, RectangleShape, ShapeObject, TextBox,
    };
    use crate::model::table::Table;

    const BODY: LossySurface = LossySurface::Body;
    const CELL: LossySurface = LossySurface::Cell;

    #[test]
    fn content_controls_classified_lossy() {
        assert_eq!(
            classify_hwpx_lossy(&Control::Field(Field::default()), BODY),
            Some(LossyKind::Field)
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Bookmark(Bookmark::default()), BODY),
            Some(LossyKind::Bookmark)
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Equation(Box::new(Equation::default())), BODY),
            Some(LossyKind::Equation)
        );
    }

    #[test]
    fn plain_emitted_controls_classified_none() {
        // 캡션/글상자 없는 표·그림·직선·사각형은 손실 없음(false-positive 0).
        assert_eq!(
            classify_hwpx_lossy(&Control::Table(Box::new(Table::default())), BODY),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Picture(Box::new(Picture::default())), BODY),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Shape(Box::new(ShapeObject::Line(LineShape::default()))), BODY),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(
                &Control::Shape(Box::new(ShapeObject::Rectangle(RectangleShape::default()))),
                BODY
            ),
            None
        );
    }

    #[test]
    fn unsupported_shape_subtype_classified_lossy() {
        assert_eq!(
            classify_hwpx_lossy(
                &Control::Shape(Box::new(ShapeObject::Ellipse(EllipseShape::default()))),
                BODY
            ),
            Some(LossyKind::Shape)
        );
    }

    #[test]
    fn pagehide_pagenumberpos_lossy_only_in_cell() {
        // 본문은 emit → None, 셀은 drop → Some.
        assert_eq!(
            classify_hwpx_lossy(&Control::PageHide(PageHide::default()), BODY),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::PageHide(PageHide::default()), CELL),
            Some(LossyKind::PageHide)
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::PageNumberPos(PageNumberPos::default()), BODY),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::PageNumberPos(PageNumberPos::default()), CELL),
            Some(LossyKind::PageNumberPos)
        );
    }

    #[test]
    fn caption_on_table_or_picture_is_lossy() {
        let mut table = Table::default();
        table.caption = Some(Caption::default());
        assert_eq!(
            classify_hwpx_lossy(&Control::Table(Box::new(table)), BODY),
            Some(LossyKind::Caption)
        );

        let mut pic = Picture::default();
        pic.caption = Some(Caption::default());
        assert_eq!(
            classify_hwpx_lossy(&Control::Picture(Box::new(pic)), BODY),
            Some(LossyKind::Caption)
        );
    }

    #[test]
    fn rectangle_with_text_box_is_lossy() {
        // geometry 는 저장되지만 글상자 텍스트는 손실 → TextBox 경고.
        let mut rect = RectangleShape::default();
        rect.drawing.text_box = Some(TextBox {
            paragraphs: vec![crate::model::paragraph::Paragraph::default()],
            ..Default::default()
        });
        assert_eq!(
            classify_hwpx_lossy(&Control::Shape(Box::new(ShapeObject::Rectangle(rect))), BODY),
            Some(LossyKind::TextBox)
        );
    }

    #[test]
    fn group_with_unemittable_child_is_lossy() {
        let group = GroupShape {
            children: vec![
                ShapeObject::Line(LineShape::default()),
                ShapeObject::Ellipse(EllipseShape::default()),
            ],
            ..Default::default()
        };
        assert!(shape_has_unemittable(&ShapeObject::Group(group)));

        let safe_group = GroupShape {
            children: vec![
                ShapeObject::Line(LineShape::default()),
                ShapeObject::Rectangle(RectangleShape::default()),
            ],
            ..Default::default()
        };
        assert!(!shape_has_unemittable(&ShapeObject::Group(safe_group)));
    }

    #[test]
    fn section_def_unemitted_content_detected() {
        // 기본 SectionDef(page_def만 의미) → 손실 없음.
        let mut sd = SectionDef::default();
        assert!(!section_def_has_unemitted_content(&sd));
        // 바탕쪽이 있으면 손실.
        sd.master_pages.push(Default::default());
        assert!(section_def_has_unemitted_content(&sd));
        // 감추기 플래그도 손실 신호.
        let mut sd2 = SectionDef::default();
        sd2.hide_header = true;
        assert!(section_def_has_unemitted_content(&sd2));
        // 표/그림/수식 시작 번호(템플릿은 0 하드코딩)도 손실 신호.
        let mut sd3 = SectionDef::default();
        sd3.table_num = 5;
        assert!(section_def_has_unemitted_content(&sd3));
        // 세로쓰기(템플릿은 HORIZONTAL)도 손실 신호.
        let mut sd4 = SectionDef::default();
        sd4.text_direction = 1;
        assert!(section_def_has_unemitted_content(&sd4));
        // 템플릿 기본 탭(8000)은 손실 아님(false-positive 회피).
        let mut sd5 = SectionDef::default();
        sd5.default_tab_spacing = 8000;
        assert!(!section_def_has_unemitted_content(&sd5));
    }

    #[test]
    fn as_str_matches_variant_name() {
        assert_eq!(LossyKind::Field.as_str(), "Field");
        assert_eq!(LossyKind::Caption.as_str(), "Caption");
        assert_eq!(LossyKind::TextBox.as_str(), "TextBox");
        assert_eq!(LossyKind::SectionSettings.as_str(), "SectionSettings");
        assert_eq!(LossyKind::PageHide.as_str(), "PageHide");
    }
}
