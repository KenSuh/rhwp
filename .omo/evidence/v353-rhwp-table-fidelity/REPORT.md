# v353 rhwp HWPX 표 충실도 P0–P2 — 1차 구현 반환 (Claude)

- task `v353-rhwp-table-fidelity` · Orca `task_d0e83437b94a` / `ctx_72c050282d39` · 단계 initial (리뷰·수정은 Codex)
- worktree `/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity` · 브랜치 `v353-rhwp-table-fidelity` · base `a34d88ff3bae317ad08d8f0b99504e4395278e6a`
- 커밋(로컬만, push 없음): `6e05aec` P0 하네스·baseline → `45d4f16` P1–P2 구현 → 원장 증거·보고서 커밋(이 파일을 담은 커밋 — SHA 는 worker_done 본문)
- `INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7`
- lease: `mah start --task v353-rhwp-table-fidelity` + 의도 결속, `mah intent show` → `status=current`
- `jev=used` — `mah impact --corpus code --code-ref 45d4f16` 를 바뀐 소스 9개에 돌렸다. 결정적 결속(pin/exec)은 이 레인 원장(G11 경로 목록)·`src/wasm_api/tests.rs`(exec, G8 전체 시험이 덮음)·문서뿐이다.

## 게이트 원장 — 12/12 met

`gates/V353_RHWP_TABLE_FIDELITY_GATES.md` 를 unlazy `gate-check.mjs --approve --cwd <worktree> --timeout 3600` 으로 source HEAD `45d4f16` 에서 돌렸다 → `ALL MET (12 met)`. 원장에는 `automatic-evidence=v1` 지문만 남으므로 사람이 읽을 원 출력을 `.omo/evidence/v353-rhwp-table-fidelity/gate-raw-output.txt` 에 따로 남겼다. 마지막 커밋은 원장 증거·보고서·원 출력만 더하고 소스는 바꾸지 않는다(G11 은 그 커밋에서 다시 돌려 `OUT_OF_SCOPE=0`).

| 게이트 | 상태 | 원 출력 | definition-sha256 앞 12자 |
|---|---|---|---|
| G1 변경 전 baseline | ✓ met | `BASELINE_OK 471 22` | `3ae476367c9e` |
| G2 속성 보존 | ✓ met | `ATTR_LOSS_TOTAL=0` (T1·T2·T6·T7 각 loss=0) | `99eee8a5f69b` |
| G3 속성 보존 단위 시험(noAdjust·lock·sz | ✓ met | `test result: ok. 29 passed; 0 failed` | `5711150d7db7` |
| G4 pageBreak 4값 | ✓ met | `test result: ok. 4 passed; 0 failed` | `f28ebdc4c823` |
| G5 표시 높이 | ✓ met | `test result: ok. 5 passed; 0 failed` | `a1d733c00db0` |
| G6 폭 공식 | ✓ met | `test result: ok. 4 passed; 0 failed` | `f119aa1a9367` |
| G7 음성 대조 | ✓ met | `NEG_OK` (base: page_break 0/4·display_height 1/5·width_formula 1/4 통과) | `c3139c1e1432` |
| G8 전체 시험 | ✓ met | `FAILED_TOTAL=0` (별도 집계 1148 통과·0 실패·3 무시, 16 묶음) | `63996319e8a0` |
| G9 clippy | ✓ met | `Finished` (clippy -D warnings) | `3e5152971bf3` |
| G10 WASM 빌드(에디터가 쓰는 대상) | ✓ met | `Finished` (wasm32 release) | `8899a6f30c11` |
| G11 범위 | ✓ met | `OUT_OF_SCOPE=0` | `12d010e03d44` |
| G12 한컴 확인용 산출물 | ✓ met | `HANCOM_FILES=8` | `b667f896cb71` |


## 성공 조건

- AC1: met — `.omo/evidence/v353-rhwp-table-fidelity/baseline.json`(head a34d88ff, src_dirty=false) T1 471·T2 22 (G1 `BASELINE_OK 471 22`); 수정 head 에서 `python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary` → `ATTR_LOSS_TOTAL=0` (G2).
- AC2: met (Parent 처분 2 의 ROW 해석) — `tests/hwpx_table_fidelity.rs:282·295·308·341` (G4 `4 passed`; TABLE·CELL·NONE 두 경로 원 문자열 보존, ROW→CELL 정규화, HWP5 비트 1·2·0); 매핑 `src/parser/hwpx/section.rs:869`·`src/serializer/hwpx/table.rs:684`; enum 의미 `src/model/table.rs:55-75`; 근거 `pagebreak-pairs.json`(CELL↔2 760/761·TABLE↔1 4/4·NONE↔0 252/255)·OWPML 스키마 `ParaList XML schema.xml:2138`·렌더러 미분기 `engine.rs:1690·1748`/`typeset.rs:1186`.
- AC3: met — 표시 높이 `tests/hwpx_table_fidelity.rs:455·469·490·511·537` (G5 `5 passed`), 폭 공식 `:581·593·608·675` (G6 `4 passed`); a34d88ff 사본에서 page_break 0/4·display_height 1/5·width_formula 1/4 통과(`negative-control.json`, G7 `NEG_OK`); 구현 `src/model/table.rs:336·346·381·424·440·577·777`.
- AC4: met — G8 `FAILED_TOTAL=0`(1148 통과·3 무시), G9 clippy `-D warnings` `Finished`, G10 wasm32 release `Finished`, G11 `OUT_OF_SCOPE=0`(처분 1 허용 경로 포함, 그 파일 `+1 −0`), 원장 12/12 automatic-evidence.

## Parent 처분

- 처분 1(선택지 A): `src/document_core/converters/common_obj_attr_writer.rs` 의 `#[cfg(test)] make_sample()` 리터럴에 `..Default::default()` 한 줄만 추가. 그 파일 diff = **+1 −0**(`git diff --numstat a34d88ff HEAD -- <파일>` → `1 0`). G11 허용 목록에 추가하고 NOTE 를 남겼다.
- 처분 2(선택지 A): 보고서 pageBreak 매핑 채택, enum 문서 주석만 한컴 의미로 정정, ROW 는 CELL 로 정규화. G4 행에 NOTE.

## §4 pageBreak 판단 근거

| 지점 | a34d88ff | head |
|---|---|---|
| HWPX 파서 `src/parser/hwpx/section.rs` `parse_table_page_break` | CELL→CellBreak, ROW→RowBreak, TABLE→None | TABLE→CellBreak, CELL/ROW→RowBreak, NONE→None |
| HWPX 직렬화 `src/serializer/hwpx/table.rs` `table_page_break_str` | CellBreak→CELL, RowBreak→TABLE | CellBreak→TABLE, RowBreak→CELL |
| HWP5 파서 `src/parser/control.rs:251` | 1·3→CellBreak, 2→RowBreak | 변경 없음 |
| HWP5 직렬화 `src/serializer/control.rs` | CellBreak→1, RowBreak→2 | 변경 없음(아래 noAdjust 규칙만 추가) |

1. **문자열↔HWP5 값** — 업스트림 표본의 같은 이름 HWP/HWPX 쌍 144쌍 중 표 수가 맞는 36문서·1,020표를 표별로 대조: `CELL↔2` 760/761, `TABLE↔1` 4/4, `NONE↔0` 252/255 (`.omo/evidence/v353-rhwp-table-fidelity/pagebreak-pairs.json`, 스크립트·프로브 동봉).
2. **값의 한컴 의미** — 업스트림 `mydocs/working/archives/task_m100_713_stage2.md` §2 가 한컴 원 소스 `HwpDef.h`·`HwpTable.cpp:1802` 를 인용: 1=`HWPTABLE_BREAK_TABLE`(표는 나누되 셀은 나누지 않음), 2=`HWPTABLE_BREAK_CELL`(셀 안 텍스트도 나눔). 한글 2022 PDF 로 값 2 표의 인트라-로우 분할을 확인했다는 기록.
3. **OWPML 스키마** — 저장소 `mydocs/manual/OWPML SCHEMA/ParaList XML schema.xml:2138` 의 `hp:tbl@pageBreak` 열거 = `TABLE | CELL | NONE`(기본 CELL). `ROW` 는 스키마 값이 아니다(코퍼스 4,625표 0회).
4. **렌더러와의 관계** — 렌더러는 page_break 로 분기하지 않는다(`src/renderer/pagination/engine.rs:1690·1748`, `src/renderer/typeset.rs:1186` 의 `can_intra_split = !mt.cells.is_empty()`; `src/renderer/pagination/tests.rs` `test_table_none_also_intra_row_split`). 매핑 변경으로 렌더링은 바뀌지 않는다. 다만 `height_measurer.rs:114` 주석과 pagination 시험 이름은 옛 뜻(CellBreak=인트라-로우)을 쓴다 — 처분 2 대로 범위 밖 관찰.
5. **표 속성 대화상자** — `table_ops.rs:1625` 정수(None=0·CellBreak=1·RowBreak=2)와 스튜디오 라벨(2=나눔, 1=셀 단위로 나눔)이 새 매핑에서 한컴 표시와 일치한다.

## 설계 결정

- **CommonObjAttr 6필드** (`locked`·`size_protect`·`flow_with_text`·`allow_overlap`·`affect_line_spacing`·`text_flow`, 업스트림 이름). `Default` 는 손으로 구현해 `flow_with_text=true`(한컴 기본). HWP5 파서(`parser/control/shape.rs`, 범위 밖)와 편집기가 만든 표는 이 필드를 채우지 않으므로, 파생 기본값 false 면 그런 표의 HWPX 출력이 `flowWithText="1"`→`"0"` 으로 바뀐다.
- **holdAnchorAndSO** → 기존 `prevent_page_break`, **widthRelTo/heightRelTo** → 기존 `width_criterion/height_criterion` (쌍 대조 1019–1020/1020 일치, `flag-pairs.json`).
- **noAdjust** → `raw_table_record_attr` bit 3(`TABLE_RECORD_NO_ADJUST`, 쌍 대조 1020/1020). HWPX 파서는 bit 3 만 담는다. HWP5 저장(`serializer/control.rs`)은 bit 3 만 담긴 raw 를 원본이 아닌 값으로 보고 bit 0-2 를 IR 에서 다시 만든 뒤 bit 3 을 잇는다 — 열어 둔 뒤 표 속성에서 바꾼 쪽 나눔이 HWP5 에 실린다(`no_adjust_table_pagebreak_edit_reaches_hwp5`). 다른 비트가 있는 raw(HWP5 원본, 편집기 0x06/0x04000006)는 종전처럼 그대로 쓴다. (처음에는 쪽 나눔 비트까지 raw 에 채웠으나, 표 속성 편집이 HWP5 에 반영되지 않는 회귀가 생겨 바꿨다.)
- **셀 protect/editable** → `list_header_width_ref` 비트 0x02·0x08(업스트림과 같은 자리, HWP5 파서·`table_ops` 가 이미 0x02/0x04 를 같은 필드에서 쓴다). **dirty** → `dirty_flag`, **lineWrap** → `line_wrap: u8`(BREAK 0·SQUEEZE 1·KEEP 2). 행 삽입 시 새 셀은 템플릿 셀의 `line_wrap` 을 잇는다.
- **캡션·cellzoneList** 방출 복원(P1-15). 캡션 문단은 셀과 같은 문단 쓰기 경로(`write_sub_list_paragraphs`)를 쓴다.
- **pos 오프셋** 을 부호 있는 값으로 쓴다(IR u32 에 담긴 음수가 `4294967xxx` 로 새던 것).
- **표 기하** — `update_ctrl_dimensions` 는 원시 재계산(폭 = Σ열 폭 + (colCnt+1)×cellSpacing, 높이 = 셀 합)으로 두고 편집 경로마다 정책을 둔다.
  - 행 삽입: 표시 높이 = max(편집 전 + 새 행의 표시 기준 높이, 셀 합), 선언 폭 유지.
  - 행 삭제: max(편집 전 − 삭제 행의 표시 기준 높이, 셀 합), 선언 폭 유지.
  - 열 삽입·삭제: 폭은 공식으로 다시 계산, 표시 높이는 줄이지 않음.
  - 텍스트 편집(`text_editing.rs` 두 곳)·셀 크기 조절(`table_ops.rs` 두 곳)·셀 나누기: `refresh_ctrl_dimensions_after_content_edit` — 열 폭이 바뀐 경우에만 폭 재계산, 표시 높이는 셀 합이 넘을 때만 늘림.
  - `stretched_row_heights`·`sync_ctrl_height` 는 업스트림 1a76570e 이식(출처 주석). `set_column_widths`(P2-7) 추가 — 병합 셀은 덮는 열 합 + (span−1)×간격.
- **CTRL_HEADER 오프셋 결함 수정** — `raw_ctrl_data` 는 HWP5 파서가 attr 포함 CTRL_HEADER 전체를 담는다(폭 [12..16], 높이 [16..20], `object_ops.rs` 도 같은 배치). 종전 `update_ctrl_dimensions` 는 [8..12]/[12..16] 에 써서 HWP5 원본 표를 편집하면 가로 오프셋 자리에 폭을 덮어썼다. 그 결함을 그대로 기대하던 기존 단위 시험 `test_update_ctrl_dimensions_syncs_raw_ctrl_data` 를 실제 배치로 고쳤고, `row_edit_keeps_hwp5_ctrl_header_layout` 이 HWP5 파서로 되읽어 확인한다.

## 범위 밖으로 남긴 관찰

1. **T5 직렬화 오류** — `<hp:pic> binaryItemIDRef 미등록 bin_data_id=0 (BinDataContent 누락)`, base·head 같음(Phase 4). `t5-observation-base.json`.
2. **HWPX→HWP5→HWPX 그림 경로** — 그림이 있는 T1·T5·T6·T7 은 HWP5 를 거친 뒤 HWPX 저장이 `binaryItemIDRef 미등록 … BinDataContent 누락` 으로 실패한다. base 와 head 가 같다(`hwp5-path-observation.txt`). 그래서 G4 의 HWP5 경로 시험은 그림 없는 T2 로 한다.
3. **렌더러 문서·시험 이름** — `height_measurer.rs:114` 주석, `pagination/tests.rs` 의 "CellBreak 인트라-로우" 시험 이름이 한컴 의미와 반대다(처분 2). 렌더러는 모든 표에 인트라-로우 분할을 적용한다.
4. **셀 hasMargin·name, 표 id·zOrder 미파싱** — INFO 손실로 남김(T1 hasMargin 5,722·name 11, 모든 표 id=0·zOrder=0 로 저장). hasMargin 을 파싱하면 `converters/hwpx_to_hwp.rs` `adapt_cell_list_attr` 가 HWP 저장 때 살아 있는 문서의 `text_direction` 을 1 로 바꾸는 우회가 깨어나(T6 579셀) 세로쓰기로 그려질 위험이 있어 손대지 않았다.
5. **subList textDirection** — HWPX subList 에서 읽지 않는다(옛 tcPr/cellPr 만). 이 레인 표본은 전부 HORIZONTAL.
6. **HWP5 출처 표의 새 6플래그** — `parse_common_obj_attr`(범위 밖)가 attr bit 2·13·14·20·24-25·30 을 필드로 풀지 않아, HWP5 문서를 HWPX 로 저장하면 종전 출력(flowWithText=1, 나머지 0)을 유지한다. 비트 대응은 `flag-pairs.json` 으로 확인됐다.
7. **HWPX→HWP5 의 6플래그** — 어댑터 `pack_common_attr_bits`(범위 밖)가 bit 2·13·14·20·24-25·30 을 넣지 않는다. HWP5 셀 lineWrap 비트도 `serialize_cell` 이 쓰지 않는다(protect/editable 은 `list_header_width_ref` 로 실린다).
8. **표 속성 대화상자** — `table_ops.rs:1734·1861` 이 allowOverlap·keepWithAnchor 를 `table.attr`·`raw_ctrl_data` 에서 읽어 HWPX 표에는 파싱한 값이 보이지 않는다. 주석의 `raw_ctrl_data[32..36] = prevent_page_break` 는 전체 배치에서 [36..40] 이다.
9. **html_table_import.rs** 는 Task 271 이전 배치(폭 [8..12])로 `raw_ctrl_data` 를 만든다. 이번 오프셋 수정 뒤 그 표를 편집하면 폭·높이는 올바른 자리에 써진다.
10. **HWP5 원본 표의 쪽 나눔 편집** — raw 에 다른 비트가 있으면 `set_table_properties` 의 쪽 나눔·제목 줄 반복 변경이 HWP5 에 실리지 않는다(업스트림 #7288 과 같은 종전 결함). 편집기 생성 표는 IR(`page_break: None`)과 raw(0x06)가 어긋나 있어, 일괄 동기화는 그 표의 HWP5 출력을 바꾸므로 하지 않았다.
11. **캡션 문단** — 셀 문단과 같은 단순 쓰기 경로라 자동 번호 등 인라인 컨트롤은 나가지 않는다(Phase 3 영역). 종전에는 캡션 전체가 사라졌다.
12. **typeset 의 TAC 판정** — `typeset.rs:712` 가 `table.attr & 0x01` 을 읽는데 HWPX 표는 `table.attr=0` 이다(종전과 같음).
13. **병합 셀 + 셀 간격 열 삽입** — 병합 셀 폭에 새 열 폭만 더하고 간격은 더하지 않는다(종전과 같음, G6 은 병합 없는 표 대상).
14. P2-6(`flow_width_hu` 호출부)·P2-8(`TableResizeCell`)·Phase 3–5 는 하지 않았다.
15. 프로젝트 CLAUDE.md 의 하이퍼-워터폴 문서(`mydocs/plans` 등)는 패킷 §3.1 파일 범위 밖이라 쓰지 않았다.

## 재현

```bash
python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary   # ATTR_LOSS_TOTAL
cargo test --test hwpx_table_fidelity
cargo test --lib serializer::hwpx::table
node "<unlazy>/scripts/gate-check.mjs" --status gates/V353_RHWP_TABLE_FIDELITY_GATES.md
```
