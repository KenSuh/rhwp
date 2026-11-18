# v353 rhwp HWPX 저장 충실도 레인 2 — 1차 구현 반환 (Claude)

- task `v353-rhwp-hwpx-save-l2` · Orca `task_13331eece7e3` / `ctx_af492a9e5472` · 단계 initial (리뷰·수정·재리뷰는 Codex)
- worktree `/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity` · 브랜치 `v353-rhwp-table-fidelity-g0` · base `59c2285efe616211cd147dfa849d2f9f9df767dd`
- 커밋(로컬만, push 없음): `c3059ef` 소스·시험·표본·증거 → 원장 증거·보고서 커밋(이 파일을 담은 커밋, SHA 는 worker_done 본문)
- `INTENT RHWP-HWPX-SAVE-FIDELITY-L2-V353@r1 sha256:d385ca9b6e37df48347dfc71d28bfbc161c1db249304100a064347645b479366`
- lease: `mah start --task v353-rhwp-hwpx-save-l2` + 의도 결속, `mah intent show` → `status=current`
- `jev=used` — `mah impact --corpus code --code-ref c3059ef` 를 바뀐 코드 파일 6개에 돌렸다(jev=used(cache)). 결정적 결속은 이 레인 원장(pin)·레인 1 원장과 그 로그(pin)·`src/wasm_api/tests.rs`(exec, G7 전체 시험이 덮음)뿐이다.

## 게이트 원장 — 11/11 met

`gates/V353_RHWP_HWPX_SAVE_L2_GATES.md` 를 unlazy `gate-check.mjs --approve --cwd <worktree> --timeout 3600` 로 source HEAD `c3059ef` 에서 돌렸다 → `ALL MET (11 met)`. 사람이 읽을 원 출력은 `gate-raw-output.txt`.

| 게이트 | 원 출력 |
|---|---|
| G1 baseline | `BASELINE_OK 55` (59c2285, src 깨끗, G8 시험 실패) |
| G2 run 순서 | `test result: ok. 4 passed; 0 failed` |
| G3 두 번 저장 | `test result: ok. 1 passed; 0 failed` |
| G4 잔여 결함 | `test result: ok. 6 passed; 0 failed` |
| G5 음성 대조 | `NEG_OK` (59c2285 사본: run_order 3 실패 · two_save 3 실패 · residual 6 실패, 합성 문단 가드 1개만 통과) |
| G6 레인 1 | `test result: ok. 39 passed; 0 failed` |
| G7 전체 | `FAILED_TOTAL=0` — 독립 측정 cargo_exit=0 · 바이너리 17 · 1246 passed / 0 failed / 7 ignored |
| G8 clippy | `Finished` |
| G9 wasm32 | `Finished` |
| G10 범위 | `OUT_OF_SCOPE=0` (Parent 처분 2 의 두 경로 포함) |
| G11 한컴 산출물 | `HANCOM_FILES=6` |

## 성공 조건

- AC1: met (Parent 처분 1 의 알려진 한계 1 문단 제외) — `tests/hwpx_save_fidelity_l2.rs` `run_order_*` 4개. C6·FACF 열고 저장본의 2,050 문단(본문 436·셀 1,567·중첩 셀 47)을 한컴 원본 C6 와 원자 단위로 비교해 2,049 문단 일치, p195 는 IR 순서. 59c2285 에서는 55 문단 불일치(G5). 구현 `src/serializer/hwpx/table.rs` `write_paragraph_runs`/`inline_control_positions`, `section.rs` `render_paragraph_runs`.
- AC2: met — `form_attribute_entities_survive_two_hwpx_saves` 통과(G3), `two_save_first_paragraph_keeps_hancom_section_runs`(C6·FACF 두 번 저장 모두 `[secPr+colPr] [pageNum] 글자`, 구역 XML 동일), `two_save_section_def_prefix_keeps_column_in_section_run`(HWP5 처럼 구역 정의가 앞선 IR). 구현 `section.rs` `first_column`.
- AC3: met — 측정 목록과 고친 것마다 회귀 시험(아래 «G4»). 59c2285 에서 residual 6개 모두 실패(G5).
- AC4: met — G6 39 passed, G7 FAILED_TOTAL=0(독립 1246/0), G8 clippy, G9 wasm32, G10 OUT_OF_SCOPE=0, 원장 11/11, 한컴 산출물 6개.

## Parent 처분

- 처분 1 (선택지 A): 파서가 필드 아닌 컨트롤을 char_offsets 앞머리에 모으므로 C6 최상위 p195(공백 5개 + 글자처럼 취급하는 표)는 «표 → 공백» 으로 저장된다. 시험은 그 문단만 IR 순서를 기대하고, 그런 원본 문단이 정확히 하나임을 단언한다. 파서 ctrl_offset 정밀화는 후속 레인 후보.
- 처분 2 (선택지 B): `tests/hwpx_roundtrip_integration.rs` 의 c10 세 시험·c9 여백 시험은 기대값만 새 동작으로 바꿨다(다른 assertion 불변). `src/parser/hwpx/header.rs` 는 switch case `unit=CHAR` 일 때 +1 만 더했다. 근거: knowledge/raw 의 한컴 12.30 MAC64 저장본 5개에서 case unit=CHAR 값 43개가 모두 default = case×2+1(예외 0) — `char-unit-evidence.json`. 홀수·짝수·음수 여백 두 번 저장 시험은 59c2285 에서 실패하고 지금 통과한다.

## G4 — 새 base(59c2285) 잔여 결함 측정과 처리

측정: C6·FACF 열고 저장본을 한컴 원본 C6 와, `hp:line` 은 레인 1 표본 T6 와 비교(`l2_measure.py`·`shape_deep.py`, 수치는 `baseline.json`·`after.json`).

고친 것(시험):
1. header 문단 여백·줄 간격 — switch 없이 default(2배) 값을 평문으로 써서 한컴이 case 로 읽어 들여쓰기·여백이 두 배(진단 결함 D). → HwpUnitChar case(절반, 홀수 unit=CHAR)/default switch, `xmlns:hwpunitchar`. `residual_header_para_margins_use_hancom_switch`, `residual_header_odd_even_negative_margins_survive_two_saves`.
2. header 탭 위치 — 같은 2배 평문. → tabItem 마다 switch. `residual_header_tab_items_use_hancom_switch`.
3. 묶음 안 자식 도형의 sz/pos/outMargin(원본 10개에는 없음, 자식이 쪽 기준 위치를 가진 것처럼 보임). → 쓰지 않음. `residual_group_children_match_hancom_structure`.
4. 음수 `hp:offset` 을 부호 있는 표기로 써서 파서(parse_u32)가 다시 열 때 0 — 두 번째 저장에서 묶음 자식 4개(배지) 위치가 바뀜. → 한컴처럼 32비트 부호 없는 표기. `residual_negative_shape_offset_survives_second_save`.
5. `hp:line` 에 OWPML 필수 자식 `hc:startPt`/`hc:endPt` 없음(사각형 pt0..3 없음이 오류였던 것과 같은 부류). → 씀(`startX`… 속성은 이 포크 파서가 읽으므로 남김). `residual_line_writes_start_and_end_points`.
6. 필드 — 59c2285 에서 fieldBegin/End 11/11 바이트 동일·원시 제어문자 0. 고칠 것 없음. `breakNonLatinWord`·`hh:` 접두도 이미 맞다.

남긴 것(§3.1 밖 파서·모델이 값을 담지 않아 직렬화기로 못 고침):
- 도형 `id`(파서는 instid 만 읽음, 한컴은 첫 묶음 자식 0·둘째 묶음 자식 고유값으로 일관되지 않음)·`numberingType`(PICTURE→NONE)·도형 pos `allowOverlap`/`flowWithText`·`rotateimage`·lineShape 화살표(headSz/tailSz/tailStyle)·renderingInfo 행렬 분해(IR 은 합성 행렬 하나).
- 표 `id`/`zOrder` 를 0 으로 씀(레인 1 roundtrip_diff 도 info 로 보고).
- FACF 의 `-395` offset 4개 — 파서 parse_u32 가 0 으로 읽어 FACF 열고 저장에서 이미 사라진다(GearUp 템플릿 배지 약 1.4mm 이동). 파서가 i32 도 읽어야 고쳐진다.
- 한컴 선 좌표 — 파서가 `hc:startPt`/`hc:endPt` 를 읽지 않아 0. 지금은 구조만 맞고 좌표는 (0,0).
- T6 의 polygon 4개 묶음·수식 — 직렬화기가 쓰지 않아 빠진다(새 도형 종류 쓰기 = 별도 범위).
- 묶음 안 그림 — `picture.rs` 가 §3.1 밖이라 sz/pos/outMargin 을 그대로 쓴다(C6·FACF 에는 없음).

## 설계 결정

- 컨트롤 자리: char_offsets 의 8 단위 빈칸 수에서 같은 빈칸의 필드 시작·끝 표지(field_ranges)를 빼고 남은 자리를 컨트롤 순서대로 채운다. 글자가 없으면 모든 컨트롤이 앞. 글자 수와 char_offsets 수가 다르면(합성 문단) 옛 경로(글자 → 컨트롤). 같은 빈칸에서는 컨트롤을 필드 표지보다 먼저 쓴다(HWPX 파서는 컨트롤을 필드 표지 앞에 모은다).
- 렌더러 `find_control_text_positions` 도 같은 빈칸 규칙을 쓰므로 저장 순서가 rhwp 자신의 화면 순서와 같아진다.
- 첫 문단: 첫 컨트롤(구역 정의 `SectionDef` 제외)이 단 정의일 때만 secPr run 에 넣는다. 다른 컨트롤 뒤의 단 정의는 제자리에 쓰고 템플릿 단 정의를 뺀다(c10 «선두 아님» 시험의 입력 순서 유지).
- 첫 문단 lineseg 자리표를 run 을 넣기 전에 채운다(앞머리 표의 셀 문단 linesegarray 를 첫 자리로 오인하지 않게).
- 빈 글자 문단은 컨트롤 다음에 빈 `<hp:t>` run 을 쓴다(진단 «객체 앞 빈 run 84 문단» 해소, 한컴은 같은 run 안 `O+t0`).

## 한컴 확인용 산출물 (G11, Parent 가 연다)

`.omo/evidence/v353-rhwp-hwpx-save-l2/hancom/` — `L2_{C6,FACF}_{open_save,insert_row,cell_text}.hwpx`(해시 `MANIFEST.txt`). 편집 대상은 Parent `gearup_gen.rs` 규칙(행 2개 이상·표시 높이 ≠ 셀 합인 첫 본문 표) = 본문 표 #7(문단 74). 볼 곳: 문단 71 「신청방법」 표와 문단 74 표가 앞 문단·본문 왼쪽에 붙는지(진단 T7 기준), 문단 들여쓰기·여백이 원본과 같은지(switch), 쪽 여백(편집 용지 20·20·15·15·10·10), 단계 배지 사각형 위치, p195 의 표 위치(알려진 한계).

## 범위 밖 관찰

1. 파서 `ctrl_offset` — 글자 뒤 인라인 개체 순서를 IR 이 담지 못한다(처분 1). 이 정밀화를 하면 FACF 처럼 rhwp 가 «글자 먼저» 로 쓴 템플릿은 저장으로 더 이상 복구되지 않으니 템플릿 재생성과 함께 다뤄야 한다.
2. 파서가 도형 `hp:offset` 을 parse_u32 로, 선 좌표를 startX 속성으로만 읽는다(위 «남긴 것»).
3. header `lineSpacing` 의 SpaceOnly 를 `BETWEEN_LINES` 로 쓰는데 파서는 `SPACEONLY|SPACE_ONLY` 만 읽는다(기존, 손대지 않음).
4. 레인 1 원장 G11(범위, base 01a67b3)은 레인 2 커밋을 담은 HEAD 에서 다시 돌리면 OUT_OF_SCOPE>0 이 된다. 레인 1 증거는 그 head(59c2285) 기준으로 읽어야 한다. 레인 1 G2 측정(`tools/roundtrip_diff.py` T1·T2·T6·T7)은 이 HEAD 에서도 `ATTR_LOSS_TOTAL=0`.
5. 홀수 탭 위치 — 한컴 저장본 9,114 tabItem switch 가 모두 짝수라 근거가 없다. 홀수면 case = 위치/2(버림)라 rhwp 재읽기에서 1 줄어든다.
6. `cargo fmt --check` 는 이 레인 밖 파일(document.rs·object_ops.rs·table.rs 시험 모듈·hwpx_roundtrip_integration.rs 기존 부분)에서 이미 차이가 있다. 이 레인이 바꾼 줄은 rustfmt 깨끗하다.
