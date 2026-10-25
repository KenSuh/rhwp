# v353 rhwp 표 충실도 — Codex correction C1 반환

C1 수정·로컬 검증 완료, 별도 R2 재리뷰 대기. entry는 `1e87b0e5507ff3204f9729e9de800e8163ae267c`, source checkpoint는 `00cb7458986b089110c153c787c1c7b121db93ea`다. 이 보고서를 담은 후속 커밋은 원장·증거만 담으며 최종 반환 SHA는 worker_done에 있다. push·PR·merge·서브모듈 변경은 하지 않았다.

Orca task/dispatch: `task_f42d945d4913` / `ctx_db1941071e98`; executor lease: task `v353-rhwp-table-fidelity`, role executor, scope repo. 원 패킷·처분 1/2·원 보고서 §10과 R1 리뷰 및 재현 파일 두 개를 읽었다.

## Finding별 수정과 재현

| Finding | 수정 위치·동작 | entry RED → source GREEN |
|---|---|---|
| F1 표시 높이 | `src/document_core/commands/text_editing.rs:984,1185` — 셀 높이 증가 여부와 무관하게 치수를 동기화하고 common 크기 변경을 changed/dirty에 반영; `:505,520` — 줄 진행 용량 부족으로 리플로우를 못 하는 경우도 표시 높이를 동기화하고 부모를 재배치 | 일반 경로 1000→기대 200000, 정상 리플로우 및 cell-path 1000→기대 200100이 entry에서 실패하고 모두 GREEN; 행 높이가 그대로인지와 HWPX sz.height도 확인, 더 큰 표시 높이 보존 대조 통과 |
| F2 무효 resize | `src/document_core/commands/table_ops.rs:1070,1086,1454,1468` — 요청 delta 대신 실제 전후 get_column_widths 비교 | 일반·path clamp가 5000을 9600으로 바꿔 entry에서 실패, GREEN에서는 5000 보존; 무효 인덱스·상쇄 delta·열 최대 폭 미만의 셀 변경도 보존, 실제 열 변경은 10000으로 갱신하고 HWPX 출력 확인 |
| F3 열 분할 | `src/model/table.rs:1187,1339` — 분할 전후 열 폭 벡터 비교(길이에 colCnt 변화 포함) | 셀 [500,500], spacing=100인 결과가 entry 1200으로 실패, GREEN 1300; 열 변화 없는 행 방향 분할은 선언 폭 5000 보존 |

회귀 시험 위치: `tests/hwpx_table_fidelity.rs:765,770,775,780`(F1), `:834,839,844`(F2), `:883`(F3). 테스트 API·원본 시험 prefix는 유지했다. reviewer legacy 파일의 HTML producer 오프셋 관찰은 R1에서 base부터 존재한 결함이며 C1 범위로 확장하지 않았다.

## 다른 호출부 감사

| 호출부 | 판정·조치 |
|---|---|
| `Table::delete_row`, `src/model/table.rs:779,822` | 추가 결함 수정: 열 최대 폭을 제공하던 행 삭제에서 실제 열 폭이 달라지면 공식 적용, 그대로면 선언 폭 유지; `tests/hwpx_table_fidelity.rs:905,938`, entry 7000 / 기대 5000 실패 → GREEN |
| `Table::set_column_widths`, `src/model/table.rs:449,460,469` | 추가 결함 수정: 검증된 절대 폭을 셀에 실제 적용할 때 값이 달라졌는지 판정; 아무 셀 폭도 바뀌지 않으면 유효 선언 폭 보존; `tests/hwpx_table_fidelity.rs:920`, entry 3000 / 기대 7000 실패 → GREEN; 전부 병합된 표의 열 추출 기본값 1800을 쓰지 않고 실제 할당 값으로 선언 폭 산출하는 기존 동작도 `:984`로 확인 |
| `Table::insert_row`, `src/model/table.rs:578,656` | 기존 get_column_widths를 새 셀에 복제하고 colSpan은 바꾸지 않아 열 폭이 유지됨; 별도 선언 폭 5000 보존 시험 `tests/hwpx_table_fidelity.rs:956` 통과, 수정 불필요 |
| `Table::insert_column/delete_column`, `src/model/table.rs:751,900` | 유효 인덱스에서 실제 col_count를 ±1 변경하므로 공식 재계산 타당; 기존 spacing 회귀 및 전체 시험 통과, 수정 불필요 |
| 텍스트 세 경로의 false | 셀 폭·col_count를 바꾸지 않으므로 false 타당; height-only 갱신과 dirty/changed는 F1에서 수정 |
| split_cell_into의 행 분할·split_cells_in_range | 실제 전후 벡터 비교로 행 분할은 보존하고 열 분할만 재계산; 범위 분할은 같은 함수에 위임 |
| merge_cells/split_cell 및 normal/path wrapper | 병합·해제는 col_count를 유지하며 임의 격자 재유도는 P2-8 비목표; 별도의 columns_changed 결정은 없고 명령 wrapper는 모델 함수에 위임하므로 중복 수정 없음 |

## 검증과 원장

`gates/V353_RHWP_TABLE_FIDELITY_GATES.md`: **12/12 met**, unmet 0, abandoned/deferred 0. G2·G4·G5·G6·G8·G9·G10·G11·G12를 source checkpoint에서 재실행했다. G1·G3·G7은 entry 원장 증거와 `carry-inputs.json`의 입력 동일 근거를 NOTE로 이어 썼다. G3의 serializer 시험 29개는 G8에서도 모두 실행됐다.

| 증거 | 결과 |
|---|---|
| `red-source.json`, `red-1e87b0e.log` | 최종 시험으로 entry 소스 실행: rc101, 21 passed / 8 failed / 1 ignored |
| `green-focused.log` | 같은 최종 시험 SHA-256: rc0, 29 passed / 0 failed / 1 ignored; RED 8개 전부 GREEN |
| `g2.log` | T1·T2·T6·T7 각각 loss=0, ATTR_LOSS_TOTAL=0 |
| `g4.log`, `g5.log`, `g6.log` | pageBreak 4 / 표시 높이 9 / 폭 공식 14 passed |
| `g8.log` | 전체 cargo test rc0, 1162 passed / 0 failed / 3 ignored, 16 묶음 |
| `g9.log`, `g10.log` | cargo clippy -- -D warnings, wasm32-unknown-unknown release 빌드 각각 rc0 |
| `hancom-regenerated.log` | T1/T6 head 산출물 6개 재생성, 전체 8개; 원 산출물 바이트 동일 |
| `gate-run.log`, `verification.json` | 실제 9개 게이트 PASS + 기존 3개 carry, ALL MET (12 met), source SHA-256 결속 |

RED는 현재의 동일 시험을 두고 수정한 소스 세 파일만 entry의 정확한 Git blob으로 잠시 바꿔 실행한 뒤 finally로 복원했다. 그 밖의 소스는 당시 entry와 같았다. `red-precondition.log`는 path의 px 올림이 셀 높이를 50 HU 늘린 초기 전제 오류라 제외했고, `green-attempt-reflow-bypass.log`는 미완성 수정 실행이라 최종 GREEN 근거로 쓰지 않았다. 최종 RED/GREEN 시험 digest는 `c67a13b797813604e21fba636d66093f2847862807fe793c65f1f43b3a46e840`다.

G8 CHECK는 pipefail로 cargo의 실패 코드를 보존하도록 강화했다. 컴파일 실패를 빈 grep 결과의 FAILED_TOTAL=0으로 잘못 통과시키지 않으며 각 실행의 원 출력도 저장한다. 전체 suite·clippy·WASM은 이 수정 소스에서 각각 한 번만 실행했다.

## 범위·의도·인계

제품 소스 변경은 `src/model/table.rs`, `src/document_core/commands/table_ops.rs`, `src/document_core/commands/text_editing.rs` 세 파일뿐이고 시험·원장·이 C1 증거는 §3.1 안이다. 처분 1의 common_obj_attr_writer.rs는 원 base 대비 +1/-0 유지, renderer diff 0. 기존 시험 파일은 entry 버전의 바이트 동일 prefix에 C1 사례만 더했다.

INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7

- AC1: met — G1 baseline 471/22 carry, G2 네 표본 손실 0 재실행
- AC2: met — 처분 2의 TABLE/CELL/NONE 보존·ROW→CELL 정규화와 HWP5 비트 시험 G4 4 passed; parser/serializer·enum·렌더러 판단 입력은 entry와 동일
- AC3: met — F1/F2/F3와 추가 두 호출부 RED→GREEN, G5 9 / G6 14 passed; 원 a34d88ff 음성 대조는 G7에 carry
- AC4: met — 전체 1162/0/3, clippy·WASM 통과, 허용 범위, 원장 12/12

jev=used — `impact-summary.json`: full code ref `00cb7458986b089110c153c787c1c7b121db93ea`에서 변경 소스·시험·원장 5개 impact 실행, top=0 결정적 pin/exec 모두 보존; 점수로 제외하지 않음. 원 패킷·처분 1/2의 vault 질의만 후보 0으로 unused였고 결정적 결과로 계속했다. graphify query는 이 worktree에 graph.json이 없어 사용할 수 없었고 직접 파일·호출부 rg와 실행으로 확인했다.

자기 리뷰 receipt나 lane receipt는 발행하지 않았다. 이 결과는 독립 재리뷰가 아니며 코디네이터가 별도 R2를 발주한다. 한컴에서 실제 열기·시각 확인은 Parent 소관이고, P2-6·P2-8·Phase 3–5·T5 그림 오류·기존 HTML producer 결함은 범위 밖으로 유지한다.
