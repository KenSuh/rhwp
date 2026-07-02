# Task 1790 — 1단계 완료 보고서: bootstrap 추출 (동작 불변)

브랜치: feature/embed-bootstrap-api-1790
계획서: mydocs/plans/task_1790.md

## 수행 내용

- `src/bootstrap.ts` 신설 — `export function bootstrap(rootEl: HTMLElement): void`.
  main.ts(635줄)의 초기화 로직 전체를 verbatim 이동:
  모듈 상태(wasm/eventBus/canvasView/inputHandler/toolbar/ruler/totalSections)
  → 함수 스코프, 커맨드 시스템(registry/dispatcher/8종 등록), initialize()
  (CanvasView·Ruler·InputHandler·Toolbar·표 렌더러 4종·ContextMenu·
  CommandPalette·MenuBar·도구상자 바인딩), setupFileInput/ZoomControls/
  EventListeners/GlobalShortcuts, initializeDocument·loadFile·loadBytes·
  createNewDocument·notifyHwpxBetaIfNeeded·loadFromUrlParam, eventBus 구독
  3종(create-new-document/open-document-bytes/equation-edit-request),
  postMessage API(hwpctl-load + rhwp-request).
- DOM 접근 스코프화 — `document.getElementById('X')` → rootEl 스코프
  `q('X')`, `document.querySelectorAll('.tb-*')` → `rootEl.querySelectorAll`.
  document/window 레벨 리스너(전역 단축키·drop 방지·스플릿 메뉴 외부 클릭·
  postMessage)와 DEV 전역 노출(__wasm/__eventBus/__inputHandler/__canvasView)은
  기존 그대로 유지.
- `src/main.ts` → `bootstrap(document.getElementById('studio-root'))` 호출
  1줄로 축소 (index.html 진입점·번들 계약 불변).

## 검증 결과

| 항목 | 결과 |
|------|------|
| vite build | ✅ 성공 (458ms) |
| tsc | 에러 25건 — **pristine 8944b4a 와 동일**(git stash 후 재빌드로 입증). bootstrap.ts/main.ts 신규 에러 0. 원인 = devDep `typescript ^6` 최신판의 기존 엔진 파일(commands/table.ts·input-handler-keyboard.ts·input-handler.ts) 지적 — 본 타스크 범위 외 |
| 스탠드얼론 부팅 (vite dev :7700) | ✅ 메뉴바/도구상자/서식도구/눈금자/상태표시줄 정상 |
| 문서 로드 (?url= 경로·3페이지 HWPX) | ✅ initDoc 1~8 시퀀스 완료·비표준 lineseg 15건 감지 모달→자동 보정 정상 |
| 표 편집 스모크 | ✅ 표 셀 클릭 → isInTable()=true·cellPath 정확(cellIndex 18)·cursorRect 정상 |

## 다음 단계 (승인 대기)

2단계 — embed opts(`enablePostMessageApi`·`extendContextMenuItems`·
`embed: { minZoom, disableMobileAutoFit }`) + 반환 핸들(wasm/extensionAPI/
command factory/getInputHandler/getCanvasView/loadBytes/ready/destroy).
