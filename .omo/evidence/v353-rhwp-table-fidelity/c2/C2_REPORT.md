# rhwp HWPX 표 충실도 C2

R2의 P2 무변경 요청 회귀를 수정했다. 빈 삽입·0개 삭제·텍스트 끝 삭제는 유효한 셀/문단을 확인한 뒤 반환하므로 높이 동기화, dirty 마킹, reflow, raw 무효화 및 변경 이벤트를 실행하지 않는다. 실제 편집의 C1 grow-only 높이 규칙과 폭 수정은 유지했다.

- Entry: `e6e5850ee07c952f2908df8e5845124c224c2f90`
- 소스 체크포인트: `7668ec27182792350b7aa8d43b52c0e14320919e` → `9ac673ce16034fc03d819e5186485a920e5d2c16`
- Task/Dispatch: `task_4f268eb31c56` / `ctx_325f26bb9aa1`
- 이후 커밋은 이 보고서·원장·증거만 포함한다. push 및 자기 리뷰 receipt는 없다.

## 수정과 자체 점검

| 경로 | 수정 위치 | 결과 |
|---|---|---|
| 일반 삽입 | `src/document_core/commands/text_editing.rs:287` | 빈 문자열은 셀 검증 뒤 반환 |
| 일반 삭제 | 같은 파일 `:344` | `delete_text_at`의 실제 삭제 수가 0이면 반환 |
| path 삽입 | 같은 파일 `:3221` | 빈 문자열은 전체 path 검증 뒤 반환 |
| path 삭제 | 같은 파일 `:3259` | 실제 삭제가 없으면 부모 표까지 refresh하지 않음 |
| 추가: 셀 선택 영역 삭제 | 같은 파일 `:1249` | 단일 문단의 실제 삭제가 없으면 raw 무효화 전 반환 |

요청한 8개 시험은 `tests/hwpx_table_fidelity.rs:1071`부터 각각 독립 시험으로 옮겼다. 2×2, 행 높이 `[100050,100050]`, 표시 높이 1000을 사용해 텍스트·행 높이·표 dirty·IR 표시 높이·저장 XML 높이와 HWPX 전체 바이트 동일성을 확인한다. 추가 시험 `:1111`은 public `delete_range_in_cell`의 셀 0/1 × 0길이/텍스트 끝 이후 삭제를 검증한다.

추가 경로는 현재 세션의 호출부 감사에서 같은 원인으로 재현됐다(`red-range-7668ec2.log`). 문단 분할/병합은 유효한 요청이면 문단 구조가 실제로 바뀌므로 기존 동기화를 유지한다. 네 삽입/삭제 API의 셀 검증과 성공 응답 offset은 유지했으며, 셀 선택 영역의 빈 요청도 셀/문단을 먼저 검증한다. 현재 세션에서 `e6e5850..9ac673c` 전체 diff와 형제 호출부를 직접 재독한 뒤 추가 substantive finding은 없었다. 이 자체 점검은 독립 보증이 아니다.

## RED / GREEN

| 검증 | 실제 결과 | 증거 |
|---|---|---|
| 요청한 8개, entry 소스 | rc101, 0 passed / 8 failed; 높이 200100 ≠ 1000 | `red-e6e5850.log` |
| 최종 동일 시험, entry snapshot의 독립 target | rc101, 0 passed / 9 failed | `red-final-e6e5850.log`, `red-source-binding.json` |
| 최종 소스 no-op | 9 passed / 0 failed | `green-noop-final.log`, `g5.log` |
| 표 충실도 전체 | 38 passed / 0 failed / 1 ignored | `g8.log`, `verification.json` |
| G4 / G5 / G6 | 4 / 18 / 14 passed, 각각 실패 0 | `g4.log`, `g5.log`, `g6.log` |
| 전체 cargo test | 1171 passed / 0 failed / 3 ignored, 16 묶음 | `g8.log` |
| clippy `-D warnings` / WASM release | 모두 rc0 | `g9.log`, `g10.log`, `gate-run.log` |

최종 시험 SHA-256: `0bc0cf833692199a509d985fbf720fca1e9bdfa03134beec52f2012a613ffac3`. RED와 GREEN의 시험 바이트가 같고, 각각 src 353파일을 해당 Git blob과 대조했다. RED는 `/tmp/rhwp-c2-red.cj8w45/target`을 사용했다. GREEN은 현재 worktree의 rhwp 패키지 산출물을 비운 뒤 소스 체크포인트에서 다시 빌드했다(`clean-package.log`). Cargo 실행은 offline/locked였다.

초기 시험 작성의 반환형 컴파일 오류는 `red-initial-compile.log`로 제외했다. 원본 snapshot이 live target을 공유한 예상 밖 GREEN은 `red-final-shared-target-discarded.log`로 제외하고 독립 target에서 RED를 다시 확인했다. 중간 7668ec2의 전체 게이트 실행도 `intermediate-7668ec2/`에 보존하고 최종 증거로 사용하지 않았다.

## 원장과 범위

`gates/V353_RHWP_TABLE_FIDELITY_GATES.md`: **12/12 met**, unmet/abandoned/deferred 0. G4·G5·G6·G8·G9·G10·G11·G12를 최종 소스에서 실행했다. 재실행 8개의 원장 출력 SHA-256·bytes를 원 로그와 대조했다(`verification.json`). G5의 기존 >=3 정규식이 10–29를 제외하던 오류도 바로잡았다.

G1·G2·G3·G7은 이전 증거를 입력 동일 NOTE로 carry했다. 고정 baseline/negative-control, 편집 없는 열기→저장, 속성 serializer는 수정한 명령을 호출하지 않는다. carry 입력 19개, C1 시험 39011-byte prefix, 원 시험 27484-byte prefix가 같다(`carry-inputs.json`). G3의 serializer 시험 29개는 G8에서도 통과했다.

한컴 head 산출물 6개를 재생성했으며 총 8파일 모두 entry와 바이트가 같다(`hancom-before.json`, `hancom-after.json`, `hancom-regenerated.log`). 범위 밖 변경은 0이며 제품 diff는 `text_editing.rs`와 통합 시험뿐이다. 나머지 변경은 허용된 원장·증거 경로다. 처분 1의 `common_obj_attr_writer.rs`와 `.gitignore`는 원 base 대비 각각 +1/-0이고 C2에서는 바꾸지 않았다.

## 의도와 반환

INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7

- AC1: met — G1 baseline T1=471/T2=22 및 G2 T1/T2/T6/T7 loss=0 carry; 입력 동일 근거는 `carry-inputs.json`과 원장 NOTE.
- AC2: met — 처분 2 기준의 두 경로와 ROW→CELL, G4 4 passed; 매핑·enum·renderer 판단은 C1 입력과 동일.
- AC3: met — 기존 C1 시험을 보존했고 새 무변경 요청 9개가 exact entry에서 RED→수정본 GREEN; G5 18/G6 14 passed.
- AC4: met — 전체 cargo 1171/0/3, clippy·WASM rc0, 허용 밖 0, 원장 12/12와 원 로그 해시 대조.

jev=used — 원 패킷·원장·manifest/lock·변경 소스/시험을 impact했고 최종 source full SHA 9ac673ce16034fc03d819e5186485a920e5d2c16의 `impact-final-*.log`에 결정적 pin/exec 후보 전체를 보존했다. 처분 1/2와 C1 brief는 후보 0으로 jev=unused(후보 0)였다. Jev 점수로 결정적 후보를 제외하지 않았다.

graphify report는 이 worktree에 그래프가 없어 사용할 수 없었으며 focused rg와 직접 호출부 읽기로 대체했다. 서브에이전트·중첩 Codex·push·PR·자기 리뷰 receipt는 사용하지 않았다. 별도 R3, Parent의 한컴 앱 시각 확인과 착지 감사는 남아 있다. P2-6/P2-8, Phase 3–5, T5 그림 오류 등 범위 밖 관찰은 기존 상태로 남겼다.
