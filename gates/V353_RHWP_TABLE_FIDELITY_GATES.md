# Gates: v353 rhwp table fidelity P0–P2

- [x] G1: 변경 전 baseline — T1·T2 왕복 손실이 0 이 아님을 수정 전 커밋(01a67b3)에서 기록했다
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/c3/baseline.json"));assert d["head"]=="01a67b3f77760b5a9dae9fc155d83f8ca84ab529";assert d["T1"]["ATTR_LOSS_TOTAL"]>0 and d["T2"]["ATTR_LOSS_TOTAL"]>0;print("BASELINE_OK",d["T1"]["ATTR_LOSS_TOTAL"],d["T2"]["ATTR_LOSS_TOTAL"])'
  EXPECT: /^BASELINE_OK \d+ \d+$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=0767a902842e1a3703488d258336578c5ebb48afdcb5592865d84a643b225540; exit=0; EXPECT=matched; output-sha256=d5512ddc9de72c874df41c3f5e170ebfa5533062414fc8ea263ce25e3203fa6a; output-bytes=19; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G2: 속성 보존 — 수정 head 에서 T1·T2·T6·T7 왕복 손실 합계 0(lineWrap·noAdjust·lock·protect·pos.*·sz.*·textFlow·pageBreak)
  CHECK: bash -o pipefail -c 'python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g2.log'
  EXPECT: /^ATTR_LOSS_TOTAL=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=d772a45addd74446e9cd8040de32766f208b7f07a736fb5cad69876768a726fc; exit=0; EXPECT=matched; output-sha256=975a3b49f42c3770d08d76851c646a7fc45f2773a94ad2c75379bd8f3381f0be; output-bytes=622; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G3: 속성 보존 단위 시험(noAdjust·lock·sz protect·allowOverlap·flowWithText·textFlow·셀 lineWrap SQUEEZE·셀 protect 각 1 사례 이상)
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --lib serializer::hwpx::table 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g3.log'
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=1a6424eca037b00ce4cc26391fc344ff37d8d05d6a64fe98f0954892972f4909; exit=0; EXPECT=matched; output-sha256=1deace0b0228f7e8262f720448214cf08bc684a2f99ee86efc87e49cd545598b; output-bytes=4440; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G4: pageBreak 4값 — HWPX→IR→HWPX 와 HWPX→HWP5→HWPX 두 경로 모두 원 문자열 보존
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity page_break 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g4.log'
  EXPECT: /test result: ok\. [2-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=14618f0bd816910a34072eff93f8935c57b2f650b6d947f3388538ac60964abc; exit=0; EXPECT=matched; output-sha256=48daa9b215716d0a7af983fce2785955174eb3d8f2a4978548736853b8178bf0; output-bytes=505; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G5: 표시 높이 — T6(표시 높이 ≠ 셀 합) 행 삽입·삭제·텍스트 편집 뒤 sz.height 가 §3.2 공식대로 유지
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity display_height 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g5.log'
  EXPECT: /test result: ok\. (?:[3-9]|[1-9]\d+) passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=df977e30a421b4258a2db71128526618f98fab9cfe3fff2faff6827386415592; exit=0; EXPECT=matched; output-sha256=5366daa67b0a46e81ba265ed1edce42f0dffceabb3379e95f607ec1df4909145; output-bytes=1385; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G6: 폭 공식 — cellSpacing>0 표와 병합 없는 표에서 sz.width == Σ열 폭 + (colCnt+1)×cellSpacing
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity width_formula 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g6.log'
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=782d7c6fe97d35b140a99f3797fbc66338b18e42f8400e90c65936f596eb846d; exit=0; EXPECT=matched; output-sha256=37cfc3dccd4fb98183960a3f1f716a446d775e21e0cab4b5fc68fc81eb6b61f2; output-bytes=1272; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G7: 음성 대조 — G4·G5 시험을 수정 전 01a67b3 의 해당 함수에 적용하면 실패한다(시험이 결함을 실제로 잡는다)
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/c3/negative-control.json"));assert d["base"]=="01a67b3f77760b5a9dae9fc155d83f8ca84ab529";assert d["page_break_failed"]>0 and d["display_height_failed"]>0;print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=3247ec311e3dc8b252911c8b45a8dd91c2f54dd30ccb92592cd3529ed454935d; exit=0; EXPECT=matched; output-sha256=b5568d8ad4ed8d6a9fab8348e510711f8358c218705c2d8f502ebf2bbb470a0f; output-bytes=7; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [ ] G8: 전체 시험
  CHECK: bash -o pipefail -c 'cargo test --offline --locked 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g8.log'
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: FAILED exit=101; c3/g8.log — 1000 passed / 1 failed / 1 ignored in lib; same failure independently reproduced on verified 01a67b3 archive (c3/g0-base-form-failure.log).

ABANDON: G8 PRE-EXISTING-BASE — 01a67b3 자체에서 form_attribute_entities_survive_two_hwpx_saves 실패(두 번째 저장의 colPr run 분리); C3 재정렬 손상 아님; rhwp 레인 2 에서 수정

- [x] G9: clippy
  CHECK: bash -o pipefail -c 'cargo clippy --offline --locked -- -D warnings 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g9.log'
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=739aae5ce14aa57d20b95ab77a77801b03def630bea5cd4684d85ffaed750803; exit=0; EXPECT=matched; output-sha256=192f346a831bf242f6362c9dd6271dd991a2d249c793229d9b0ec13edb76e7db; output-bytes=156; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G10: WASM 빌드(에디터가 쓰는 대상)
  CHECK: bash -o pipefail -c 'cargo build --offline --locked --lib --target wasm32-unknown-unknown --release 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c3/g10.log'
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=22cd66a83ce347df24a7371857ef533b9c3f09b9928ad3c521139a1b60e71c28; exit=0; EXPECT=matched; output-sha256=0d3814088b37419e0b3aece1fe92d7a0c3b25c6459d26b581211698e5c0ada0b; output-bytes=147; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G11: 범위 — base 대비 변경 파일이 전부 §3.1 MUST 안
  CHECK: git diff --name-only 01a67b3f77760b5a9dae9fc155d83f8ca84ab529 HEAD | grep -vE '^(src/model/table\.rs|src/model/table/tests\.rs|src/model/shape\.rs|src/parser/hwpx/section\.rs|src/parser/control\.rs|src/serializer/hwpx/table\.rs|src/serializer/control\.rs|src/document_core/commands/(table_ops|text_editing)\.rs|src/document_core/converters/common_obj_attr_writer\.rs|tools/roundtrip_diff\.py|tests/hwpx_table_fidelity\.rs|tests/hwpx_roundtrip_integration\.rs|tests/fixtures/table-fidelity/.*|gates/V353_RHWP_TABLE_FIDELITY_GATES\.md|\.omo/evidence/v353-rhwp-table-fidelity/.*|\.gitignore)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=466d3dc471963bdd47d6ae6025244722d1ace6152b4597cb3d480f917660457a; exit=0; EXPECT=matched; output-sha256=13a10730bd1c382bb5fe6ac3f0e5a982453970f00ca15e1b5fda9516047c9824; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries

- [x] G12: 한컴 확인용 산출물 — T1·T6 의 편집 전·후 HWPX 를 .omo/evidence/v353-rhwp-table-fidelity/hancom/ 에 저장(한컴 열기·시각 확인은 Parent 가 한다)
  CHECK: ls .omo/evidence/v353-rhwp-table-fidelity/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=[4-9]$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=b667f896cb71ac9829dfe5876a4cf6daf56839c29d6995245ceab462363805fd; exit=0; EXPECT=matched; output-sha256=b137407ae26fe9a944d552420da78510f0121687d7c98e1a4e096c20350b70cd; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=b32f84a3da1f/49 entries


## C1 — correction 검증 범위

- 원 소스 증거: `45d4f16`에서 실행한 기존 CHECK와 `1e87b0e5507ff3204f9729e9de800e8163ae267c`의 원장·`gate-raw-output.txt`. C1은 그 HEAD의 직계 후속이다.
- C1 승인 기준: F1 일반/cell-path 텍스트 편집의 max(표시 높이, 셀 합), F2 실제 열 폭이 같은 resize의 선언 폭 보존, F3 열 분할 뒤 새 간격을 포함한 폭 공식. R1 실패 사례를 `tests/hwpx_table_fidelity.rs`에 옮겨 수정 전 소스로 RED를 확인하고 GREEN으로 닫는다.
- 추가 호출부 감사: 행 삽입·삭제, 열 삽입·삭제, 절대 열 폭 설정, 병합 해제·범위 분할 및 normal/cell-path wrapper의 실제 열 변화 판정을 대조한다.
- 재실행: G2(Table 모듈 영향 확인), G4(테스트 파일 결정적 pin), G5·G6·G8·G9·G10·G11, G12(변경된 행 삭제 경로의 한컴 확인용 head 산출물을 재생성). §V delta 규칙에 따라 나머지는 아래 증거를 carry한다.
- NOTE G1 CARRY: 원 baseline JSON·a34d88ff 소스·fixture 바이트는 C1 diff의 입력이 아니다. 기존 `BASELINE_OK 471 22` 증거를 유지한다.
- NOTE G2: 편집 없는 열기→저장 경로이나 Table 모듈 변경의 영향을 직접 확인하도록 재실행한다.
- NOTE G3 CARRY: serializer::hwpx::table 단위 시험과 속성 파싱·쓰기 입력은 C1 diff 밖이다. 기존 29 passed 증거를 유지하며 G8 전체 시험에서도 다시 실행한다.
- NOTE G7 CARRY: a34d88ff 음성 대조의 원 시험 바이트·JSON·로그는 보존한다. C1의 새 시험은 별도 `c1/red-1e87b0e.log`로 검증하며 기존 a34d88ff 전체 음성 대조를 반복하지 않는다.
- NOTE G4: Parent 처분 2 적용 — TABLE·CELL·NONE 두 경로 원 문자열 보존, ROW/ROW_BREAK는 CELL로 정규화한다.
- NOTE G11: Parent 처분 1의 common_obj_attr_writer.rs 변경은 base 대비 정확히 +1/-0으로 유지한다.
- C1 결과: source checkpoint `00cb7458986b089110c153c787c1c7b121db93ea`에서 9개 게이트 재실행, G1·G3·G7 carry로 `ALL MET (12 met)`. 원 소스 시험 묶음은 현재 시험 파일의 바이트 동일 prefix이고, carry 입력 8개는 entry HEAD와 SHA-256이 같다(`c1/carry-inputs.json`).
- CHECK 강화: C1 재실행 명령은 pipefail과 원 로그 tee를 사용한다. G8은 cargo 자체 exit code와 성공 시험 결과를 함께 확인하며 컴파일 실패를 FAILED_TOTAL=0으로 오인하지 않는다.
- NOTE C1 RED/GREEN: 동일 최종 시험 SHA-256 `c67a13b797813604e21fba636d66093f2847862807fe793c65f1f43b3a46e840`으로 entry `1e87b0e` 소스는 21 passed / 8 failed / 1 ignored, source checkpoint는 29 passed / 0 failed / 1 ignored. 초기 픽셀 올림 전제 오류와 미완성 수정 실행은 판정에서 제외하고 로그에 보존했다.
- NOTE C1 G5/G6/G8: 표시 높이 9 passed, 폭 공식 14 passed, 전체 cargo test 1162 passed / 0 failed / 3 ignored(16 묶음). G8 안에서 serializer::hwpx::table 29개도 통과했다(`c1/g5.log`·`g6.log`·`g8.log`·`verification.json`).
- NOTE C1 G9/G10/G12: clippy `-D warnings`·wasm32 release rc=0; 한컴 head 산출물 6개 재생성 후 총 8개이며 entry의 산출물 바이트와 같다. 한컴 열기·시각 확인은 Parent 소관이다.
- NOTE C1 범위·잔여: 추가 수정은 delete_row의 실제 열 폭 변화 판정과 set_column_widths의 실제 셀 폭 변경 판정이며, 행 삽입·열 삽입/삭제·행 방향 분할·wrapper도 감사했다. code/test source는 이후 증거·보고서 커밋에서 바꾸지 않는다. 자기 리뷰 receipt는 만들지 않았고 별도 R2 리뷰가 남는다.

## C2 — correction 검증 범위

- Entry: `e6e5850ee07c952f2908df8e5845124c224c2f90`; 최종 source checkpoint: `9ac673ce16034fc03d819e5186485a920e5d2c16` (중간 `7668ec27182792350b7aa8d43b52c0e14320919e`). 제품 변경은 text_editing.rs의 일반/path 삽입·삭제 네 호출부 및 같은 원인의 셀 선택 영역 삭제 호출부와 회귀 시험 추가뿐이다.
- 승인 기준: 실제 텍스트·문단 구조 변화가 없는 요청은 높이 동기화·dirty 마킹·reflow·상위 재배치를 실행하지 않고 저장 HWPX 바이트가 같다. 실제 편집의 C1 grow-only 동작은 유지한다.
- 재실행: G5·G8·G9·G10·G11 및 시험 파일의 결정적 pin인 G4·G6; 변경된 텍스트 호출부를 쓰는 G12 산출물도 재생성·대조한다. 나머지는 아래 입력 동일 NOTE로 carry한다.
- NOTE C2 G1 CARRY: 고정 a34d88ff baseline JSON·fixture·도구 입력은 e6e5850..9ac673c diff 밖이다. 기존 BASELINE_OK 471 22 및 원장 증거를 유지한다.
- NOTE C2 G2 CARRY: 편집 없는 열기→저장은 수정한 셀 텍스트 명령을 호출하지 않는다. 파서·serializer·Table 모듈·roundtrip 도구·fixture가 e6e5850과 동일하므로 C1 source 00cb745의 T1/T2/T6/T7 ATTR_LOSS_TOTAL=0 증거를 이어 쓴다.
- NOTE C2 G3 CARRY: serializer::hwpx::table 단위 시험 및 속성 파싱·쓰기는 수정한 텍스트 명령을 호출하지 않는다. 입력 바이트는 동일하며 기존 29 passed 증거를 유지하고 G8에서 다시 실행한다.
- NOTE C2 G7 CARRY: 고정 a34d88ff 음성 대조 JSON·negative_control.py·원 시험 prefix는 그대로다. 새 C2 사례는 e6e5850에서 별도 RED를 기록하고 이 head에서 GREEN을 검증한다. 원 a34d88ff 전수 음성 대조는 반복하지 않는다.
- NOTE C2 G5: 17개 이상의 표시 높이 시험 수를 허용하도록 EXPECT의 >=3 정규식을 바로잡았다(기존 [3-9]\d*는 10–29를 잘못 제외했다). 각 명령의 cargo exit code는 pipefail로 보존한다.

- NOTE C2 추가 호출부: `delete_range_native`의 셀 단일 문단 삭제도 실제 삭제 수가 0이면 raw 무효화·높이 동기화·dirty/reflow를 생략한다. public `delete_range_in_cell`에서 셀 0/1 × 길이 0/끝 이후 삭제의 HWPX 바이트 동일성을 추가 검증한다.
- NOTE C2 검증 교정: 중간 source 7668ec2의 8개 게이트 실행은 `c2/intermediate-7668ec2/`에 보존하며 최종 source 9ac673c의 증거로 사용하지 않는다. 공유 target을 사용한 별도 원본 RED의 예상 밖 GREEN도 `red-final-shared-target-discarded.log`로 제외한다. 원본 src 353파일을 Git과 대조한 독립 target의 동일 최종 시험은 e6e5850에서 9/9 실패했다. 최종 소스는 rhwp 패키지 산출물을 비운 뒤 검증한다.
- NOTE C2 최종 결과: source 9ac673c에서 G4 4 passed, G5 18 passed, G6 14 passed; 전체 cargo test 1171 passed / 0 failed / 3 ignored(16 묶음), 표 충실도 38 passed / 0 failed / 1 ignored, serializer::hwpx::table 29개 통과. clippy -D warnings·wasm32 release rc=0. 8개 재실행 + 4개 입력 동일 carry로 ALL MET (12 met), unmet/abandoned/deferred 0이다.
- NOTE C2 증거 결속: 최종 시험 SHA-256 `0bc0cf833692199a509d985fbf720fca1e9bdfa03134beec52f2012a613ffac3`이 RED/GREEN 사본과 같다. 두 source의 src 353파일을 각 Git blob과 대조했다. C1 39011-byte prefix·원 27484-byte prefix 및 carry 입력 19개가 같다. 재실행 8개 EVIDENCE의 출력 SHA-256·bytes를 c2/g<n>.log와 직접 대조했다(`c2/verification.json`, `carry-inputs.json`, `*-source-binding.json`).
- NOTE C2 G12: 최종 소스에서 한컴 head 파일 6개를 다시 생성했고 총 8파일의 SHA-256은 entry와 같다(`c2/hancom-regenerated.log`, `hancom-before.json`, `hancom-after.json`). 한컴 앱 열기·시각 확인은 Parent 소관이다.
- NOTE C2 자체 점검: e6e5850..9ac673c diff를 현재 Codex 세션에서 직접 검토했다. 유효한 셀/문단 검증 뒤에 no-op guard를 두어 성공 응답의 offset과 오류 처리를 유지하며, 실제 삭제 수로 판단하고 실제 텍스트 편집·문단 분할/병합의 기존 reflow는 유지한다. 최종 재독에서 추가 substantive finding은 없으며 독립 보증이나 self-review receipt로 취급하지 않는다. 별도 R3와 Parent 착지 감사가 남는다.


## C3 — G0 base remeasurement

- Base is now `01a67b3f77760b5a9dae9fc155d83f8ca84ab529`; entry old branch is preserved at `0052deae929c53830d2ff573c9c8ec817a4693b6`.
- All twelve CHECKs are retained and rerun: no C1/C2 carry is used for the changed base. C1/C2 notes above are historical evidence only.
- NOTE G1/G7/G11: the base literals and G1/G7 artifact paths now identify the independently measured G0 base, not the old a34d88ff input. EXPECTs remain unchanged unless measured semantics require an explicit note.
- NOTE G2/G3/G4/G5/G6/G8/G9/G10: log paths move to c3 to preserve earlier rounds; G3 keeps its original test filter and EXPECT and now uses pipefail/offline/locked like the other cargo checks.
- NOTE G4: disposition 2 still applies (TABLE/CELL/NONE preserve; ROW aliases normalize to CELL).
- NOTE G12: regenerate six head files and two new-base files before running the retained file-count CHECK; Parent owns Hancom app review.
- C3 acceptance and conflict resolutions are in `.omo/evidence/v353-rhwp-table-fidelity/c3/REBASE.md`.

- NOTE G11 C3 DISPOSITION1: G0 시험이 처분 2 와 반대 매핑을 기대해 기대값만 갱신. Parent 승인 msg와 outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_REBASE_C3_V353_DISPOSITION1.md에 따라 tests/hwpx_roundtrip_integration.rs tuple 5행(+5/-5)만 범위에 추가한다. 제품 매핑과 나머지 assertion은 그대로다.
- NOTE C3 source: `dbf294757690e723c614c22e467022522ea3b44e`에서 모든 CHECK를 실행했다. 이후 커밋은 원장·보고서·원 출력·한컴 검토 산출물만 기록하며 code/test 입력은 바꾸지 않는다. 8개 lane commit replay와 11개 G0 ancestor, 기존 브랜치 보존을 `c3/verification.json`으로 대조한다.
- NOTE C3 G1: 새 base의 T1=448, T2=18, 합계=466이다. 원 a34d88ff의 471/22 기록은 이력으로 보존한다. 새 base는 .git 없는 `git archive`의 모든 2817 tracked blob을 대조한 사본이며, baseline의 관측 HEAD null과 명시적 base 결속을 함께 남겼다(`base-source-binding.json`).
- NOTE C3 G2/G3/G4/G5/G6/G7: head의 T1/T2/T6/T7 ATTR_LOSS_TOTAL=0, serializer::hwpx::table 29 passed, page_break 4 passed, display_height 18 passed, width_formula 14 passed. 새 base 음성 대조는 page_break 2 passed/2 failed, display_height 8 passed/10 failed다. G6 추가 대조는 3 passed/9 failed이며 base에 없는 set_column_widths를 쓰는 G6 시험 두 개만 archive에서 cfg(any())로 제외했다. G4/G5 시험은 바꾸지 않았고 제외 시험명·바이트 digest는 negative-control.json에 있다.
- NOTE C3 noAdjust: G0의 public bool을 HWPX 권위로 유지하고 HWP5 import의 raw bit 3을 bool에 투영했다. 동일 최종 회귀 시험은 연결 전 RED 0 passed/1 failed, 연결 후 GREEN 1 passed다(no-adjust-red-valid.log / no-adjust-green.log). 다중 section 기대값을 잘못 만든 초기 red.log·red-final.log 실행은 판정에서 제외했다.
- NOTE C3 G8 DISPOSITION2: `serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves`가 section XML 전체 동일성에서 실패했다. 재저장 때 colPr이 별도 run으로 분리되며 속성/entity 자체 assertion은 통과했다. 수정하지 않은 새 base에서도 같은 XML diff로 실패했고 mod.rs/section.rs는 새 base와 byte 동일하다. Parent `msg_fc62bcbcfa2c` 및 `outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_REBASE_C3_V353_DISPOSITION2.md`에 따라 C3에서 고치지 않으며 위 ABANDON을 남긴다.
- NOTE C3 전체 시험 수: G8이 lib에서 멈춘 뒤 모든 나머지 cargo target과 doctest를 명시 실행했다. lib=1000 passed/1 failed/1 ignored; table fidelity=39/0/1, G0 roundtrip integration=57/0/2, G0 header stability=7/0/0; 나머지 target=130/0/1; doc=0/0/0. 중복 없는 합계는 1233 passed/1 failed/5 ignored(16 묶음)다. 추가 G0 Studio table-resize는 24 passed/0 failed다. ignored/private 입력 부재를 실제 private 문서 검증으로 세지 않는다.
- NOTE C3 G9/G10/G11/G12: clippy -D warnings 및 wasm32 release rc=0; source checkpoint 범위 0 out-of-scope(증거 커밋 뒤 재확인). 한컴 파일 8개를 새 source/base에서 재생성하고 ZIP/XML·제어문자 및 해시를 대조했다. 한컴 앱 검토는 Parent 소관이다.
- NOTE C3 결과: 12 CHECK 모두 실제 실행, 11 met + G8 ABANDON 1, 12/12 통과가 아니다. EXPECT는 바꾸지 않았다. source diff 자체 점검은 현 Codex 세션에서 수행했으며 독립 리뷰나 self-review receipt가 아니다. 다음 lane의 table-front spacing·나머지 drawing/field 수정은 시작하지 않았다.

INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7
AC1: met — 원 a34d88ff baseline 이력 보존, 새 base G1 448/18 및 head G2 0.
AC2: met — 승인된 ROW→CELL 정규화 처분을 적용한 G4 4 passed 및 G0 tuple 정합.
AC3: met — G5/G6 18/14 passed와 새 base G7 2/10 failed.
AC4: unmet — G8 기존 base 시험 실패; G9/G10/G11은 통과, Parent 처분에 따라 ABANDON으로 반환.
jev=used — 결정적 pin/exec 결속을 유지하고 Jev 순위를 함께 확인했다.
