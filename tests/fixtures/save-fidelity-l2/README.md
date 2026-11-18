# HWPX 저장 충실도 레인 2 시험 표본 (v353)

`tests/hwpx_save_fidelity_l2.rs` 가 쓰는 표본이다. 기대값은 한컴이 저장한 C6 의 XML 에서 낸다.

| 파일 | 원본 경로 | 바이트 | SHA-256 | 무엇인가 |
|---|---|---:|---|---|
| `C6_hancom_original.hwpx` | `/Users/ken/ken wiki/outputs/v337-resume/G0_RHWP_QA/g0-c6-input.hwpx` (진단 문서의 `g0-c6-input.hwpx`, 같은 해시 사본 `outputs/v338-resume/C10_NATIVE_QA/OWNED_C6_INPUT_223A.hwpx`) | 217,029 | `223a0d79395d7123388cbbc95030414b31104ad03dd443d55a3632683cd3faea` | 한컴오피스 한글 12 가 저장한 원본 (중소벤처기업부 공고 사업신청서, 문단 2,050 · 표 82) |
| `FACF_gearup_template.hwpx` | `/Users/ken/ken wiki/outputs/v338-resume/C10_NATIVE_QA/OWNED_ORIGINAL_FACF.hwpx` | 127,213 | `facf154a897924da8fcf293206949281030266ed4386b2ad05b97d639dcbe5bb` | GearUp 템플릿 — G0 시기 rhwp WASM 이 C6 를 내보낸 파일. 문단·글자는 C6 와 같고 run 순서는 그 rhwp 의 것이다 |

`hp:line` 은 이 폴더가 아니라 레인 1 표본 `../table-fidelity/T6_table_giant_cell_overfill.hwpx` 로 잰다(출처는 그 폴더 README).

## 재검증

```bash
shasum -a 256 tests/fixtures/save-fidelity-l2/*.hwpx
```
