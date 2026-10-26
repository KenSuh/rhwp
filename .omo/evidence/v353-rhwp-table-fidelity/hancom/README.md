# 한컴 확인용 산출물 (v353 C3, 게이트 G12)

새 base는 `01a67b3f77760b5a9dae9fc155d83f8ca84ab529`, source head는
`dbf294757690e723c614c22e467022522ea3b44e`다. `write_hancom_review_files`의
`head` 6개와 byte-verified 새 base archive의 `base` 2개를 다시 생성했다.

경로: `DocumentCore::from_bytes` → 행 삽입/삭제 또는 셀 텍스트 편집 → `export_hwpx_native`.
명령과 원 출력은 `../c3/hancom-head.log`, `../c3/hancom-base.log`에 있다.
아래 수치는 실제 XML에서 재측정했다. ZIP/XML 및 금지 제어문자 부재를 검사했으며,
한컴 앱 열기·시각 확인은 Parent 소관이다.

표시 높이 규칙: 행 삽입은 편집 전 높이에 표시 기준 행 높이를 더하고, 행 삭제는
max(편집 전 높이 − 삭제 행의 표시 기준 높이, 셀 합)이며, 텍스트 편집은 셀 합이 넘을 때만 늘린다.

| 파일 | source | 본문 표 | 높이 | 폭 |
|---|---|---|---:|---:|
| `T1_after_insert_row.hwpx` | dbf2947 | T1 #58 | 28253 | 47624 |
| `T1_after_insert_row_base.hwpx` | 01a67b3 | T1 #58 | 5144 | 47624 |
| `T1_after_text_edit.hwpx` | dbf2947 | T1 #58 | 14901 | 47624 |
| `T1_before_edit.hwpx` | dbf2947 | T1 #58 | 14901 | 47624 |
| `T6_after_delete_row.hwpx` | dbf2947 | T6 #0 | 53721 | 47903 |
| `T6_after_insert_row.hwpx` | dbf2947 | T6 #0 | 93179 | 47903 |
| `T6_after_insert_row_base.hwpx` | 01a67b3 | T6 #0 | 7396 | 47903 |
| `T6_before_edit.hwpx` | dbf2947 | T6 #0 | 73450 | 47903 |

한컴에서는 행 삽입 head/base 쌍과 편집 전·후 배치를 비교한다.

## SHA-256

```text
54a5e0763800791d261bc6dbbd444ca55817caf3643321433be120ee22590bf5  T1_after_insert_row.hwpx
a0ae073cfe84ba144aca808faba20215c144630f6aa0b883b852eefaf389435e  T1_after_insert_row_base.hwpx
b8ae886f2f78aa4ea4777e036b346d5016cc10c5fc373aee0b63f1c399cbc14a  T1_after_text_edit.hwpx
04c591973f933c81b6f470e51c01452090409d58d499ff2b320714cf906eb910  T1_before_edit.hwpx
b407b3dd838cfe694850a9e888aa2993512f52bbdb3ccfda8c6cd3978a89d25a  T6_after_delete_row.hwpx
0b8265dcc2ada3891f9492a57fb0a8bbfd8791cda8135f2b7071ca73d2a96bad  T6_after_insert_row.hwpx
df1bd8bf01749eed3b2ef59362c3b8c4da0e4307c613477a6ba3e47ca06fdd93  T6_after_insert_row_base.hwpx
f1ae941e93b4e167696cb86e8f513c34dfa7c4e1c801db50e801d2cd0d251bff  T6_before_edit.hwpx
```
