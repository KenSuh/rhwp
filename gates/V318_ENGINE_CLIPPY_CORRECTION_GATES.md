# v318 Engine Clippy correction 검증 원장

상태: **READY_FOR_VAULT_SINGLE_MERGE**. 최초11파일의14진단과 coordinator가 추가 승인한 src/main.rs 1-byte/15번째 진단을 수정했고 실제 Rust1.99 전체 엄격 Clippy가 rc0이다. 로컬 후보 인계이며 parent R2, final5, exact-head CI 또는 착지 승인과 다르다. 원래14 RED 및 중간15번째 rc101 증거는 보존한다.

## 고정 소스와 권한

- 작업 트리: `/Users/ken/orca/workspaces/rhwp/v318-engine-clippy-correction`.
- base: `3033ae22c9031919757be1357a998471e67f1fab`; base tree: `c46c996dfdc57f5f8c75c51a8e49d9b2cd1c2a42`.
- 최초11 수정 소스 checkpoint: `e957b3b87a8bebaf9091d3cf590a388c7a7d1ad7`; parent는 위 base; source tree: `377e08af4758266c2a9f271723e32a7f5f7f87c3`.
- 추가 CLI 수정까지 포함한 소스 checkpoint: `0a9ffb11999565619be38937ad6b369cb1a3818b`; parent `fc3184b9a3fb1ff57a8dd04de6cf55d774fa367e`; source tree `1251cecc0d8ade8a33bca388beda847c8aa5923c`.
- 추가 권한 = coordinator Orca ask 응답 `msg_8518abc7d707` 및 갱신된 parent CLIPPY_CORRECTION_PACKET.md. CLI_SCOPE_AUTHORIZATION.md에 답변을 보존했다. src/main.rs의 & 하나만 제거했고 다른 CLI 코드는 수정하지 않았다.
- 원장/증거를 추가하는 후속 commit은 제품 소스를 바꾸지 않는다. 최종 인계 HEAD/tree는 raw git으로 별도 보고한다.
- matching mah lease: `0f9b97c9eec07c961d503882dd248ac1`, task `v318-engine-clippy-correction`, agent `codex`, role `executor`, scope `repo`.
- 실제 모델/effort: **gpt-6.1-sol / xhigh**. 추천값 max와 다르며 사용자 승인 fallback이다. `dispatch-identity.log`의 terminal.preview가 실제 xhigh를 보여준다. xhigh를 max로 주장하지 않는다.
- Orca task `task_0647f7865697`; dispatch `ctx_00b0367d9380`; terminal `term_cb89e300-b7cc-4a69-b1e9-09259ba1e277`.

## 정확한 제품 diff

최종12 files, 40 insertions, 43 deletions. 완전한 제품 diff = `.omo/evidence/v318-engine-clippy-correction/final-source-diff.log`, SHA256 `683b4d62cc871ff7c2ba8ea3e782d9fc9319827a184cf523aa97e370556ceff6`. 최초11 diff는 source-diff.log, SHA256 `6baa9451651e2fd20d6cc08bae16de2c3a87e874237926a469f8db6b0c9a076b`로 보존했다.

| 파일 | 변경과 의미 보존 |
| --- | --- |
| src/document_core/commands/object_ops.rs | 0으로 평가되는 OR 항만 제거. table flag 바이트 동일. |
| src/document_core/commands/table_ops.rs | Some(Result.ok())를 Ok(Result)로 바꿈. 성공 시 dirty 설정, 실패 시 기존처럼 건너뜀. 전파/반환 변경 없음. |
| src/document_core/queries/cursor_nav.rs | 동일 타입 tuple을 if 식으로 초기화. 세 분기의 순서, 호출 인수, `?`, fallback 동일. |
| src/document_core/queries/form_query.rs | UTF-16LE의 완전한 2-byte pair만 순서대로 디코딩. 홀수 꼬리 byte를 기존처럼 무시. 압축/실패 정책 동일. |
| src/document_core/queries/rendering.rs | dirty_sections.fill(true), para_offset.fill(0). 길이와 빈 slice 동작 동일. |
| src/model/style.rs | 7-language font_ids.fill(id). 이후 개별 font_ids override 순서 동일. |
| src/parser/body_text.rs | 길이 검증 뒤 UTF-16LE field name pair 디코딩만 변경. |
| src/parser/byte_reader.rs | read_exact와 strict String::from_utf16 오류 정책 그대로. |
| src/parser/control.rs | cell field name 및 form UTF-16 decoder의 완전한 LE pair iteration만 변경. |
| src/serializer/hwpx/section.rs | PageDef seam 밖 format 인수의 & 두 개만 제거. format의 자동 borrow로 텍스트/anchor 동일. |
| src/wmf/parser/objects/structure/mod.rs | 완전한 LE pair를 typed array로 읽음. 기존 odd-length 거부와 strict surrogate 오류 정책 그대로. |
| src/main.rs | coordinator 승인 후 println의 & 하나만 제거. 자동 Debug borrow 및 format 문자열 그대로. |

manifest, tracked lockfile, workflow, renderer/pagination, API, publication, PageDef 모델/파서/테스트 입력 변경 0. 처음에는 Cargo.lock이 없고 gitignored였으며 cargo가 재현 빌드의 로컬 산출물로 생성했다. 추적되는 lockfile/dependency 수정은 없다. upstream 및 origin/main ref는 이 새 source root에 없어 원격 비교는 미검증이다.

## Compiler/MSRV와 원래 실패

원래 CI37244217873/job111558788237는 3033에서 Clippy rc101, 14 library errors이다. parent CI_FAILURE_FIRST.json은 FAIL 그대로이며 log SHA256 `7e85dabacd7d5aa5b7abfd223497a607b2e0a4e72f6da16f90cd940ad3868b8e`다. native build/tests와 WASM 성공은 parent의 이전 CI 관측이며 현재 head 성공으로 재명명하지 않는다.

로컬 기본 compiler는 1.95.0이었다. task 전용 `RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380`에 1.99.0을 설치했다. 실제 rustc: `1.99.0 (b940084d7 2026-09-28)`, host `aarch64-apple-darwin`, LLVM23.1.1. Ubuntu CI와 플랫폼이 다르므로 최종 exact-head CI가 남는다.

Cargo package rust_version는 null이고 CI는 stable을 사용한다. 현재 manifest의 zip ^8.5가 로컬에서 8.6.0으로 해석되며 그 rust_version는 1.88이다(`metadata-online.log`). README의 1.75+ 표기는 기존 의존성과 불일치한다. 이를 고치거나 package rust-version을 올리지 않았다. 선택한 safe [slice::as_chunks API](https://doc.rust-lang.org/std/primitive.slice.html#method.as_chunks)는 Rust1.88부터 안정화되어 현재 manifest의 기존 의존 요구보다 높지 않다. slice.fill은 기존 요구 아래의 API다.

## Gates

- [x] G1: base identity, owned11+승인CLI1 source diff, PageDef seam/입력 불변
  CHECK: node -e 'const fs=require("fs"),a=require("assert/strict"),cp=require("child_process"),d=".omo/evidence/v318-engine-clippy-correction"; const id=JSON.parse(fs.readFileSync(d+"/source-identity.json")); a.equal(cp.execFileSync("git",["rev-parse",id.base],{encoding:"utf8"}).trim(),id.base); a.deepEqual(cp.execFileSync("git",["diff","--name-only",id.base,"HEAD","--","src"],{encoding:"utf8"}).trim().split("\n").sort(),id.owned_rust.slice().sort()); console.log("PASS owned12")'
  EXPECT: PASS owned12
  EVIDENCE: source-input-check.log는 최초11에서 rc0,SHA256 a3a9217c05147258b69bee3b1a58ccecbf4e1039762761bfab59c4346491ab48. 최종12 source-identity.json과 final-input-check.log rc0/SHA256 197f8c614dd1cdba3dc1f1317b0c229390599a4594da0742e7c7495a36c121e7로 실제 Clippy 입력을 소스HEAD와 대조했다. library-carry-check.log rc0, fc3184b9..0a9ffb11에서 src/main.rs 외 library/test/manifest/workflow 차이0.

- [x] G2: 실제 Rust1.99 baseline RED를 정직하게 기록
  CHECK: node -e 'const fs=require("fs"),a=require("assert/strict"),d=".omo/evidence/v318-engine-clippy-correction"; const r=JSON.parse(fs.readFileSync(d+"/baseline-clippy.json")); a.equal(r.rc,101); a.equal(r.head,"3033ae22c9031919757be1357a998471e67f1fab"); a.equal(fs.readFileSync(r.log,"utf8").match(/^error:/gm).length,15); console.log("BASELINE_RED: 14 diagnostics plus compile summary")'
  EXPECT: BASELINE_RED
  EVIDENCE: baseline-clippy.log actual cargo +1.99.0 clippy -- -D warnings rc101, exact14 reported paths/lines, SHA256 2d37421af33401109b94e14f178aea8c4e816b7fcc0740fc4b107d9e4820aabe. G2 성공은 실패 재현 사실의 검증이며 제품 green 주장이 아니다.

- [x] G3: 전체 cargo clippy -- -D warnings GREEN
  CHECK: env RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380 cargo +1.99.0 clippy -- -D warnings
  EXPECT: Finished
  EVIDENCE: strict-clippy-final.log 실제 동일 full cargo +1.99.0 clippy -- -D warnings rc0,SHA256 745bd9aca75097f35b503e3273dcf3701d54199eff4e903f1f500331c027aca6. 이전 candidate-clippy.log rc101/SHA256 0e87a2e4ad81dfa524ddabef49de6f80233842f33edda90bb3c09c052cf9ce4f는 그대로 보존. 14 library errors가 먼저 main의15번째 진단을 가렸고 coordinator 승인 뒤 & 하나만 고쳐 해결했다.

- [x] G4: scoped library strict Clippy, owned 파일 format, diff 구조
  CHECK: env RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380 cargo +1.99.0 clippy --lib -- -D warnings
  EXPECT: Finished
  EVIDENCE: scoped-clippy.log rc0 SHA256 f7bd0d2105c42a805e8bed56ed58c4f2984f020b9fd6a009b5394c2a88bbd6d5; rustfmt-final.log rc0, diff-check.log rc0. --lib 결과가 G3 전체 결과를 대신하지 않는다. 초기 format rc1은 rustfmt.log에 보존하고 해당 두 owned hunks만 수정했다.

- [x] G5: 바뀐 parser/serializer 및 PageDef13 기존 테스트
  CHECK: env RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380 cargo +1.99.0 test --test hwpx_page_def_roundtrip
  EXPECT: 13 passed; 0 failed
  EVIDENCE: pagedef13.log rc0,13/13,SHA256 7bc03dcc5fd25c665af2f0319d352f3bdbc5615cca9d4dd2193344737deabb16; focused-parser.log rc0,157/157,SHA256 e720af335530aaa6df964f0a77e6e7bed7f4699a3c94853d5148a8aad2018efb; focused-hwpx.log rc0,63/63,SHA256 46395d11d8c6bdebd12e888bc3ca36782faefe1682b50ff84af9bc1853a7caa7. Parser filter는 equation/parser 및 WMF parser도 포함한다.

- [x] G6: table/dirty/command/cursor/style 기존 네이티브 API 검증
  CHECK: env RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380 cargo +1.99.0 test --lib wasm_api::tests::test_task105_nested_table_path_api -- --exact
  EXPECT: 1 passed; 0 failed
  EVIDENCE: nested-table, inline-table, command-batch, cursor-line-break 각각 rc0,1/1; model-style rc0,3/3. 모든 .json에 실제 argv/cwd/rc/source-input SHA256 및 raw log SHA256이 있다. move_vertical_native의 세 분기는 tuple 초기화 변환을 diff로 검토했고 직접 실행하는 기존 테스트는 발견하지 못했다. cursor rect 테스트를 세 분기 직접 coverage로 주장하지 않는다.

- [x] G7: UTF-16 pair 교체의 경계와 오류 의미
  CHECK: env RUSTUP_HOME=/tmp/rhwp-v318-clippy-rustup-ctx_00b0367d9380 node .omo/evidence/v318-engine-clippy-correction/semantics.mjs
  EXPECT: PASS: 131075
  EVIDENCE: semantics.log rc0,SHA256 992f59ee85c4cae8e10ab72b8f5bee94fb0ad19e77df0bea905794ca2a26f201. base/current 실제 private 함수 4개를 추출해 131075 inputs 비교하고 별도 고정값 oracle로 Korean LE, surrogate pair, lone surrogate, empty/odd WMF 오류를 확인. WMF ParseError enum은 standalone compile용 최소 stub이며 실제 WMF 통합 테스트를 대체하지 않는다. 압축 script와 ByteReader read_exact는 기존 코드/테스트 및 동일 chunk 변환의 검토 증거다.

- [x] G8: raw 증거 해시와 현재 세션 자체 diff 검토
  CHECK: git diff --check 3033ae22c9031919757be1357a998471e67f1fab HEAD -- src gates .omo/evidence/v318-engine-clippy-correction/SELF_CHECK.md .omo/evidence/v318-engine-clippy-correction/run.mjs .omo/evidence/v318-engine-clippy-correction/semantics.mjs
  EVIDENCE: record-integrity.log rc0,최초34 raw command hashes 확인; final-integrity.log rc0,49 hashes/SHA256 e7b0dab8df38713193ad3dc723a1ac044049242042c4bf30589183e36b986117. final-gate-status.log rc0,8 met/1 deferred/SHA256 aa2a78d9140d2556f759bd117e6bda00a78ff28d6da77d7a554890821582d141. 현재 세션 findings-first 자체 검토: open substantive blocker0, 승인된 main 진단1 closed. 코드/테스트 삭제, allows/flags 완화, byte order/여백/방향/제책 변경 없음. 같은 Codex 검토이므로 독립 보증 아님; mah self-review/review-result receipt 발행0. parent Codex R2/final5 NOT_RUN.

NOTE: 원시 compiler/Jev/git 출력의 공백을 보존했으므로 전체 staged git diff --check는 rc2다(staged-diff-check.log, SHA256 ebb094aaf7152dabb4295cb0bd6a278ef8b817c5970da946ecc35786db93a817). 해당 실패는 raw .log의 trailing whitespace/EOF blank에만 해당하며 소스/원장/스크립트 검사는 source-doc-diff-check.log rc0다. 원시 출력 byte/hash를 바꾸거나 전체 rc2를 성공으로 재명명하지 않는다.

NOTE: CLI warm build rc0(cli-build.log), src/main.rs rustfmt rc0(cli-rustfmt.log), 실제 native dump 전후329732 bytes/SHA256 55575c736ec6265b3979401d780251e27830734ade653be45d7aac93df85acb2 동일(cli-before/after.log, cli-output-parity.log rc0). 이 대표 HWPX fixture는 head/raw_extra 줄이 없으므로 해당 println 분기 직접 coverage라고 주장하지 않으며 단일 Debug borrow는 전체 diff/컴파일 검토로 동등성을 확인했다.

NOTE: 승인CLI1 후 G4..G7의 library/PageDef/semantic 테스트 입력은 library-carry-check로 동일함을 확인해 이전 실제 증거를 이어 쓴다. 전체240 테스트/131075 입력 matrix를 다시 돌리지 않았다. 최종 strict Clippy와 CLI cheap checks만 실행했다.

- [ ] G9: 최종 exact-head native/full suite/WASM/전체 workflow
  CHECK: false
  EVIDENCE: NOT_RUN; parent가 R2 수렴 뒤 최종 candidate exact-head CI로 한 번 수행한다.
ABANDON: G9 DEFERRED-TO-LANDING — executor에게 full/native cold-start/WASM/fullworkflow 반복 실행 권한 없음; parent exact-head CI 담당. 병합 전 실제 증거로 채우고 이 줄을 제거해야 한다.

## 남은 범위와 기존 문제

publication/merge/push/PR/Actions/provider/DB/deploy/host-browser/full-fidelity 검증은 NOT_RUN. font/table/line placement/pagecount/hostWASM/한컴 앱 전체 fidelity는 승인 범위 밖이다. 테스트 컴파일의 기존 경고 4개는 serializer/hwpx/field.rs:192, wasm_api/tests.rs:1173/11958/16440에 남으며 보고했다. 경고를 숨기거나 해당 파일을 고치지 않았다.

원래3033의 PARENT_REVIEW_R2.md, PUBLICATION_PREFLIGHT.md 및 final5는 원래 source identity를 유지한다. PageDef parser/test 및 PageDef render/replace 함수 입력 불변은 source-input-check로 확인했고 현재13 테스트는 새로 돌렸다. 원래 corpus/native/바이트 출력 proof를 현재 후보의 재실행으로 표시하지 않는다. 이 correction의 두 format borrow는 PageDef 기능 밖이며 전체 diff로 확인했다.

jev=used: owned11/main/manifest/workflow 후보는 Jev cache/실제 ranking 사용. 결정적 pin/exec 후보는 점수 아래라도 제외하지 않았다(impact-1..11.log --top0). binding=exec의 wasm_api/tests 및 schema 후보도 검토 범위로 유지하며 관련 parser/serializer/nested-table 실행을 기록했다. 전체 wasm_api suite는 G9 최종 CI로 넘긴다. 처음 새 gate 경로는 jev=unused(후보0), evidence를 commit한 후 HEAD에서는 gate 후보 ranking도 jev=used다. 보호된 prior failure/갱신packet은 impact 실행 후 읽었으며 고치지 않았다.
