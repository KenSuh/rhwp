# Gates: v353 rhwp table fidelity P0–P2

- [x] G1: 변경 전 baseline — T1·T2 왕복 손실이 0 이 아님을 수정 전 커밋(a34d88ff)에서 기록했다
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/baseline.json"));assert d["head"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["T1"]["ATTR_LOSS_TOTAL"]>0 and d["T2"]["ATTR_LOSS_TOTAL"]>0;print("BASELINE_OK",d["T1"]["ATTR_LOSS_TOTAL"],d["T2"]["ATTR_LOSS_TOTAL"])'
  EXPECT: /^BASELINE_OK \d+ \d+$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=3ae476367c9e6811816b0419fdb42e0b37d9c34d23f50b77408537fcd3f1a0c3; exit=0; EXPECT=matched; output-sha256=1ff7980dac04ff5cf41f0421135c6429538eee84bd24d33810813b6ed1c579fe; output-bytes=19; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G2: 속성 보존 — 수정 head 에서 T1·T2·T6·T7 왕복 손실 합계 0(lineWrap·noAdjust·lock·protect·pos.*·sz.*·textFlow·pageBreak)
  CHECK: bash -o pipefail -c 'python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g2.log'
  EXPECT: /^ATTR_LOSS_TOTAL=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=4a85d17a27a0fbbbf490693c82cb512ed6fe80c75481af6462d128daf36224f5; exit=0; EXPECT=matched; output-sha256=78f9e9eaa187df0c7211d424521d66efd5fc1d6f52b2f306007cc0326c4860f4; output-bytes=977; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G3: 속성 보존 단위 시험(noAdjust·lock·sz protect·allowOverlap·flowWithText·textFlow·셀 lineWrap SQUEEZE·셀 protect 각 1 사례 이상)
  CHECK: cargo test --lib serializer::hwpx::table 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5711150d7db779b9dcf20355e8924c632466dc433a87bddb07e0d76e65d8bb3b; exit=0; EXPECT=matched; output-sha256=85c6d3c82d9f49573a425aeec71fb9b7665de1e907e3055177cc1deba7d839bf; output-bytes=99; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G4: pageBreak 4값 — HWPX→IR→HWPX 와 HWPX→HWP5→HWPX 두 경로 모두 원 문자열 보존
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity page_break 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g4.log'
  EXPECT: /test result: ok\. [2-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=1c2c76e8c7e8b9e9fe1d939daa65b744c400dd1cda316c33cb694ff33af42738; exit=0; EXPECT=matched; output-sha256=3d65811e1ca9d8bb41c211bcbf50b1fa3b426440a3bfe367dfe3aa505881f1af; output-bytes=505; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G5: 표시 높이 — T6(표시 높이 ≠ 셀 합) 행 삽입·삭제·텍스트 편집 뒤 sz.height 가 §3.2 공식대로 유지
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity display_height 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g5.log'
  EXPECT: /test result: ok\. [3-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=a38e0846bc1cd608ffafe1bf2503e62c94815079e4dfb8133801f73cbb4fcdab; exit=0; EXPECT=matched; output-sha256=b5075a94350116efffb241084ab2c49827280f0be7243bea668b7665e3514637; output-bytes=905; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G6: 폭 공식 — cellSpacing>0 표와 병합 없는 표에서 sz.width == Σ열 폭 + (colCnt+1)×cellSpacing
  CHECK: bash -o pipefail -c 'cargo test --offline --locked --test hwpx_table_fidelity width_formula 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g6.log'
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=a0d77357828106de166cf4ab2f7efcc2e66dcf64ff04f90cba73ca8159310dde; exit=0; EXPECT=matched; output-sha256=62f1a81c544bbe0369cb306668b4a1a8cf7fdd5b53f9e413fc365ec2e565dccf; output-bytes=1272; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G7: 음성 대조 — G4·G5 시험을 수정 전 a34d88ff 의 해당 함수에 적용하면 실패한다(시험이 결함을 실제로 잡는다)
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/negative-control.json"));assert d["base"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["page_break_failed"]>0 and d["display_height_failed"]>0;print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=c3139c1e143258d683dcf84c36e3f8a0b689a42043addbdfccbe376bd898980b; exit=0; EXPECT=matched; output-sha256=b5568d8ad4ed8d6a9fab8348e510711f8358c218705c2d8f502ebf2bbb470a0f; output-bytes=7; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G8: 전체 시험
  CHECK: bash -o pipefail -c 'cargo test --offline --locked 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g8.log'
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=85b3df787e0ac7925589e198a34a6bcd308e939afba29875b1c223653ad96780; exit=0; EXPECT=matched; output-sha256=d834d889b4ef5abe7cbe25d4351f315cdcedeef04d68c179507e7363136caff0; output-bytes=84107; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G9: clippy
  CHECK: bash -o pipefail -c 'cargo clippy --offline --locked -- -D warnings 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g9.log'
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=d954e5158a6dd24d57dd3f614070b65c001997ecbfc46ef240a01f5f2d5b94a7; exit=0; EXPECT=matched; output-sha256=964b6a1838b6d5ae4df8cda9724fd6509580a9f0b1abbfab47b1b6c24cd06516; output-bytes=156; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G10: WASM 빌드(에디터가 쓰는 대상)
  CHECK: bash -o pipefail -c 'cargo build --offline --locked --lib --target wasm32-unknown-unknown --release 2>&1 | tee .omo/evidence/v353-rhwp-table-fidelity/c1/g10.log'
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=f608bbaa871682ac76593a886807221ab743e58dce42dd2f4677b7a0b60b502d; exit=0; EXPECT=matched; output-sha256=2c23fba7fa6d1c93d89e523fd59f7e8c7dbd77818f85df8ea78e6d4bb6e82ad8; output-bytes=147; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G11: 범위 — base 대비 변경 파일이 전부 §3.1 MUST 안
  CHECK: git diff --name-only a34d88ff3bae317ad08d8f0b99504e4395278e6a HEAD | grep -vE '^(src/model/table\.rs|src/model/table/tests\.rs|src/model/shape\.rs|src/parser/hwpx/section\.rs|src/parser/control\.rs|src/serializer/hwpx/table\.rs|src/serializer/control\.rs|src/document_core/commands/(table_ops|text_editing)\.rs|src/document_core/converters/common_obj_attr_writer\.rs|tools/roundtrip_diff\.py|tests/hwpx_table_fidelity\.rs|tests/fixtures/table-fidelity/.*|gates/V353_RHWP_TABLE_FIDELITY_GATES\.md|\.omo/evidence/v353-rhwp-table-fidelity/.*|\.gitignore)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=12d010e03d44ece5dce140b16c10c23fb74c8be48aa7b9c6b3c4c0f0aa968de1; exit=0; EXPECT=matched; output-sha256=13a10730bd1c382bb5fe6ac3f0e5a982453970f00ca15e1b5fda9516047c9824; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries

- [x] G12: 한컴 확인용 산출물 — T1·T6 의 편집 전·후 HWPX 를 .omo/evidence/v353-rhwp-table-fidelity/hancom/ 에 저장(한컴 열기·시각 확인은 Parent 가 한다)
  CHECK: ls .omo/evidence/v353-rhwp-table-fidelity/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=[4-9]$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=b667f896cb71ac9829dfe5876a4cf6daf56839c29d6995245ceab462363805fd; exit=0; EXPECT=matched; output-sha256=b137407ae26fe9a944d552420da78510f0121687d7c98e1a4e096c20350b70cd; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=928d1e4c5007/49 entries


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
