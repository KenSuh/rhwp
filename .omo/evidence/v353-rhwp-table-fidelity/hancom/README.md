# 한컴 확인용 산출물 (v353, 게이트 G12)

Parent 가 한컴오피스로 열어 표 배치를 눈으로 확인할 HWPX 다. 모두 `tests/hwpx_table_fidelity.rs` 의
`write_hancom_review_files`(기본 시험에서는 돌지 않는 `#[ignore]` 시험)로 만들었다.

```bash
# 수정 head (6개)
V353_HANCOM_OUT=.omo/evidence/v353-rhwp-table-fidelity/hancom V353_HANCOM_SET=head \
  cargo test --test hwpx_table_fidelity write_hancom_review_files -- --ignored --nocapture
# 비교용 옛 코드 (a34d88ff 사본에서, 2개 `*_base.hwpx`)
V353_HANCOM_OUT=<위 폴더 절대경로> V353_HANCOM_SET=base \
  cargo test --test hwpx_table_fidelity write_hancom_review_files -- --ignored --nocapture
```

경로는 에디터와 같은 `DocumentCore::from_bytes` → 편집 명령 → `export_hwpx_native` 다.

## 파일과 기대값

표시 높이 규칙(패킷 §3.2): 행 삽입 = 편집 전 표시 높이 + 새 행의 표시 기준 높이, 행 삭제 =
max(편집 전 − 삭제 행의 표시 기준 높이, 셀 합), 행 수가 그대로인 편집 = 셀 합이 넘을 때만 늘림.
표시 기준 행 높이는 저장 행 높이를 표시 높이에 비례 배분한 값이다.

| 파일 | 표본·표 | 편집 | 저장된 `hp:sz@height` | 비고 |
|---|---|---|---:|---|
| `T1_before_edit.hwpx` | T1 본문 표 #58 (2×2) | 없음 (열기 → 저장) | 14901 | 원본과 같음 |
| `T1_after_insert_row.hwpx` | T1 본문 표 #58 | 첫 행 아래 행 삽입 | 28253 | 14901 + 13352 |
| `T1_after_insert_row_base.hwpx` | T1 본문 표 #58 | 같은 편집, 옛 코드 | 5144 | 셀 합으로 납작해짐 |
| `T1_after_text_edit.hwpx` | T1 본문 표 #58 | 셀 1 에 20자 | 14901 | 셀 합 6375 < 표시 높이 → 유지 |
| `T6_before_edit.hwpx` | T6 본문 표 #0 (5×1) | 없음 (열기 → 저장) | 73450 | 원본과 같음 |
| `T6_after_insert_row.hwpx` | T6 본문 표 #0 | 첫 행 아래 행 삽입 | 93179 | 73450 + 19729 |
| `T6_after_insert_row_base.hwpx` | T6 본문 표 #0 | 같은 편집, 옛 코드 | 7396 | 셀 합으로 납작해짐 |
| `T6_after_delete_row.hwpx` | T6 본문 표 #0 | 둘째 행 삭제 | 53721 | 73450 − 19729 |

T1 표 #58 의 행 높이는 [2431, 282](셀 합 2713)이고 표시 기준 행 높이는 [13352, 1549]다. 텍스트 편집 뒤
행 높이는 [5025, 1350](셀 합 6375)이다. T6 표 #0 은 [1566, 1566, 1566, 566, 566](셀 합 5830)이고 첫 두 행의
표시 기준 높이는 각각 19729 다. 표의 `hp:sz@width` 는 여덟 파일 모두 원본과 같다(T1 #58 47624, T6 #0 47903).

## 한컴에서 볼 것

- `*_after_insert_row.hwpx` 와 `*_after_insert_row_base.hwpx` 를 나란히 연다. head 는 표가 원래 비율로 한 행만큼
  길어지고, base 는 표가 셀 높이 합으로 납작해져야 한다(종전 결함).
- `*_before_edit.hwpx` 는 원본과 표 모양이 같아야 한다(속성 보존 — 캡션·cellzone·셀 `lineWrap=SQUEEZE` 포함).

## SHA-256

```text
8b9fa2104ae1db77a423521df56df8b1110f76db18ccc9291ab452981365e1a1  T1_before_edit.hwpx
e97e419e0db26fd91f14c4a03638c02e83e52a4b730b9d472cc3f632ac85d8d5  T1_after_insert_row.hwpx
ac1debb66f3fddfa3d37c444b0bbb9862e734cfed94075b78fa98e1957059c7f  T1_after_insert_row_base.hwpx
d2341d1015deed4a41a378661288ae05259aece9e0d110c3dd0f88c05a6514e3  T1_after_text_edit.hwpx
ff0b23a07d66c1cf67abc1ed83d3a267fb7a80a3a69ad018b9c51548e7b65248  T6_before_edit.hwpx
806c46a03cc52aa8b52e1a26d8f5c0a40a1e95c3453f9f488d6f92d3e7ca30d9  T6_after_insert_row.hwpx
e0d332a561fed3cde95f864b0ab80708f923938a86b04764aef1f6fee150bd92  T6_after_insert_row_base.hwpx
9341d054b596c64bc3c8b0226dc779b9c87ba13fdb978438bf8762423895be5f  T6_after_delete_row.hwpx
```
