# Task 1790 — 3단계 완료 보고서: 표/그림 spec 삽입 command factory 이관

브랜치: feature/embed-bootstrap-api-1790

## 수행 내용

- `src/engine/embed-commands.ts` 신설 — GearUp 임베드 런타임에서 검증된 스냅샷
  undo 기반 구현을 TS 로 이관.
  - `EmbedInsertTableCommand` / `EmbedInsertPictureCommand` — `EditCommand`
    인터페이스 구현(execute/undo/mergeWith/discard·스냅샷 save/restore/discard).
  - 본문 위치(`createTable`/`insertTextInCell`/`insertPicture`)와 셀 내부
    위치(`createTableInCellByPath`/`insertTextInCellByPath`/
    `insertPictureInCellByPath`) 모두 지원 — cellPath 정규화 helper 포함.
  - 그림: binData 전용·PNG 헤더 실측 치수(그 외 960×540 기본)·mm→HWPUNIT 변환
    (7200/25.4)·비율 유지.
- `BootstrapHandle` 에 `createInsertTableCommand`/`createInsertPictureCommand`
  factory 추가 (+ `EmbedTableSpec`/`EmbedPictureSpec` 타입 export).

## 검증 결과

| 항목 | 결과 |
|------|------|
| tsc | bootstrap.ts / embed-commands.ts 에러 0 (전체 25건 = pre-existing 그대로) |
| vite build | ✅ 성공 |
| 런타임 스모크 | 호스트 임베드 전환(GearUp 저장소) 후 표/그림 삽입 QA 로 검증 — 전환 작업의 검증 항목에 포함 |

## 후속

- GearUp 저장소에서 복제 런타임 삭제 → 본 bootstrap 소비로 전환 (본 이슈 범위 외,
  호스트 저장소 커밋).
- 표 spec 서식(정렬/헤더 스타일)·이미지 포맷 확장은 후속 이슈로.
