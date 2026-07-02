# Task 1790 — 최종 결과 보고서: rhwp-studio embed bootstrap API

이슈: https://github.com/edwardkim/rhwp/issues/1790
브랜치: feature/embed-bootstrap-api-1790 (기점 8944b4a3)
커밋: 000a59d(계획서) → 18eab44(1단계) → d7e1891(2단계) → fd83733(3단계)

## 결과 요약

| 단계 | 내용 | 검증 |
|------|------|------|
| 1 | main.ts(635줄) 초기화 로직을 `bootstrap(rootEl)` 로 추출 — 스탠드얼론 동작 불변 (main.ts = 호출 1줄) | vite build ✅·tsc 신규 에러 0(전체 25건은 pristine 동일 pre-existing)·스탠드얼론 부팅/문서 로드/표 셀 스모크 ✅ |
| 2 | `BootstrapOpts`(postMessage 토글·컨텍스트 메뉴 훅·embed.minZoom) + `BootstrapHandle`(wasm·eventBus·extensionAPI·factory·ready·접근자·destroy) | 동일 기준 ✅·opts 미지정 스탠드얼론 무회귀 ✅ |
| 3 | 표/그림 spec 삽입 커맨드(`engine/embed-commands.ts`·스냅샷 undo·본문/셀 내부) upstream 이관 + 핸들 factory | tsc/빌드 ✅·런타임 검증은 호스트 전환 스모크에 포함 |

## 호스트(GearUp) 전환 결과 — 본 API 의 실소비 검증

GearUp 저장소에서 복제 런타임(rhwp-bootstrap-runtime.ts, 929줄) 삭제 후 본
bootstrap 직접 소비로 전환 완료(fddadbcc). 라이브 스모크 — 문서 로드·
embed.minZoom 0.55 적용(grid 미진입)·협업 2클라이언트(원격 커서/선택/편집중
표시)·비표준 lineseg 자동 보정 모달 — 전부 정상.

## 계획 대비 변경점

- `embed.disableMobileAutoFit` 별도 플래그 미구현 — `minZoom` 하한 래핑이 모바일
  폭맞춤 setZoom 을 clamp 하고 로드 시 정규화가 잔여 케이스를 덮어 의미 중복
  (2단계 보고서에 기록).

## 남은 항목 (후속 이슈 제안)

- grid(다중 열) 레이아웃용 마우스 히트테스트 — 현재 단일 열 전제
  (`getPageAtY` + 중앙정렬). 임베드는 minZoom 으로 회피 중이나 스탠드얼로도
  zoom ≤ 0.5 축소 보기에서 클릭 좌표가 어긋남.
- embed-commands 표 spec 서식(정렬·헤더 스타일)·이미지 포맷 치수 실측 확장.
- devDep `typescript ^6` 최신판이 지적하는 기존 엔진 파일 tsc 25건 정리.

이슈 클로즈는 작업지시자 승인 후 진행.
