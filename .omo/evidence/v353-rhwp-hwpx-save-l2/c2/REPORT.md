# rhwp L2 C2 — clean-checkout helper 수정

R2-1/P2의 무시된 `Cargo.lock` 필수 복사 문제를 고쳤다. 잠금 파일이 없으면 Cargo가 의존성을 해석하고, 있으면 기존 파일을 사본에 복사한다. 입력 종류·입력 SHA256·실행 뒤 잠금 파일 SHA256·Cargo 종료 코드를 기록한다. 제품 소스는 바꾸지 않았다. R3와 최종 G7–G9는 아직 남아 있다.

## 신원과 진입

- 저장소: `/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity`
- 브랜치: `v353-rhwp-table-fidelity-g0`
- C2 base 및 검증한 제품 HEAD: `7370ec9fdcf89eb4a372143bcb555ddd9bfe0b46`
- 원래 레인 base: `59c2285efe616211cd147dfa849d2f9f9df767dd`
- 로컬 checkpoint는 이 보고서를 포함하는 커밋이다. 전체 SHA는 `git log -1 --format=%H -- .omo/evidence/v353-rhwp-hwpx-save-l2/c2/REPORT.md`와 완료 handoff로 식별한다.
- reservation은 `active`, `claimed_by=v353-rhwp-hwpx-save-l2`였다. 이전 dispatch 값은 이력이며 task 키로 대사했다. 정확한 필수 시작 HEAD와 clean 상태를 확인했고, 기존 수정 브랜치를 이어받았다. 패킷의 initial HEAD로 되돌리지 않았다.
- `mah doctor/status/recommend`를 실행했다. 겹치는 rhwp lease는 없었다. `preflight.sh "$PWD" --offline --base main`은 behind=0, clean, 최근 한 시간의 알려진 C1 checkpoint 경고를 냈다. 별도 30분 창에 평행 commit은 없었다. 처음 `--base origin/main`은 잘못된 `origin/origin/main` 경고를 내서 올바른 인수로 다시 실행했다.
- worker는 fetch하지 않았다. `git ls-remote origin refs/heads/main`과 로컬 `origin/main`이 모두 `dd1600342c68ef920a6211f783d92a7d5fb53252`였고, `origin/main...HEAD`는 0/45였다. 따라서 기존 수정 브랜치는 관측한 main의 후손이다. upstream 설정은 없다.
- 지시된 기존 task 키 `v353-rhwp-hwpx-save-l2`로 executor/repo lease를 잡으면서 intent id/revision/digest를 같은 명령에 명시했다. `intent-current.log:1`의 `status=current`를 확인했다. intent ID와 lease task 키를 혼동해 두 번째 충돌 lease를 만들지 않았다.
- 전체 R2 보고서와 clean/lockless clone·checkout·status·직접 Cargo·helper 실패/성공/오류 로그를 읽었다. 원 패킷과 처분 1/2 및 C1 처분 1도 적용했다.

## 변경 경로와 동작

1. `c1/clean-cargo.sh`: 선택적 lock 입력, 입력/결과 SHA256, 실제 target 경로와 Cargo 종료 코드 기록. `git archive | tar`의 pipefail, EXIT 정리, Cargo 상태 전달을 유지한다.
2. `gates/V353_RHWP_HWPX_SAVE_L2_GATES.md`: G7–G9를 미체크/명시적 deferral로 돌리고 기존 EVIDENCE는 역사적 NOTE로 보존했다. 로그 목적지를 `c1`에서 `c2`로 옮겼다. G8/G9는 끝의 의존성 메타데이터 때문에 `tail -1` 대신 Cargo의 `Finished` 행을 선택한다. Cargo 명령과 11개 EXPECT는 그대로다.
3. `c2/**`: probe/구조 확인 스크립트, archive 실패 주입용 좁은 git 대역, 명령별 로그와 이 보고서만 추가했다. 아래 명령의 `--name-only`와 Source clean 확인은 checkpoint 뒤에도 다시 수행한다.

제품·시험·fixture·manifest·이전 증거·Hancom 파일·V318 원장 변경은 0이다. 추적되는 잠금 파일이나 의존성 제약 변경은 없다. `structure.log:2`의 제품 입력 Git tree 목록 digest는 `e37e82934c69b185df54c38abf1028a4872da80b2ca77c9d445b87f94072495a`다. 수정 helper SHA256은 `2b0e9279535d7656d764db8a7c13619eea7bab200a61b40b06ab32a5811ce1c6`, 기존 L2 시험 SHA256은 `31b58bbaf1a8d7a1426c192aa1113190d801c350124dc4ac89249aef517bd066`다.

## 직접 실행한 RED/GREEN

순서: `bash c2/probe.sh red` → helper 수정 → `bash c2/probe.sh green` → `bash c2/probe.sh focused` → `bash c2/structure.sh` (경로 접두사는 `.omo/evidence/v353-rhwp-hwpx-save-l2/`). 각 단계는 기대 상태와 정리를 assert하고 exit=0으로 끝났다.

| 검사 | 실제 결과 | 증거 |
| --- | --- | --- |
| RED: pristine exact-base clone의 옛 helper `--version` | `cp: .../Cargo.lock: No such file or directory`, Cargo 전 exit=1; target 생성 없음 | `red-clone.log`, `red-checkout.log`, `red-status.log`, `red-no-lock.log:3` |
| GREEN: 동등한 두 번째 pristine clone에서 수정 helper `--version` | lock 입력 없음, Cargo 도달, exit=0 | `green-clone.log`, `green-initial-status.log`, `green-no-lock.log:5` |
| 실제 Cargo 오류 | 직접 Cargo와 helper 모두 `--c2-invalid-option`에서 exit=1 | `direct-command-error.log`, `green-command-error.log` |
| 선택적 기존 lock | 복사 입력/실행 뒤 SHA256 동일, 원본 복사본도 불변; 검사 뒤 probe 소유 lock 제거 | `green-optional-lock.log` |
| archive 실패 | 정상 빈 tar 스트림을 출력한 git 대역이 73으로 실패; pipefail이 exit=73 전달, Cargo 전 중단 | `fault-bin/git`, `green-archive-error.log` |
| 실제 L2 집중 시험 | `residual_c1_tab_halving_path_survives_two_saves`: 1 passed / 0 failed / 0 ignored, exit=0 | `focused-no-lock.log:161` |
| 값싼 구조 검사 | G1/G5/G10/G11 PASS, EXPECT 11개 동일, 제품/옛 증거/V318 불변, G7–G9 세 deferral 확인 | `structure.log` |

각 helper 호출 뒤 지정한 `snapshots` 디렉터리가 비었음을 확인했다. 두 clone의 Source HEAD는 모두 정확히 base이며 `git status --porcelain -uall`은 빈 값, `git ls-files Cargo.lock`도 빈 값이다. GREEN과 집중 시험은 pristine clone의 cwd에서 수정 helper의 Source 절대 경로를 호출했다. clone의 옛 helper 파일을 덮어쓰지 않았으며, Cargo가 읽은 archive 제품 바이트는 C2와 같다. 수정 helper 자체의 신원은 위 SHA256으로 묶었다.

probe root는 `/tmp/v353-c2.wjOeCn`이며 Source target은 사용하지 않았다. helper가 정한 실제 target은 `/private/tmp/v353-c2.wjOeCn/green/target`이다. 집중 시험 전에 이 target이 없음을 assert했고, 157개 package를 해석하고 30.47초 동안 컴파일했다. registry cache는 기존 환경의 것을 offline으로 사용했다. helper를 원래 Source에서 실행할 때의 `$repo/target`은 공유 캐시다. 이번 결과를 그 공유 캐시의 cold build로 설명하지 않는다.

## 의존성과 toolchain

| 경로 | 입력 | 해석/보존된 lock SHA256 | zip |
| --- | --- | --- | --- |
| 집중 시험 | `DEPENDENCY_INPUT=absent`, offline registry 해석 | `e3bc5a141eb24b2ed63b11a6ddcb7979c855615e926cc8a575423f0e0c5ab8d1` | 9.1.1 (`focused-no-lock.log:122`) |
| 선택적 lock 검사 | 기존 Source lock을 probe clone에 복사, `--version` | `a28b1372dcf6c2a945130ff6e1b6e5b43444f5b29f0d7b6b41847c003a351c93` | 9.0.3 (`structure.log:7`) |

`--version`은 새 lock을 만들지 않는다. 선택적 입력 검사는 복사와 식별만 증명하며 그 lock으로 시험을 다시 실행한 것이 아니다. 잠금 파일의 원본 바이트는 바꾸지 않았다. 제품 소스 동등성은 의존성 동등성을 뜻하지 않는다. 새 lock에서 한 시험이 통과한 사실로 전체 suite/clippy/WASM 통과를 주장하지 않는다.

`cargo-version.log`와 `rustc-version.log`: cargo 1.95.0 (`f2d3ce0bd7f24a49f8f72d9000448f8838c4e850`), rustc 1.95.0 (`59807616e1fa2540724bfbac14d7976d7e4a3860`), host `aarch64-apple-darwin`, LLVM 22.1.2. Bash LSP는 설치되어 있지 않고 이번 범위에 설치하지 않았다. 네 shell 파일은 각각 `bash -n`으로 확인했다.

## 게이트 carry와 남은 일

현재 원장은 8개 carry/구조 통과와 G7–G9 세 deferral이다. `mah unlazy-status`는 invalid/unmet 0, 세 항목을 handoff로 분류했다. 전체 CHECK를 한 run으로 재봉인하지 않았다.

- G1/G5의 baseline·음성대조 JSON·현재 시험 digest는 불변이다. G2/G3/G4/G6의 제품/시험/fixture도 불변이라 5315315/a6d024f의 기존 실행과 R2 검토를 입력 동일 근거로 이어 쓴다. 이번 새 의존성 해석으로 이 게이트들을 재실행한 것은 아니다.
- C1의 전체 1252/0/7, clippy/WASM Finished는 추가 lock 입력 `a28b…`와 기존 target을 쓴 역사적 로컬 결과다. R2의 clean-checkout 지적을 닫는 최종 근거로 승격하지 않았다.
- G10은 허용 경로 검사와 C2 전용 범위 검사 모두 통과했다. G11의 기존 6개 artifact는 변경하지 않았고, R2 및 Parent의 별도 native opening 근거를 carry한다. 전체 layout/page count나 p195의 IR 한계가 새로 해결됐다는 주장은 없다.
- Parent가 read-only R3를 별도로 실행하고, 수렴한 최종 clean checkout에서 G7 전체 Cargo suite·G8 clippy·G9 wasm32 build를 한 번 실행해야 한다. 세 게이트 모두 `ABANDON: G<n> DEFERRED-TO-LANDING — final clean-checkout dependency resolution after R3`로 정직하게 남겼다.

현재 세션은 helper/원장 diff를 직접 읽고 실행 결과와 대조했다. 추가 substantive 문제는 관측하지 않았다. 이는 같은 Codex executor의 점검이며 독립 보증·R3 승인·self-review receipt가 아니다. push/merge/Actions/child dispatch는 하지 않았다. checkpoint 뒤의 clean/product 동일성 결과와 lease 종료 상태는 완료 handoff에 기록한다.

jev=used — helper/ledger/lock/c2 질의에서 사용했고 결정적 G7/G8/G9 pin을 모두 보존했다. 새 c2 디렉터리 질의는 넓은 lexical/stem 후보도 반환한다. 처분 문서의 후보 0 질의는 `jev=unused(후보 0)`이며 판단으로 세지 않았다. 근거는 `impact-helper.log`, `impact-ledger.log`, `impact-lock.log`, `impact-evidence.log`다. 앞의 세 로그 발췌에서 끝 공백 5행만 정규화했고, 결속·경로·명령 토큰은 보존했다.

INTENT RHWP-HWPX-SAVE-FIDELITY-L2-V353@r1 sha256:d385ca9b6e37df48347dfc71d28bfbc161c1db249304100a064347645b479366

AC1: met — `/Users/ken/.multiagent/reports/v353-rhwp-hwpx-save-l2/review-r2/REVIEW_R2.md:106`의 fixture/음성대조 판정과 `c2/structure.log:2`의 제품 입력 동일성; 승인된 p195 처분 유지.

AC2: met — 같은 `REVIEW_R2.md:108`의 G3/두 번 저장 판정과 불변 제품/시험 바이트; 새 helper의 집중 시험도 `c2/focused-no-lock.log:161`에서 통과.

AC3: met — 같은 `REVIEW_R2.md:110` 및 기존 원장의 측정/회귀 inventory를 carry; 제품 수정 0이며 새 결함 수정 범위를 만들지 않았다.

AC4: deferred — `gates/V353_RHWP_HWPX_SAVE_L2_GATES.md:33`의 G7–G9 세 deferral; `c2/structure.log`의 scope/6개 artifact 확인과 helper RED/GREEN은 완료했지만 최종 clean-checkout heavy 검증은 R3 뒤에 남아 있다.
