# Task 1790 — 2단계 완료 보고서: embed opts + 반환 핸들

브랜치: feature/embed-bootstrap-api-1790

## 수행 내용

- `BootstrapOpts` — `enablePostMessageApi?`(기본 true·false 면 window message
  리스너 미등록), `extendContextMenuItems?`(ContextMenu.show 래핑 훅),
  `embed.minZoom?`(ViewportManager.setZoom 하한 래핑 + 문서 로드 시 하한 미만이면
  100% 정규화 — grid 레이아웃 진입 차단).
  - 계획서의 `disableMobileAutoFit` 은 별도 플래그로 만들지 않음 — minZoom 하한
    래핑이 모바일 폭맞춤 setZoom 도 clamp 하고, 로드 시 정규화가 잔여 케이스를
    덮어 의미가 중복. (계획 대비 축소·사유 기록)
- `BootstrapHandle` 반환 — wasm·eventBus·extensionAPI(StudioExtensionAPI 신설)·
  createInsertTextCommand/createSplitParagraphCommand(셀 내부는
  SplitParagraphInCellCommand 자동 선택)·ready(초기화 실패 시 reject —
  initialize 가 rethrow·스탠드얼론은 내부 catch 로 unhandled rejection 방지)·
  getInputHandler/getCanvasView·loadFile/loadBytes/createNewDocument·destroy.
- 스탠드얼론 불변 — opts 미지정 시 기존 동작과 완전 동일 (main.ts 변경 없음).

## 검증 결과

| 항목 | 결과 |
|------|------|
| tsc | bootstrap.ts/main.ts 에러 0 (전체 25건 = 1단계에서 입증된 pre-existing 그대로) |
| vite build | ✅ 성공 |
| 스탠드얼론 부팅 (opts 미지정) | ✅ "HWP 파일을 선택해주세요" 도달·메뉴바·DEV 전역(__wasm) 정상 |

## 다음 단계

3단계 — 표/그림 spec 삽입 command factory 이관 (GearUp 스냅샷 구현 upstream 화).
