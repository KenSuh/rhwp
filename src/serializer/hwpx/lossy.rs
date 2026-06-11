//! HWPX 저장 시 emit되지 않아 손실되는 컨트롤의 분류(단일 진실원).
//!
//! HWPX serializer(`section.rs`/`table.rs`)는 컨트롤 중 6종(Table/Picture/Shape/
//! PageHide/PageNumberPos/Form)만 emit하고 나머지는 소리 없이 버린다(silent loss).
//! 파서는 전 컨트롤을 모델에 보존하므로, **저장 직전**에 무엇이 손실될지 100% 감지할 수 있다.
//! 본 모듈은 그 분류를 `classify_hwpx_lossy` 한 곳에 모아 두고, 두 drop site
//! (본문 문단 `render_controls_xml`, 표 셀 `write_cell_control_run`)가 동일 분류를 쓴다.
//!
//! ## drift 가드
//! `classify_hwpx_lossy` 와 `shape_has_unemittable` 는 **`_` 와일드카드 없는 exhaustive
//! match** 다. `Control`/`ShapeObject` enum 에 variant 가 추가/제거되면 컴파일 에러가 나서
//! "새 컨트롤이 분류 없이 silent drop 되는" drift 를 막는다.
//!
//! ## fail-safe 원칙
//! 어떤 컨트롤이 다른 경로로 재-emit 되는지 **불확실하면 None 이 아니라 Some(lossy)** 로
//! 둔다(over-warn > silent-loss). 현재 None 으로 두는 컨트롤은 재-emit 이 코드로 입증된
//! 것뿐이다(emit-6 + SectionDef 의 page_def — 아래 주석 참조).

use crate::model::control::Control;
use crate::model::shape::ShapeObject;

/// HWPX 저장 시 손실되는 컨트롤의 사용자-노출 종류.
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
    /// 그리기 개체(타원/호/다각형/곡선 등 미지원 서브타입)
    Shape,
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
            LossyKind::Shape => "Shape",
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

/// HWPX serializer 가 emit 하지 않아 손실될 컨트롤이면 `Some(kind)`, 안전(무손실)하면 `None`.
///
/// ⚠️ exhaustive match(`_` 금지) — `Control` variant 변경 시 컴파일 에러로 drift 차단.
///
/// ## None(무손실) 근거 — 코드로 입증된 것만
/// - **emit-6 중 5종**(Table/Picture/PageHide/PageNumberPos/Form): `write_control_xml`
///   / `write_cell_control` 이 항상 직렬화(`Ok(true)`).
/// - **Shape**: 컨트롤 레벨은 emit 되나 서브타입(타원/호/다각형/곡선)은 drop 되므로
///   서브타입을 재귀 검사해 미지원이 하나라도 있으면 `Some(Shape)`.
/// - **SectionDef**: 용지 크기/여백(`page_def`)은 `section.rs::render_page_pr` 가 재-emit
///   하므로(저장→재로드 시 레이아웃 보존) 레이아웃 손실 없음. 잔여 secPr 구조 정규화는
///   별도 추적되는 한계이며 '내용 손실'이 아니다. 또한 SectionDef 는 **모든 문서/모든 섹션에
///   존재**하므로 Some 으로 두면 매 저장마다 경고가 떠 경고 피로(crying wolf)로 실제 위험
///   (누름틀/수식 손실) 경고가 무시된다 → 의도적으로 None.
pub fn classify_hwpx_lossy(ctrl: &Control) -> Option<LossyKind> {
    match ctrl {
        // ── 무손실 확정(항상 직렬화) ──
        Control::Table(_)
        | Control::Picture(_)
        | Control::PageHide(_)
        | Control::PageNumberPos(_)
        | Control::Form(_) => None,
        // ── Shape: 서브타입 검사(미지원 서브타입 = 손실) ──
        Control::Shape(shape) => {
            if shape_has_unemittable(shape) {
                Some(LossyKind::Shape)
            } else {
                None
            }
        }
        // ── SectionDef: page_def 재-emit 입증됨(위 주석) → None ──
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

/// 그리기 개체(또는 묶음 내 자식)에 serializer 가 emit 하지 못하는 서브타입이 있으면 true.
///
/// serializer(`write_shape_xml`/`write_cell_shape`)가 emit 하는 것: Line/Rectangle/Picture/
/// Group(자식 재귀). drop: Ellipse/Arc/Polygon/Curve. Group 은 컨테이너 자체는 emit 되나
/// 자식 중 미지원이 있으면 그 자식이 소리 없이 사라지므로, 자식까지 재귀 검사한다.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::control::{Bookmark, Equation, Field};
    use crate::model::image::Picture;
    use crate::model::shape::{EllipseShape, GroupShape, LineShape, RectangleShape, ShapeObject};
    use crate::model::table::Table;

    #[test]
    fn content_controls_classified_lossy() {
        assert_eq!(
            classify_hwpx_lossy(&Control::Field(Field::default())),
            Some(LossyKind::Field)
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Bookmark(Bookmark::default())),
            Some(LossyKind::Bookmark)
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Equation(Box::new(Equation::default()))),
            Some(LossyKind::Equation)
        );
    }

    #[test]
    fn emit_six_classified_none() {
        // 실제 직렬화되는 콘텐츠를 손실로 오경고하지 않는다(false-positive 0).
        assert_eq!(
            classify_hwpx_lossy(&Control::Table(Box::new(Table::default()))),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Picture(Box::new(Picture::default()))),
            None
        );
        // Shape(직선) = emit → None
        assert_eq!(
            classify_hwpx_lossy(&Control::Shape(Box::new(ShapeObject::Line(
                LineShape::default()
            )))),
            None
        );
        assert_eq!(
            classify_hwpx_lossy(&Control::Shape(Box::new(ShapeObject::Rectangle(
                RectangleShape::default()
            )))),
            None
        );
    }

    #[test]
    fn unsupported_shape_subtype_classified_lossy() {
        // 타원은 serializer 가 drop → 경고해야 함.
        assert_eq!(
            classify_hwpx_lossy(&Control::Shape(Box::new(ShapeObject::Ellipse(
                EllipseShape::default()
            )))),
            Some(LossyKind::Shape)
        );
    }

    #[test]
    fn group_with_unemittable_child_is_lossy() {
        // 묶음 자체는 emit 되지만 미지원 자식(타원)이 있으면 그 자식이 손실 → 경고.
        let group = GroupShape {
            children: vec![
                ShapeObject::Line(LineShape::default()),
                ShapeObject::Ellipse(EllipseShape::default()),
            ],
            ..Default::default()
        };
        assert!(shape_has_unemittable(&ShapeObject::Group(group)));

        // 모두 지원 서브타입이면 무손실.
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
    fn as_str_matches_variant_name() {
        assert_eq!(LossyKind::Field.as_str(), "Field");
        assert_eq!(LossyKind::Equation.as_str(), "Equation");
        assert_eq!(LossyKind::Shape.as_str(), "Shape");
        assert_eq!(LossyKind::Unknown.as_str(), "Unknown");
    }
}
