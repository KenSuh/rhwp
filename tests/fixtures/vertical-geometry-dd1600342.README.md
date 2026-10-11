# Vertical geometry oracle

The gzip fixture contains 40,320 rows captured from the original library at
`dd1600342c68ef920a6211f783d92a7d5fb53252`, not from the corrected library.
Its uncompressed SHA256 is
`beeac860efe6dc618a3507480910961601fc2d21c3efc8094700ad167bbbc208`;
the gzip SHA256 is
`68f95520ffefab9e5aae48575c48be2beb7430996090b7b08ed302c465c9ad10`.
Compression uses `gzip -n -c` and adds no timestamp.

The generator iterates `vertical_geometry_matrix::cases()` and calls
`snapshot(case, action)` for document reflow, insertion, a real 1800-to-900
size change, a real 1800-to-600 size change, and eight repeated insertions
that exercise content/row growth. Each action starts from its own fixture.

The 8,064 cases cross all of these axes:

- Ancestor width: positive 45000 HU, zero with fixed height, zero with automatic
  height. The model represents missing/automatic width with zero.
- Ancestor padding: 0 or 7500 HU per side, inherited from the table.
- Ancestor paragraph margins: 0 or 6000 HWP margin units per side.
- One, two or three ancestors, every horizontal/vertical ancestor combination,
  both vertical directions, and target width 0 or 20000 HU.
- Chains hosted in a table, a textbox, or a table caption.
- Physical target cellPath, ancestor index, caption index, and textbox index.

Index APIs address the outer container; nested target edits use the physical
cellPath. The base's text-editing path supports tables only. Paths through
textbox/caption containers therefore compare the native error as well as the
unchanged target; they do not claim a successful nested edit. Document reflow
inside those containers also preserves the base's existing traversal boundary.

Each row compares all nine LineSeg fields, native HeightMeasurer content-height
and row-height float bits, the renderer's max-segment vertical-height input,
and ordered rendered target text across every page. The target contains only
CJK text; ancestor/container insertions contain only ASCII, so filtering ASCII
from the native render tree isolates the target without hiding target clipping.
Horizontal ancestor layout is outside this equivalence assertion.

Capture provenance and negative controls are in the task's runtime report
directory `~/.multiagent/reports/v327-docq-l1/r4-correction/`. All 358 tracked
source/template/manifest inputs of the baseline archive matched their base
Git blobs. The entry engine was built offline from its own 5fd6a93 archive and
differs in 528 rows. A separate descendant-fallback mutation must also fail
the matrix test. Earlier incomplete captures are not acceptance evidence.

The existing 1,128-row render oracle and its positive 112/112-character tests
remain separate. This oracle preserves baseline behavior, including clipping;
it does not implement vertical layout improvements or browser/undo behavior.
