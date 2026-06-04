//! HWPX Phase 1 저장/재열기 fixture corpus matrix를 검증한다.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::model::document::Document;
use rhwp::model::paragraph::Paragraph;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CorpusMetrics {
    sections: usize,
    paragraphs: usize,
    top_level_controls: usize,
    tables: usize,
    table_cells: usize,
    table_cell_paragraphs: usize,
    pictures: usize,
    shapes: usize,
    text_chars: usize,
    bin_data: usize,
    char_shapes: usize,
    para_shapes: usize,
    border_fills: usize,
    styles: usize,
}

impl CorpusMetrics {
    fn from_document(doc: &Document) -> Self {
        let mut metrics = Self {
            sections: doc.sections.len(),
            bin_data: doc.bin_data_content.len(),
            char_shapes: doc.doc_info.char_shapes.len(),
            para_shapes: doc.doc_info.para_shapes.len(),
            border_fills: doc.doc_info.border_fills.len(),
            styles: doc.doc_info.styles.len(),
            ..Self::default()
        };

        for section in &doc.sections {
            metrics.paragraphs += section.paragraphs.len();
            for paragraph in &section.paragraphs {
                metrics.text_chars += paragraph.text.chars().count();
                metrics.top_level_controls += paragraph.controls.len();
                for control in &paragraph.controls {
                    metrics.visit_control(control);
                }
            }
        }

        metrics
    }

    fn visit_control(&mut self, control: &Control) {
        match control {
            Control::Table(table) => {
                self.tables += 1;
                self.table_cells += table.cells.len();
                for cell in &table.cells {
                    self.table_cell_paragraphs += cell.paragraphs.len();
                    for paragraph in &cell.paragraphs {
                        self.text_chars += paragraph.text.chars().count();
                        for nested in &paragraph.controls {
                            self.visit_control(nested);
                        }
                    }
                }
            }
            Control::Picture(_) => {
                self.pictures += 1;
            }
            Control::Shape(_) => {
                self.shapes += 1;
            }
            Control::Header(header) => {
                for paragraph in &header.paragraphs {
                    self.text_chars += paragraph.text.chars().count();
                    for nested in &paragraph.controls {
                        self.visit_control(nested);
                    }
                }
            }
            Control::Footer(footer) => {
                for paragraph in &footer.paragraphs {
                    self.text_chars += paragraph.text.chars().count();
                    for nested in &paragraph.controls {
                        self.visit_control(nested);
                    }
                }
            }
            Control::Footnote(footnote) => {
                for paragraph in &footnote.paragraphs {
                    self.text_chars += paragraph.text.chars().count();
                    for nested in &paragraph.controls {
                        self.visit_control(nested);
                    }
                }
            }
            Control::Endnote(endnote) => {
                for paragraph in &endnote.paragraphs {
                    self.text_chars += paragraph.text.chars().count();
                    for nested in &paragraph.controls {
                        self.visit_control(nested);
                    }
                }
            }
            _ => {}
        }
    }

    fn regressions(&self, after: &Self) -> Vec<String> {
        let mut regressions = Vec::new();
        self.push_if_less(
            after,
            &mut regressions,
            "sections",
            self.sections,
            after.sections,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "paragraphs",
            self.paragraphs,
            after.paragraphs,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "top_level_controls",
            self.top_level_controls,
            after.top_level_controls,
        );
        self.push_if_less(after, &mut regressions, "tables", self.tables, after.tables);
        self.push_if_less(
            after,
            &mut regressions,
            "table_cells",
            self.table_cells,
            after.table_cells,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "table_cell_paragraphs",
            self.table_cell_paragraphs,
            after.table_cell_paragraphs,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "pictures",
            self.pictures,
            after.pictures,
        );
        self.push_if_less(after, &mut regressions, "shapes", self.shapes, after.shapes);
        self.push_if_less(
            after,
            &mut regressions,
            "text_chars",
            self.text_chars,
            after.text_chars,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "bin_data",
            self.bin_data,
            after.bin_data,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "char_shapes",
            self.char_shapes,
            after.char_shapes,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "para_shapes",
            self.para_shapes,
            after.para_shapes,
        );
        self.push_if_less(
            after,
            &mut regressions,
            "border_fills",
            self.border_fills,
            after.border_fills,
        );
        self.push_if_less(after, &mut regressions, "styles", self.styles, after.styles);
        regressions
    }

    fn push_if_less(
        &self,
        _after: &Self,
        regressions: &mut Vec<String>,
        name: &str,
        before: usize,
        after: usize,
    ) {
        if after < before {
            regressions.push(format!("{} {} -> {}", name, before, after));
        }
    }
}

impl fmt::Display for CorpusMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sec={} para={} ctrl={} tbl={} cell={} cell_para={} pic={} shape={} text={} bin={} charPr={} paraPr={} borderFill={} style={}",
            self.sections,
            self.paragraphs,
            self.top_level_controls,
            self.tables,
            self.table_cells,
            self.table_cell_paragraphs,
            self.pictures,
            self.shapes,
            self.text_chars,
            self.bin_data,
            self.char_shapes,
            self.para_shapes,
            self.border_fills,
            self.styles,
        )
    }
}

fn top_level_control_counts(doc: &Document) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for section in &doc.sections {
        for paragraph in &section.paragraphs {
            for control in &paragraph.controls {
                *counts.entry(control_name(control)).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn all_control_counts(doc: &Document) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for section in &doc.sections {
        for paragraph in &section.paragraphs {
            count_controls_recursive(&paragraph.controls, &mut counts);
        }
    }
    counts
}

fn count_controls_recursive(controls: &[Control], counts: &mut BTreeMap<String, usize>) {
    for control in controls {
        *counts.entry(control_name(control)).or_insert(0) += 1;
        match control {
            Control::Table(table) => {
                for cell in &table.cells {
                    for paragraph in &cell.paragraphs {
                        count_controls_recursive(&paragraph.controls, counts);
                    }
                }
            }
            Control::Header(header) => {
                for paragraph in &header.paragraphs {
                    count_controls_recursive(&paragraph.controls, counts);
                }
            }
            Control::Footer(footer) => {
                for paragraph in &footer.paragraphs {
                    count_controls_recursive(&paragraph.controls, counts);
                }
            }
            Control::Footnote(footnote) => {
                for paragraph in &footnote.paragraphs {
                    count_controls_recursive(&paragraph.controls, counts);
                }
            }
            Control::Endnote(endnote) => {
                for paragraph in &endnote.paragraphs {
                    count_controls_recursive(&paragraph.controls, counts);
                }
            }
            _ => {}
        }
    }
}

fn control_name(control: &Control) -> String {
    match control {
        Control::SectionDef(_) => "SectionDef".to_string(),
        Control::ColumnDef(_) => "ColumnDef".to_string(),
        Control::Table(_) => "Table".to_string(),
        Control::Shape(_) => "Shape".to_string(),
        Control::Picture(_) => "Picture".to_string(),
        Control::Header(_) => "Header".to_string(),
        Control::Footer(_) => "Footer".to_string(),
        Control::Footnote(_) => "Footnote".to_string(),
        Control::Endnote(_) => "Endnote".to_string(),
        Control::AutoNumber(_) => "AutoNumber".to_string(),
        Control::NewNumber(_) => "NewNumber".to_string(),
        Control::PageNumberPos(_) => "PageNumberPos".to_string(),
        Control::Bookmark(_) => "Bookmark".to_string(),
        Control::Hyperlink(_) => "Hyperlink".to_string(),
        Control::Ruby(_) => "Ruby".to_string(),
        Control::CharOverlap(_) => "CharOverlap".to_string(),
        Control::PageHide(_) => "PageHide".to_string(),
        Control::HiddenComment(_) => "HiddenComment".to_string(),
        Control::Equation(_) => "Equation".to_string(),
        Control::Field(_) => "Field".to_string(),
        Control::Form(_) => "Form".to_string(),
        Control::Unknown(unknown) => format!("Unknown(0x{:08X})", unknown.ctrl_id),
    }
}

fn format_control_count_delta(
    before: &BTreeMap<String, usize>,
    after: &BTreeMap<String, usize>,
) -> String {
    let mut names = BTreeSet::new();
    names.extend(before.keys().cloned());
    names.extend(after.keys().cloned());

    let mut deltas = Vec::new();
    for name in names {
        let before_count = before.get(&name).copied().unwrap_or(0);
        let after_count = after.get(&name).copied().unwrap_or(0);
        if before_count != after_count {
            deltas.push(format!("{} {} -> {}", name, before_count, after_count));
        }
    }

    if deltas.is_empty() {
        "none".to_string()
    } else {
        deltas.join(", ")
    }
}

fn text_delta_summary(before: &Document, after: &Document) -> String {
    let mut before_entries = Vec::new();
    let mut after_entries = Vec::new();
    collect_text_entries(before, &mut before_entries);
    collect_text_entries(after, &mut after_entries);

    let max_len = before_entries.len().max(after_entries.len());
    let mut deltas = Vec::new();
    for idx in 0..max_len {
        match (before_entries.get(idx), after_entries.get(idx)) {
            (Some((before_path, before_text)), Some((after_path, after_text))) => {
                if before_path != after_path || before_text != after_text {
                    deltas.push(format!(
                        "{} -> {}: {} -> {}; before={:?}; after={:?}",
                        before_path,
                        after_path,
                        before_text.chars().count(),
                        after_text.chars().count(),
                        summarize_text(before_text),
                        summarize_text(after_text),
                    ));
                }
            }
            (Some((before_path, before_text)), None) => {
                deltas.push(format!(
                    "{} missing after: {}; before={:?}",
                    before_path,
                    before_text.chars().count(),
                    summarize_text(before_text),
                ));
            }
            (None, Some((after_path, after_text))) => {
                deltas.push(format!(
                    "{} added after: {}; after={:?}",
                    after_path,
                    after_text.chars().count(),
                    summarize_text(after_text),
                ));
            }
            (None, None) => {}
        }
        if deltas.len() >= 5 {
            break;
        }
    }

    if deltas.is_empty() {
        "none".to_string()
    } else {
        deltas.join(" | ")
    }
}

fn collect_text_entries(doc: &Document, out: &mut Vec<(String, String)>) {
    for (si, section) in doc.sections.iter().enumerate() {
        for (pi, paragraph) in section.paragraphs.iter().enumerate() {
            collect_paragraph_text_entries(&format!("s{}.p{}", si, pi), paragraph, out);
        }
    }
}

fn collect_paragraph_text_entries(
    path: &str,
    paragraph: &Paragraph,
    out: &mut Vec<(String, String)>,
) {
    out.push((path.to_string(), paragraph.text.clone()));
    for (ci, control) in paragraph.controls.iter().enumerate() {
        match control {
            Control::Table(table) => {
                for (cell_index, cell) in table.cells.iter().enumerate() {
                    for (cell_para_index, cell_para) in cell.paragraphs.iter().enumerate() {
                        collect_paragraph_text_entries(
                            &format!(
                                "{}.ctrl{}.cell{}.p{}",
                                path, ci, cell_index, cell_para_index
                            ),
                            cell_para,
                            out,
                        );
                    }
                }
            }
            Control::Header(header) => {
                for (nested_index, nested) in header.paragraphs.iter().enumerate() {
                    collect_paragraph_text_entries(
                        &format!("{}.ctrl{}.header.p{}", path, ci, nested_index),
                        nested,
                        out,
                    );
                }
            }
            Control::Footer(footer) => {
                for (nested_index, nested) in footer.paragraphs.iter().enumerate() {
                    collect_paragraph_text_entries(
                        &format!("{}.ctrl{}.footer.p{}", path, ci, nested_index),
                        nested,
                        out,
                    );
                }
            }
            Control::Footnote(footnote) => {
                for (nested_index, nested) in footnote.paragraphs.iter().enumerate() {
                    collect_paragraph_text_entries(
                        &format!("{}.ctrl{}.footnote.p{}", path, ci, nested_index),
                        nested,
                        out,
                    );
                }
            }
            Control::Endnote(endnote) => {
                for (nested_index, nested) in endnote.paragraphs.iter().enumerate() {
                    collect_paragraph_text_entries(
                        &format!("{}.ctrl{}.endnote.p{}", path, ci, nested_index),
                        nested,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}

fn summarize_text(text: &str) -> String {
    text.chars().take(60).collect()
}

struct HwpxFixtureSet {
    files: Vec<PathBuf>,
    skipped_non_hwpx: Vec<PathBuf>,
}

fn hwpx_fixtures() -> HwpxFixtureSet {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/hwpx");
    let mut files = Vec::new();
    collect_hwpx_files(&root, &mut files);
    files.sort();

    let mut hwpx_files = Vec::new();
    let mut skipped_non_hwpx = Vec::new();
    for file in files {
        if is_zip_hwpx(&file) {
            hwpx_files.push(file);
        } else {
            skipped_non_hwpx.push(file);
        }
    }

    HwpxFixtureSet {
        files: hwpx_files,
        skipped_non_hwpx,
    }
}

fn collect_hwpx_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("read fixture dir {:?}: {}", dir, err));
    for entry in entries {
        let path = entry.expect("read fixture entry").path();
        if path.is_dir() {
            collect_hwpx_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("hwpx"))
        {
            files.push(path);
        }
    }
}

fn is_zip_hwpx(path: &Path) -> bool {
    fs::read(path)
        .map(|bytes| bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]))
        .unwrap_or(false)
}

#[test]
#[ignore = "Phase 1 GA gate: run manually with --ignored --nocapture until all fixture regressions are fixed"]
fn phase1_hwpx_fixture_corpus_export_reparse_matrix() {
    let fixture_set = hwpx_fixtures();
    let fixtures = fixture_set.files;
    assert!(
        fixtures.len() >= 10,
        "Phase 1 corpus must include at least 10 HWPX fixtures, got {}",
        fixtures.len()
    );

    eprintln!("\n=== Phase 1 HWPX export/reparse corpus matrix ===");
    if !fixture_set.skipped_non_hwpx.is_empty() {
        eprintln!("skipped non-ZIP HWPX fixtures:");
        for skipped in &fixture_set.skipped_non_hwpx {
            let relative = skipped
                .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
                .unwrap_or(skipped);
            eprintln!("  - {}", relative.display());
        }
    }
    eprintln!(
        "{:<58} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "fixture", "para", "ctrl", "tbl", "cell", "pic", "text"
    );
    eprintln!("{}", "-".repeat(104));

    let mut failures = Vec::new();
    for fixture in fixtures {
        let bytes = match fs::read(&fixture) {
            Ok(bytes) => bytes,
            Err(err) => {
                failures.push(format!("{}: read failed: {}", fixture.display(), err));
                continue;
            }
        };

        let before = match DocumentCore::from_bytes(&bytes) {
            Ok(core) => core,
            Err(err) => {
                failures.push(format!("{}: parse failed: {}", fixture.display(), err));
                continue;
            }
        };
        let before_metrics = CorpusMetrics::from_document(before.document());
        let before_control_counts = top_level_control_counts(before.document());
        let before_all_control_counts = all_control_counts(before.document());

        let saved = match before.export_hwpx_native() {
            Ok(saved) => saved,
            Err(err) => {
                failures.push(format!("{}: export failed: {}", fixture.display(), err));
                continue;
            }
        };

        let after = match DocumentCore::from_bytes(&saved) {
            Ok(core) => core,
            Err(err) => {
                failures.push(format!("{}: reparse failed: {}", fixture.display(), err));
                continue;
            }
        };
        let after_metrics = CorpusMetrics::from_document(after.document());
        let after_control_counts = top_level_control_counts(after.document());
        let after_all_control_counts = all_control_counts(after.document());
        let regressions = before_metrics.regressions(&after_metrics);

        let relative = fixture
            .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
            .unwrap_or(&fixture);
        eprintln!(
            "{:<58} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            relative.display(),
            after_metrics.paragraphs,
            after_metrics.top_level_controls,
            after_metrics.tables,
            after_metrics.table_cells,
            after_metrics.pictures,
            after_metrics.text_chars,
        );

        if !regressions.is_empty() {
            failures.push(format!(
                "{}: structural regression: {}; top-level control deltas [{}]; all control deltas [{}]; text deltas [{}]; before [{}], after [{}]",
                relative.display(),
                regressions.join(", "),
                format_control_count_delta(&before_control_counts, &after_control_counts),
                format_control_count_delta(&before_all_control_counts, &after_all_control_counts),
                text_delta_summary(before.document(), after.document()),
                before_metrics,
                after_metrics,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Phase 1 HWPX fixture corpus export/reparse regressions:\n{}",
        failures.join("\n")
    );
}
