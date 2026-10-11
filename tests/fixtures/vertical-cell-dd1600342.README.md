# Vertical cell pre-L1 render oracle

`vertical-cell-dd1600342.txt` was captured by executing the original library from
`dd1600342c68ef920a6211f783d92a7d5fb53252`, built offline from a git archive in an
isolated target directory. It was not generated from the corrected engine.

Generator: `tests/common/vertical_baseline.rs::census`, driven by the shared
`tests/common/reflow_geometry.rs` fixtures. Each row pins the scenario id, model
character count, rendered character count, and ordered rendered text across all
pages. The 1,128 rows cover both vertical directions, index/path APIs, nested and
merged cells, inherited padding, CJK/Latin, fixed/automatic/zero geometry,
actual insertion, 900-to-1800 font changes, spacing, repeated insertion and
document reflow. Clipping inherited from the baseline is intentionally visible.

The R3 entry engine is a negative control; it produces a different census.
The separate positive-geometry test asserts all 112 characters after insertion
and a real 900-to-600 font change. Vertical reflow improvements are a follow-up;
this lane changes horizontal cell formatting only.
