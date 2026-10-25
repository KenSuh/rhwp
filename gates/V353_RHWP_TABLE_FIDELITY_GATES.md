# Gates: v353 rhwp table fidelity P0–P2

- [x] G1: 변경 전 baseline — T1·T2 왕복 손실이 0 이 아님을 수정 전 커밋(a34d88ff)에서 기록했다
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/baseline.json"));assert d["head"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["T1"]["ATTR_LOSS_TOTAL"]>0 and d["T2"]["ATTR_LOSS_TOTAL"]>0;print("BASELINE_OK",d["T1"]["ATTR_LOSS_TOTAL"],d["T2"]["ATTR_LOSS_TOTAL"])'
  EXPECT: /^BASELINE_OK \d+ \d+$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=3ae476367c9e6811816b0419fdb42e0b37d9c34d23f50b77408537fcd3f1a0c3; exit=0; EXPECT=matched; output-sha256=1ff7980dac04ff5cf41f0421135c6429538eee84bd24d33810813b6ed1c579fe; output-bytes=19; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G2: 속성 보존 — 수정 head 에서 T1·T2·T6·T7 왕복 손실 합계 0(lineWrap·noAdjust·lock·protect·pos.*·sz.*·textFlow·pageBreak)
  CHECK: python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary
  EXPECT: /^ATTR_LOSS_TOTAL=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=99eee8a5f69b4bc568c56db737b508a34cb233459b46fa8ab0115016c77c9709; exit=0; EXPECT=matched; output-sha256=78f9e9eaa187df0c7211d424521d66efd5fc1d6f52b2f306007cc0326c4860f4; output-bytes=977; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G3: 속성 보존 단위 시험(noAdjust·lock·sz protect·allowOverlap·flowWithText·textFlow·셀 lineWrap SQUEEZE·셀 protect 각 1 사례 이상)
  CHECK: cargo test --lib serializer::hwpx::table 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5711150d7db779b9dcf20355e8924c632466dc433a87bddb07e0d76e65d8bb3b; exit=0; EXPECT=matched; output-sha256=85c6d3c82d9f49573a425aeec71fb9b7665de1e907e3055177cc1deba7d839bf; output-bytes=99; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G4: pageBreak 4값 — HWPX→IR→HWPX 와 HWPX→HWP5→HWPX 두 경로 모두 원 문자열 보존
  CHECK: cargo test --test hwpx_table_fidelity page_break 2>&1 | tail -3
  EXPECT: /test result: ok\. [2-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=f28ebdc4c82399b35a626445c955ecedb3d018552051d15ebab29ad8f45ac99e; exit=0; EXPECT=matched; output-sha256=9682a397cf38c0e7d03893da486c157122653e6e2fd64d1435c2611ed0c2b8e8; output-bytes=97; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G5: 표시 높이 — T6(표시 높이 ≠ 셀 합) 행 삽입·삭제·텍스트 편집 뒤 sz.height 가 §3.2 공식대로 유지
  CHECK: cargo test --test hwpx_table_fidelity display_height 2>&1 | tail -3
  EXPECT: /test result: ok\. [3-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=a1d733c00db0659c3f6ea0dd18f1cd0b2a68f16904481ebf4f5bd9cf69c1c757; exit=0; EXPECT=matched; output-sha256=d44086389761361ea3fde061cb68805a5a0b6d12c73709aa0da26152a0cf3349; output-bytes=97; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G6: 폭 공식 — cellSpacing>0 표와 병합 없는 표에서 sz.width == Σ열 폭 + (colCnt+1)×cellSpacing
  CHECK: cargo test --test hwpx_table_fidelity width_formula 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=f119aa1a936739ea8134f8523155dd94d5635fc6a0b6162f0cff45462094281c; exit=0; EXPECT=matched; output-sha256=be62f3927275a76de02e8d1274602d6e05f9df7d2e48a28be355ccd924bec785; output-bytes=97; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G7: 음성 대조 — G4·G5 시험을 수정 전 a34d88ff 의 해당 함수에 적용하면 실패한다(시험이 결함을 실제로 잡는다)
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/negative-control.json"));assert d["base"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["page_break_failed"]>0 and d["display_height_failed"]>0;print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=c3139c1e143258d683dcf84c36e3f8a0b689a42043addbdfccbe376bd898980b; exit=0; EXPECT=matched; output-sha256=b5568d8ad4ed8d6a9fab8348e510711f8358c218705c2d8f502ebf2bbb470a0f; output-bytes=7; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G8: 전체 시험
  CHECK: cargo test 2>&1 | grep -E "^test result:" | awk '{f+=$6} END {print "FAILED_TOTAL=" f+0}'
  EXPECT: /^FAILED_TOTAL=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=63996319e8a0c984e16d0aefd51b6a046f357406cb8f3e20150600fec617de00; exit=0; EXPECT=matched; output-sha256=34acd69d8c8337c7c993bc0a99093bb0f4dfc879c5cea9515b73e5957c9ccf55; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G9: clippy
  CHECK: cargo clippy -- -D warnings 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=3e5152971bf36fdc9ab776f4b17d0cafba7ae0c8abd875897bc61c40f185c672; exit=0; EXPECT=matched; output-sha256=299f16a2c523be18fef769975d2df00e0505521db047540feea3a74b94666887; output-bytes=72; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G10: WASM 빌드(에디터가 쓰는 대상)
  CHECK: cargo build --lib --target wasm32-unknown-unknown --release 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=8899a6f30c11d7ccf6aa324d8401238a5f8b4d29522bff113e76443728fa1769; exit=0; EXPECT=matched; output-sha256=4d2e7a2a9be886d66add9c0099d84403f887151ea75302f11f731d908da44705; output-bytes=63; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G11: 범위 — base 대비 변경 파일이 전부 §3.1 MUST 안
  CHECK: git diff --name-only a34d88ff3bae317ad08d8f0b99504e4395278e6a HEAD | grep -vE '^(src/model/table\.rs|src/model/table/tests\.rs|src/model/shape\.rs|src/parser/hwpx/section\.rs|src/parser/control\.rs|src/serializer/hwpx/table\.rs|src/serializer/control\.rs|src/document_core/commands/(table_ops|text_editing)\.rs|src/document_core/converters/common_obj_attr_writer\.rs|tools/roundtrip_diff\.py|tests/hwpx_table_fidelity\.rs|tests/fixtures/table-fidelity/.*|gates/V353_RHWP_TABLE_FIDELITY_GATES\.md|\.omo/evidence/v353-rhwp-table-fidelity/.*|\.gitignore)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=12d010e03d44ece5dce140b16c10c23fb74c8be48aa7b9c6b3c4c0f0aa968de1; exit=0; EXPECT=matched; output-sha256=13a10730bd1c382bb5fe6ac3f0e5a982453970f00ca15e1b5fda9516047c9824; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G12: 한컴 확인용 산출물 — T1·T6 의 편집 전·후 HWPX 를 .omo/evidence/v353-rhwp-table-fidelity/hancom/ 에 저장(한컴 열기·시각 확인은 Parent 가 한다)
  CHECK: ls .omo/evidence/v353-rhwp-table-fidelity/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=[4-9]$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=b667f896cb71ac9829dfe5876a4cf6daf56839c29d6995245ceab462363805fd; exit=0; EXPECT=matched; output-sha256=b137407ae26fe9a944d552420da78510f0121687d7c98e1a4e096c20350b70cd; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries
