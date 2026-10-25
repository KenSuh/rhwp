# HWPX 표 충실도 시험 표본 (v353)

`tools/roundtrip_diff.py` 와 `tests/hwpx_table_fidelity.rs` 가 쓰는 표본이다.
파일 이름 앞의 `T<n>_` 은 분석 보고서 §9.1 테스트 매트릭스의 사례 번호다. 하네스는 이 접두어로 사례를 찾는다.

## 출처와 라이선스

모든 표본은 업스트림 `edwardkim/rhwp` v0.8.7
(커밋 `1a76570e833917d15817415a53c09ad61ab3203f`, 2026-10-06) 의 `samples/` 에서 바이트 그대로 복사했다.
업스트림 저장소는 MIT 라이선스다(Copyright (c) 2025-2026 Edward Kim). 라이선스 전문은 업스트림 `LICENSE` 를 따른다.

복사 원본 위치(분석 작업 사본):
`~/Downloads/rea-table-analysis-20261025-170753/_analysis/rhwp-upstream/samples/`

| 사례 | 파일 | 업스트림 원 경로 | 바이트 | SHA-256 | 검증 대상(보고서 §9.1) |
|---|---|---|---:|---|---|
| T1 | `T1_press_release_cell_logo.hwpx` | `samples/issue6310/press_release_cell_logo.hwpx` | 679,201 | `67cf6aa2f6042cd0c743c7ae03d46cc97a0c398df331606c98b26dacc6c62f1e` | 셀 `lineWrap=SQUEEZE`, `noAdjust`, `allowOverlap`, 셀 그림 |
| T2 | `T2_2026_oss_rst.hwpx` | `samples/hwpx/2026_oss_rst.hwpx` | 61,434 | `90858f0ecf9e95afe46b6dace6a61960b03552236a1204be530ce3d1866a8398` | `lock=1`, `pageBreak=TABLE` |
| T5 | `T5_36395325_gyeoljae_consulting.hwpx` | `samples/task2243/36395325_gyeoljae_consulting.hwpx` | 117,672 | `a0bd691420456e4b6c0b0127d526b6a38d7c1afc3f07e9b84cddb01495e48b0d` | `pageBreak=TABLE`, `protect=1`, 그림 bin 참조 |
| T6 | `T6_table_giant_cell_overfill.hwpx` | `samples/table_giant_cell_overfill.hwpx` | 534,016 | `5d7eb4a21e46d9ad01a0f631eea2b1f2ec8a71750b4d448868e944e1b95042f4` | 표시 높이 ≠ 셀 높이 합 |
| T7 | `T7_nested_row_first_line_height.hwpx` | `samples/issue5884/nested_row_first_line_height.hwpx` | 28,383 | `c6bcf3686e8fbfbc79f5f3e2c690a62577e8ef63f0bad74e66ee3b255d76b931` | 중첩 표 |

T5 는 그림 `binaryItemIDRef` 직렬화 오류(보고서 부록 B.4, Phase 4 소관)를 관찰하려고 함께 둔다.
v353 레인은 그 오류를 고치지 않는다.

## 재검증

```bash
shasum -a 256 tests/fixtures/table-fidelity/*.hwpx
```
