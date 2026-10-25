use super::*;

/// 테스트용 N×M 표 생성 헬퍼
fn make_table(rows: u16, cols: u16) -> Table {
    let cell_width: HwpUnit = 3600; // 약 12.7mm
    let cell_height: HwpUnit = 1000;
    let mut cells = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            cells.push(Cell::new_empty(c, r, cell_width, cell_height, 1));
        }
    }
    let mut table = Table {
        row_count: rows,
        col_count: cols,
        row_sizes: vec![cols as i16; rows as usize],
        border_fill_id: 1,
        cells,
        ..Default::default()
    };
    table.rebuild_grid();
    table
}

#[test]
fn test_table_default() {
    let table = Table::default();
    assert_eq!(table.row_count, 0);
    assert_eq!(table.col_count, 0);
    assert!(table.cells.is_empty());
}

#[test]
fn test_cell_span() {
    let cell = Cell {
        col: 0,
        row: 0,
        col_span: 2,
        row_span: 3,
        ..Default::default()
    };
    assert_eq!(cell.col_span, 2);
    assert_eq!(cell.row_span, 3);
}

#[test]
fn test_cell_new_empty() {
    let cell = Cell::new_empty(2, 3, 3600, 1000, 5);
    assert_eq!(cell.col, 2);
    assert_eq!(cell.row, 3);
    assert_eq!(cell.col_span, 1);
    assert_eq!(cell.row_span, 1);
    assert_eq!(cell.width, 3600);
    assert_eq!(cell.height, 1000);
    assert_eq!(cell.border_fill_id, 5);
    assert_eq!(cell.paragraphs.len(), 1);
    assert_eq!(cell.paragraphs[0].char_count, 1); // 끝 마커(0x000D) 포함
}

#[test]
fn test_get_column_widths() {
    let table = make_table(2, 3);
    let widths = table.get_column_widths();
    assert_eq!(widths, vec![3600, 3600, 3600]);
}

#[test]
fn test_get_row_heights() {
    let table = make_table(2, 3);
    let heights = table.get_row_heights();
    assert_eq!(heights, vec![1000, 1000]);
}

#[test]
fn merged_rows_keep_physical_height_when_dimensions_are_updated() {
    let mut table = make_table(2, 2);
    table.merge_cells(0, 0, 1, 1).unwrap();
    table.cells[0].height = 32001;

    table.update_ctrl_dimensions();

    assert_eq!(table.common.height, 32001);
    assert_eq!(table.get_row_heights(), vec![16001, 16000]);
    assert_eq!(table.get_raw_row_heights(), vec![0, 0]);
}

#[test]
fn merged_rows_fill_only_unknown_heights() {
    let mut table = make_table(2, 2);
    table.merge_cells(0, 0, 1, 0).unwrap();
    table.cells.retain(|cell| cell.row == 0);
    table.cells[0].height = 32000;

    let heights = table.get_row_heights();

    assert_eq!(heights, vec![1000, 31000]);
}

#[test]
fn merged_rows_use_the_largest_span_height_independent_of_cell_order() {
    let mut table = make_table(2, 2);
    table.merge_cells(0, 0, 1, 0).unwrap();
    table.merge_cells(0, 1, 1, 1).unwrap();
    table.cells[0].height = 32000;
    table.cells[1].height = 64000;

    table.update_ctrl_dimensions();

    assert_eq!(table.common.height, 64000);
    assert_eq!(table.get_row_heights(), vec![32000, 32000]);
    table.cells.reverse();
    assert_eq!(table.get_row_heights(), vec![32000, 32000]);
}

#[test]
fn test_update_ctrl_dimensions_syncs_common_attr() {
    let mut table = make_table(2, 3);
    table.common.width = 1;
    table.common.height = 1;

    table.update_ctrl_dimensions();

    assert_eq!(table.common.width, 10800);
    assert_eq!(table.common.height, 2000);
}

#[test]
fn test_update_ctrl_dimensions_syncs_raw_ctrl_data() {
    // raw_ctrl_data 는 HWP5 파서가 담는 CTRL_HEADER 전체(attr 포함)다:
    // [8..12] 가로 오프셋, [12..16] 폭, [16..20] 높이. 가로 오프셋은 건드리지 않는다.
    let mut table = make_table(1, 1);
    let mut raw = vec![0u8; 40];
    raw[8..12].copy_from_slice(&777u32.to_le_bytes());
    table.raw_ctrl_data = raw;

    table.update_ctrl_dimensions();

    assert_eq!(raw_u32(&table.raw_ctrl_data, 12), 3600);
    assert_eq!(raw_u32(&table.raw_ctrl_data, 16), 1000);
    assert_eq!(raw_u32(&table.raw_ctrl_data, 8), 777, "가로 오프셋은 그대로");
}

fn raw_u32(raw: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
}

// === insert_row 테스트 ===

#[test]
fn test_insert_row_below() {
    let mut table = make_table(2, 2);
    assert_eq!(table.cells.len(), 4);

    table.insert_row(0, true).unwrap();

    assert_eq!(table.row_count, 3);
    assert_eq!(table.row_sizes.len(), 3);
    assert_eq!(table.cells.len(), 6);

    // 행 0: 원래 첫 행
    assert_eq!(table.cells[0].row, 0);
    assert_eq!(table.cells[1].row, 0);
    // 행 1: 새 행
    assert_eq!(table.cells[2].row, 1);
    assert_eq!(table.cells[3].row, 1);
    // 행 2: 원래 둘째 행 (시프트)
    assert_eq!(table.cells[4].row, 2);
    assert_eq!(table.cells[5].row, 2);
}

#[test]
fn test_insert_row_above() {
    let mut table = make_table(2, 2);

    table.insert_row(0, false).unwrap();

    assert_eq!(table.row_count, 3);
    assert_eq!(table.cells.len(), 6);

    // 행 0: 새 행
    assert_eq!(table.cells[0].row, 0);
    assert_eq!(table.cells[1].row, 0);
    assert_eq!(table.cells[0].paragraphs.len(), 1); // 빈 문단
                                                    // 행 1: 원래 첫 행 (시프트)
    assert_eq!(table.cells[2].row, 1);
    assert_eq!(table.cells[3].row, 1);
}

#[test]
fn test_insert_row_with_merged_cell() {
    let mut table = make_table(3, 2);
    // (0,0) 셀을 row_span=2로 병합
    table.cells[0].row_span = 2;
    table.cells[0].height = 2000;
    // 병합된 (0,1) 셀 제거
    table.cells.retain(|c| !(c.col == 0 && c.row == 1));

    // 행 0 아래에 삽입 → 병합 셀이 삽입 지점을 걸침
    table.insert_row(0, true).unwrap();

    assert_eq!(table.row_count, 4);
    // (0,0) 셀의 row_span이 3으로 확장되어야 함
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.row_span, 3);
    // 새 행의 열 0은 병합 셀에 의해 커버 → 새 셀 없음
    // 새 행의 열 1에만 새 셀 생성
    let new_cells: Vec<_> = table.cells.iter().filter(|c| c.row == 1).collect();
    assert_eq!(new_cells.len(), 1);
    assert_eq!(new_cells[0].col, 1);
}

#[test]
fn test_insert_row_out_of_bounds() {
    let mut table = make_table(2, 2);
    assert!(table.insert_row(5, true).is_err());
}

// === insert_column 테스트 ===

#[test]
fn test_insert_column_right() {
    let mut table = make_table(2, 2);

    table.insert_column(0, true).unwrap();

    assert_eq!(table.col_count, 3);
    assert_eq!(table.cells.len(), 6);

    // 열 0: 원래, 열 1: 새로, 열 2: 원래 (시프트)
    let row0: Vec<u16> = table
        .cells
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.col)
        .collect();
    assert_eq!(row0, vec![0, 1, 2]);
}

#[test]
fn test_insert_column_left() {
    let mut table = make_table(2, 2);

    table.insert_column(0, false).unwrap();

    assert_eq!(table.col_count, 3);
    assert_eq!(table.cells.len(), 6);

    // 열 0: 새로, 열 1: 원래 (시프트), 열 2: 원래 (시프트)
    let row0: Vec<u16> = table
        .cells
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.col)
        .collect();
    assert_eq!(row0, vec![0, 1, 2]);
}

#[test]
fn test_insert_column_with_merged_cell() {
    let mut table = make_table(2, 3);
    // (0,0) 셀을 col_span=2로 병합
    table.cells[0].col_span = 2;
    table.cells[0].width = 7200;
    // 병합된 (1,0) 셀 제거
    table.cells.retain(|c| !(c.col == 1 && c.row == 0));

    // 열 0 오른쪽에 삽입 → 병합 셀이 삽입 지점을 걸침
    table.insert_column(0, true).unwrap();

    assert_eq!(table.col_count, 4);
    // (0,0) 셀의 col_span이 3으로 확장
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 3);
    // 행 0의 새 열에는 셀 없음 (병합에 의해 커버)
    // 행 1의 새 열에 새 셀 생성
    let row1_new: Vec<_> = table
        .cells
        .iter()
        .filter(|c| c.row == 1 && c.col == 1)
        .collect();
    assert_eq!(row1_new.len(), 1);
}

#[test]
fn test_insert_column_out_of_bounds() {
    let mut table = make_table(2, 2);
    assert!(table.insert_column(5, true).is_err());
}

// === merge_cells 테스트 ===

#[test]
fn test_merge_cells_2x2_full() {
    let mut table = make_table(2, 2);

    table.merge_cells(0, 0, 1, 1).unwrap();

    // 비주 셀 제거 → 주 셀 1개만 남음
    assert_eq!(table.cells.len(), 1);
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 2);
    assert_eq!(merged.row_span, 2);
    assert_eq!(merged.width, 7200); // 3600 * 2
    assert_eq!(merged.height, 2000); // 1000 * 2
                                     // row_sizes 갱신: 각 행에 셀 1개(행0만 주 셀), 행1은 0개
    assert_eq!(table.row_sizes, vec![1, 0]);
}

#[test]
fn test_merge_cells_partial_row() {
    let mut table = make_table(2, 3);

    // 첫 행의 열 0~1 병합
    table.merge_cells(0, 0, 0, 1).unwrap();

    assert_eq!(table.cells.len(), 5); // 비주 셀 1개 제거
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 2);
    assert_eq!(merged.row_span, 1);
    // row_sizes: 행0=2셀(병합1+col2), 행1=3셀
    assert_eq!(table.row_sizes, vec![2, 3]);
}

#[test]
fn test_merge_cells_partial_column() {
    let mut table = make_table(3, 2);

    // 열 0의 행 0~1 병합
    table.merge_cells(0, 0, 1, 0).unwrap();

    assert_eq!(table.cells.len(), 5); // 비주 셀 1개 제거
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 1);
    assert_eq!(merged.row_span, 2);
    // row_sizes: 행0=2셀(병합1+col1), 행1=1셀(col1만), 행2=2셀
    assert_eq!(table.row_sizes, vec![2, 1, 2]);
}

#[test]
fn test_merge_cells_invalid_range() {
    let mut table = make_table(2, 2);
    assert!(table.merge_cells(1, 0, 0, 0).is_err()); // start > end
    assert!(table.merge_cells(0, 0, 5, 5).is_err()); // 범위 초과
}

#[test]
fn test_merge_cells_overlapping_span() {
    let mut table = make_table(3, 3);
    // (0,0)을 col_span=2로 병합
    table.cells[0].col_span = 2;
    table.cells[0].width = 7200;
    table.cells.retain(|c| !(c.col == 1 && c.row == 0));

    // (0,0)~(0,2) 병합 시도 → 기존 병합이 범위 안에 있으므로 성공
    table.merge_cells(0, 0, 0, 2).unwrap();
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 3);
}

#[test]
fn test_insert_row_single_row() {
    let mut table = make_table(1, 3);

    table.insert_row(0, true).unwrap();

    assert_eq!(table.row_count, 2);
    assert_eq!(table.cells.len(), 6);
}

#[test]
fn test_insert_column_single_column() {
    let mut table = make_table(3, 1);

    table.insert_column(0, true).unwrap();

    assert_eq!(table.col_count, 2);
    assert_eq!(table.cells.len(), 6);
}

// === split_cell 테스트 ===

#[test]
fn test_split_cell_2x2_full() {
    let mut table = make_table(2, 2);
    table.merge_cells(0, 0, 1, 1).unwrap();
    assert_eq!(table.cells.len(), 1);

    // 나누기
    table.split_cell(0, 0).unwrap();

    assert_eq!(table.cells.len(), 4);
    // 모든 셀이 col_span=1, row_span=1
    for cell in &table.cells {
        assert_eq!(cell.col_span, 1);
        assert_eq!(cell.row_span, 1);
    }
    // row_sizes 복원
    assert_eq!(table.row_sizes, vec![2, 2]);
}

#[test]
fn test_split_cell_partial_row() {
    let mut table = make_table(2, 3);
    table.merge_cells(0, 0, 0, 1).unwrap();
    assert_eq!(table.cells.len(), 5);

    table.split_cell(0, 0).unwrap();

    assert_eq!(table.cells.len(), 6);
    let cell = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(cell.col_span, 1);
    assert_eq!(cell.row_span, 1);
    // row_sizes 복원: 각 행 3개 셀
    assert_eq!(table.row_sizes, vec![3, 3]);
}

#[test]
fn test_split_cell_partial_column() {
    let mut table = make_table(3, 2);
    table.merge_cells(0, 0, 1, 0).unwrap();
    assert_eq!(table.cells.len(), 5);

    table.split_cell(0, 0).unwrap();

    assert_eq!(table.cells.len(), 6);
    let cell = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(cell.col_span, 1);
    assert_eq!(cell.row_span, 1);
    // row_sizes 복원: 각 행 2개 셀
    assert_eq!(table.row_sizes, vec![2, 2, 2]);
}

#[test]
fn test_split_cell_not_merged() {
    let mut table = make_table(2, 2);
    // 병합되지 않은 셀 나누기 → 에러
    assert!(table.split_cell(0, 0).is_err());
}

#[test]
fn test_split_cell_width_distribution() {
    let mut table = make_table(2, 3);
    // 열 0~1 병합 (폭: 3600 + 3600 = 7200)
    table.merge_cells(0, 0, 0, 1).unwrap();

    table.split_cell(0, 0).unwrap();

    // 다른 행에 col_span=1 셀이 있으므로 실제 열폭 사용
    let cell0 = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    let cell1 = table
        .cells
        .iter()
        .find(|c| c.col == 1 && c.row == 0)
        .unwrap();
    assert_eq!(cell0.width, 3600);
    assert_eq!(cell1.width, 3600);
}

// === delete_row 테스트 ===

#[test]
fn test_delete_row_basic() {
    let mut table = make_table(3, 2);
    assert_eq!(table.cells.len(), 6);

    table.delete_row(1).unwrap();

    assert_eq!(table.row_count, 2);
    assert_eq!(table.cells.len(), 4);
    // 행 0: 유지, 행 1: 원래 행 2가 시프트
    assert_eq!(table.cells[0].row, 0);
    assert_eq!(table.cells[1].row, 0);
    assert_eq!(table.cells[2].row, 1);
    assert_eq!(table.cells[3].row, 1);
}

#[test]
fn test_delete_row_first() {
    let mut table = make_table(2, 2);

    table.delete_row(0).unwrap();

    assert_eq!(table.row_count, 1);
    assert_eq!(table.cells.len(), 2);
    assert_eq!(table.cells[0].row, 0);
    assert_eq!(table.cells[1].row, 0);
}

#[test]
fn test_delete_row_last() {
    let mut table = make_table(3, 2);

    table.delete_row(2).unwrap();

    assert_eq!(table.row_count, 2);
    assert_eq!(table.cells.len(), 4);
}

#[test]
fn test_delete_row_with_merged_cell() {
    let mut table = make_table(3, 2);
    // (0,0) 셀을 row_span=2로 병합
    table.cells[0].row_span = 2;
    table.cells[0].height = 2000;
    // 병합된 (0,1) 셀 제거
    table.cells.retain(|c| !(c.col == 0 && c.row == 1));
    assert_eq!(table.cells.len(), 5);

    // 행 1 삭제 → 병합 셀 row_span 축소
    table.delete_row(1).unwrap();

    assert_eq!(table.row_count, 2);
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.row_span, 1);
}

#[test]
fn test_delete_row_merged_cell_anchor() {
    let mut table = make_table(3, 2);
    // (0,0) 셀을 row_span=3으로 병합 (전체 열)
    table.cells[0].row_span = 3;
    table.cells[0].height = 3000;
    table
        .cells
        .retain(|c| !(c.col == 0 && (c.row == 1 || c.row == 2)));
    assert_eq!(table.cells.len(), 4);

    // 행 0(앵커 행) 삭제 → 병합 셀이 다음 행으로 이동, row_span 축소
    table.delete_row(0).unwrap();

    assert_eq!(table.row_count, 2);
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.row_span, 2);
}

#[test]
fn test_delete_row_single_row_error() {
    let mut table = make_table(1, 2);
    assert!(table.delete_row(0).is_err());
}

#[test]
fn test_delete_row_out_of_bounds() {
    let mut table = make_table(2, 2);
    assert!(table.delete_row(5).is_err());
}

// === delete_column 테스트 ===

#[test]
fn test_delete_column_basic() {
    let mut table = make_table(2, 3);
    assert_eq!(table.cells.len(), 6);

    table.delete_column(1).unwrap();

    assert_eq!(table.col_count, 2);
    assert_eq!(table.cells.len(), 4);
    // 열 0: 유지, 열 1: 원래 열 2가 시프트
    let row0: Vec<u16> = table
        .cells
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.col)
        .collect();
    assert_eq!(row0, vec![0, 1]);
}

#[test]
fn test_delete_column_first() {
    let mut table = make_table(2, 2);

    table.delete_column(0).unwrap();

    assert_eq!(table.col_count, 1);
    assert_eq!(table.cells.len(), 2);
    assert_eq!(table.cells[0].col, 0);
    assert_eq!(table.cells[1].col, 0);
}

#[test]
fn test_delete_column_last() {
    let mut table = make_table(2, 3);

    table.delete_column(2).unwrap();

    assert_eq!(table.col_count, 2);
    assert_eq!(table.cells.len(), 4);
}

#[test]
fn test_delete_column_with_merged_cell() {
    let mut table = make_table(2, 3);
    // (0,0) 셀을 col_span=2로 병합
    table.cells[0].col_span = 2;
    table.cells[0].width = 7200;
    // 병합된 (1,0) 셀 제거
    table.cells.retain(|c| !(c.col == 1 && c.row == 0));
    assert_eq!(table.cells.len(), 5);

    // 열 1 삭제 → 병합 셀 col_span 축소
    table.delete_column(1).unwrap();

    assert_eq!(table.col_count, 2);
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 1);
}

#[test]
fn test_delete_column_merged_cell_anchor() {
    let mut table = make_table(2, 3);
    // (0,0) 셀을 col_span=3으로 병합 (전체 행)
    table.cells[0].col_span = 3;
    table.cells[0].width = 10800;
    table
        .cells
        .retain(|c| !(c.row == 0 && (c.col == 1 || c.col == 2)));
    assert_eq!(table.cells.len(), 4);

    // 열 0(앵커 열) 삭제 → 병합 셀 col_span 축소
    table.delete_column(0).unwrap();

    assert_eq!(table.col_count, 2);
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 2);
}

#[test]
fn test_delete_column_single_column_error() {
    let mut table = make_table(2, 1);
    assert!(table.delete_column(0).is_err());
}

#[test]
fn test_delete_column_out_of_bounds() {
    let mut table = make_table(2, 2);
    assert!(table.delete_column(5).is_err());
}

// === cell_grid / cell_at 테스트 ===

#[test]
fn test_rebuild_grid_basic() {
    let table = make_table(2, 3);
    // 2×3 표: 6개 셀, 각각 (row, col) 순서대로
    assert_eq!(table.cell_grid.len(), 6);
    for r in 0..2u16 {
        for c in 0..3u16 {
            let gi = (r as usize) * 3 + (c as usize);
            let idx = table.cell_grid[gi].expect("grid entry should exist");
            assert_eq!(table.cells[idx].row, r);
            assert_eq!(table.cells[idx].col, c);
        }
    }
}

#[test]
fn test_rebuild_grid_merged() {
    let mut table = make_table(3, 3);
    // (0,0) 셀을 2×2 병합
    table.merge_cells(0, 0, 1, 1).unwrap();
    // 병합 영역 전체가 같은 셀 인덱스를 가리켜야 함
    let anchor_idx = table.cell_grid[0].unwrap(); // (0,0)
    assert_eq!(table.cell_grid[1].unwrap(), anchor_idx); // (0,1)
    assert_eq!(table.cell_grid[3].unwrap(), anchor_idx); // (1,0)
    assert_eq!(table.cell_grid[4].unwrap(), anchor_idx); // (1,1)
                                                         // 앵커 셀 확인
    let anchor = &table.cells[anchor_idx];
    assert_eq!(anchor.row, 0);
    assert_eq!(anchor.col, 0);
    assert_eq!(anchor.col_span, 2);
    assert_eq!(anchor.row_span, 2);
}

#[test]
fn test_cell_at_basic() {
    let table = make_table(2, 3);
    for r in 0..2u16 {
        for c in 0..3u16 {
            let cell = table.cell_at(r, c).expect("cell should exist");
            assert_eq!(cell.row, r);
            assert_eq!(cell.col, c);
        }
    }
}

#[test]
fn test_cell_at_out_of_bounds() {
    let table = make_table(2, 3);
    assert!(table.cell_at(5, 0).is_none());
    assert!(table.cell_at(0, 10).is_none());
}

#[test]
fn test_cell_at_merged_span() {
    let mut table = make_table(3, 3);
    table.merge_cells(0, 0, 1, 1).unwrap();
    // 비앵커 좌표에서도 앵커 셀 반환
    let anchor = table.cell_at(0, 0).unwrap();
    let from_span = table.cell_at(1, 1).unwrap();
    assert!(std::ptr::eq(anchor, from_span));
}

#[test]
fn test_cell_index_at_basic() {
    let table = make_table(2, 3);
    for r in 0..2u16 {
        for c in 0..3u16 {
            let idx = table.cell_index_at(r, c).expect("should find index");
            assert_eq!(table.cells[idx].row, r);
            assert_eq!(table.cells[idx].col, c);
        }
    }
}

#[test]
fn test_edit_ops_rebuild_grid() {
    // insert_row 후 grid 정합성 확인
    let mut table = make_table(2, 2);
    table.insert_row(0, true).unwrap();
    assert_eq!(table.cell_grid.len(), 3 * 2); // 3행 × 2열
    for r in 0..3u16 {
        for c in 0..2u16 {
            let cell = table
                .cell_at(r, c)
                .expect("cell should exist after insert_row");
            assert_eq!(cell.row, r);
            assert_eq!(cell.col, c);
        }
    }

    // delete_column 후 grid 정합성 확인
    table.delete_column(1).unwrap();
    assert_eq!(table.cell_grid.len(), 3 * 1); // 3행 × 1열
    for r in 0..3u16 {
        let cell = table
            .cell_at(r, 0)
            .expect("cell should exist after delete_column");
        assert_eq!(cell.row, r);
        assert_eq!(cell.col, 0);
    }
}

// === split_cell_into 테스트 ===

#[test]
fn test_split_cell_into_1x2() {
    // 2×2 표, (0,0)을 1줄×2칸으로 분할
    let mut table = make_table(2, 2);
    table.split_cell_into(0, 0, 1, 2, true, false).unwrap();

    assert_eq!(table.col_count, 3); // 2+1
    assert_eq!(table.row_count, 2); // 변경 없음
                                    // 서브셀 2개
    let c00 = table.cell_at(0, 0).unwrap();
    let c01 = table.cell_at(0, 1).unwrap();
    assert_eq!(c00.col_span, 1);
    assert_eq!(c01.col_span, 1);
    assert_eq!(c00.width + c01.width, 3600); // 원래 폭 보존
                                             // (0,2): 원래 (0,1)이 이동한 셀
    let c02 = table.cell_at(0, 2).unwrap();
    assert_eq!(c02.col, 2);
    // (1,0): 같은 열 → col_span=2로 확장
    let c10 = table.cell_at(1, 0).unwrap();
    assert_eq!(c10.col_span, 2);
}

#[test]
fn test_split_cell_into_2x1() {
    // 2×2 표, (0,0)을 2줄×1칸으로 분할
    let mut table = make_table(2, 2);
    table.split_cell_into(0, 0, 2, 1, true, false).unwrap();

    assert_eq!(table.row_count, 3); // 2+1
    assert_eq!(table.col_count, 2); // 변경 없음
    let c00 = table.cell_at(0, 0).unwrap();
    let c10 = table.cell_at(1, 0).unwrap();
    assert_eq!(c00.row_span, 1);
    assert_eq!(c10.row_span, 1);
    assert_eq!(c00.height + c10.height, 1000); // 원래 높이 보존
                                               // (0,1): 같은 행 → row_span=2로 확장
    let c01 = table.cell_at(0, 1).unwrap();
    assert_eq!(c01.row_span, 2);
}

#[test]
fn test_split_cell_into_2x2() {
    // 3×3 표, (1,1)을 2줄×2칸으로 분할
    let mut table = make_table(3, 3);
    table.split_cell_into(1, 1, 2, 2, true, false).unwrap();

    assert_eq!(table.col_count, 4); // 3+1
    assert_eq!(table.row_count, 4); // 3+1
                                    // 4개 서브셀 확인
    for r in 1..=2u16 {
        for c in 1..=2u16 {
            let cell = table.cell_at(r, c).unwrap();
            assert_eq!(cell.col_span, 1);
            assert_eq!(cell.row_span, 1);
        }
    }
    // (0,1): 같은 열 → col_span=2
    let c01 = table.cell_at(0, 1).unwrap();
    assert_eq!(c01.col_span, 2);
    // (1,0): 같은 행 → row_span=2
    let c10 = table.cell_at(1, 0).unwrap();
    assert_eq!(c10.row_span, 2);
}

#[test]
fn test_split_cell_into_noop() {
    let mut table = make_table(2, 2);
    table.split_cell_into(0, 0, 1, 1, true, false).unwrap();
    assert_eq!(table.col_count, 2);
    assert_eq!(table.row_count, 2);
    assert_eq!(table.cells.len(), 4);
}

#[test]
fn test_split_cell_into_width_distribution() {
    // 1×1 표 (단일 셀, 폭=7200), 1줄×3칸으로 분할
    let mut table = make_table(1, 1);
    // 기본 폭 3600 → 7200으로 변경
    table.cells[0].width = 7200;
    table.split_cell_into(0, 0, 1, 3, true, false).unwrap();

    assert_eq!(table.col_count, 3);
    let w0 = table.cell_at(0, 0).unwrap().width;
    let w1 = table.cell_at(0, 1).unwrap().width;
    let w2 = table.cell_at(0, 2).unwrap().width;
    assert_eq!(w0 + w1 + w2, 7200);
    assert_eq!(w0, 2400);
    assert_eq!(w1, 2400);
    assert_eq!(w2, 2400);
}

#[test]
fn test_split_cell_into_merged_merge_first() {
    // 2×3 표, (0,0)-(0,1) 병합 후 merge_first로 1줄×3칸 분할
    let mut table = make_table(2, 3);
    table.merge_cells(0, 0, 0, 1).unwrap();

    // 병합된 셀: col_span=2
    let merged = table
        .cells
        .iter()
        .find(|c| c.col == 0 && c.row == 0)
        .unwrap();
    assert_eq!(merged.col_span, 2);

    table.split_cell_into(0, 0, 1, 3, true, true).unwrap();

    // 병합 해제 후 (0,0) span=1x1 → 1×3 분할: extra_cols=2
    assert_eq!(table.col_count, 5); // 3 + 2
    let c00 = table.cell_at(0, 0).unwrap();
    assert_eq!(c00.col_span, 1);
}

#[test]
fn test_split_cells_in_range_2x2_into_1x2() {
    // 3×3 표, (0,0)~(1,1) 범위의 4개 셀을 각각 1줄×2칸으로 분할
    let mut table = make_table(3, 3);
    table.split_cells_in_range(0, 0, 1, 1, 1, 2, true).unwrap();

    // 열 0, 1 각각 2분할 → extra_cols = 2
    assert_eq!(table.col_count, 5); // 3 + 2
    assert_eq!(table.row_count, 3);

    // (0,0)과 (0,1): 분할된 서브셀 (span=1)
    let c00 = table.cell_at(0, 0).unwrap();
    let c01 = table.cell_at(0, 1).unwrap();
    assert_eq!(c00.col_span, 1);
    assert_eq!(c01.col_span, 1);

    // (2,0): 선택 범위 밖 → col_span=2 (열 0의 서브열 흡수)
    let c20 = table.cell_at(2, 0).unwrap();
    assert_eq!(c20.col_span, 2);

    // (0,4): 원래 (0,2)가 시프트된 셀
    let c04 = table.cell_at(0, 4).unwrap();
    assert_eq!(c04.col_span, 1);
}

#[test]
fn test_split_cells_in_range_single_cell() {
    // 범위 = 단일 셀: split_cell_into와 동일 동작
    let mut table = make_table(2, 2);
    table.split_cells_in_range(0, 0, 0, 0, 1, 3, true).unwrap();
    assert_eq!(table.col_count, 4); // 2 + 2
}

// === v353: 표 기하 — 폭 공식과 표시 높이 ===

/// 표시 높이를 셀 높이 합과 다르게 준 N×M 표 (make_table 셀은 3600×1000).
fn make_display_table(rows: u16, cols: u16, display_height: HwpUnit) -> Table {
    let mut table = make_table(rows, cols);
    table.common.width = 3600 * cols as HwpUnit;
    table.common.height = display_height;
    table
}

#[test]
fn declared_width_counts_cell_spacing() {
    // Σ열 폭 + (열 수 + 1) × 셀 간격
    let mut table = make_table(2, 3);
    table.cell_spacing = 100;
    assert_eq!(table.declared_width_from_columns(), 3 * 3600 + 4 * 100);
    table.update_ctrl_dimensions();
    assert_eq!(table.common.width, 3 * 3600 + 4 * 100);
}

#[test]
fn stretched_row_heights_scale_to_display_height() {
    let table = make_display_table(2, 1, 5000);
    assert_eq!(table.stretched_row_heights(), Some(vec![2500, 2500]));
    // 반올림 오차는 마지막 행이 흡수한다: 3333 + 3333 + 3334 = 10000
    let table = make_display_table(3, 1, 10000);
    assert_eq!(table.stretched_row_heights(), Some(vec![3333, 3333, 3334]));
    // 표시 높이가 셀 합 이하면 늘릴 것이 없다
    assert_eq!(make_display_table(2, 1, 2000).stretched_row_heights(), None);
    assert_eq!(make_display_table(2, 1, 1500).stretched_row_heights(), None);
}

#[test]
fn insert_row_adds_display_row_height_to_display_height() {
    // 행 높이 [1000, 1000], 표시 높이 5000 → 표시 기준 행 높이 [2500, 2500]
    let mut table = make_display_table(2, 1, 5000);
    table.insert_row(0, true).unwrap();
    assert_eq!(table.common.height, 5000 + 2500);
    assert_eq!(table.common.width, 3600, "행 편집은 선언 폭을 바꾸지 않는다");
}

#[test]
fn insert_row_without_stretch_follows_cell_sum() {
    // 표시 높이 = 셀 합이면 새 행의 셀 높이만큼 늘어난다
    let mut table = make_display_table(2, 1, 2000);
    table.insert_row(1, true).unwrap();
    assert_eq!(table.common.height, 3000);
    // 표시 높이 < 셀 합인 표도 셀 합 아래로는 두지 않는다
    let mut table = make_display_table(2, 1, 1500);
    table.insert_row(0, false).unwrap();
    assert_eq!(table.common.height, 3000);
}

#[test]
fn delete_row_subtracts_display_row_height() {
    // 행 높이 [1000, 1000, 1000], 표시 높이 9000 → 표시 기준 행 높이 [3000, 3000, 3000]
    let mut table = make_display_table(3, 1, 9000);
    table.delete_row(1).unwrap();
    assert_eq!(table.common.height, 6000);
    assert_eq!(table.common.width, 3600);
}

#[test]
fn delete_row_never_goes_below_cell_sum() {
    // 표시 높이 1500 < 셀 합 2000: 1500 − 1000 = 500 이지만 남은 셀 합 1000 아래로는 안 간다
    let mut table = make_display_table(2, 1, 1500);
    table.delete_row(0).unwrap();
    assert_eq!(table.common.height, 1000);
}

#[test]
fn column_edits_recompute_width_and_keep_display_height() {
    let mut table = make_display_table(2, 2, 5000);
    table.cell_spacing = 100;
    table.common.width = 2 * 3600 + 3 * 100;

    table.insert_column(0, true).unwrap();
    assert_eq!(table.common.width, 3 * 3600 + 4 * 100);
    assert_eq!(table.common.height, 5000);

    table.delete_column(2).unwrap();
    assert_eq!(table.common.width, 2 * 3600 + 3 * 100);
    assert_eq!(table.common.height, 5000);
}

#[test]
fn content_edit_refresh_only_grows_display_height() {
    let mut table = make_display_table(2, 1, 5000);
    table.common.width = 9999;

    // 셀 합 6000 > 표시 높이 5000 → 늘린다
    for cell in &mut table.cells {
        cell.height = 3000;
    }
    table.refresh_ctrl_dimensions_after_content_edit(false);
    assert_eq!(table.common.height, 6000);
    assert_eq!(table.common.width, 9999, "열 폭이 그대로면 선언 폭을 지킨다");

    // 셀 합이 줄어도 표시 높이는 줄이지 않는다
    for cell in &mut table.cells {
        cell.height = 500;
    }
    table.refresh_ctrl_dimensions_after_content_edit(false);
    assert_eq!(table.common.height, 6000);

    // 열 폭이 바뀌었으면 폭을 공식으로 다시 계산한다
    table.refresh_ctrl_dimensions_after_content_edit(true);
    assert_eq!(table.common.width, 3600);
}

#[test]
fn set_column_widths_applies_spacing_to_merged_cells() {
    let mut table = make_display_table(2, 3, 5000);
    table.cell_spacing = 100;
    table.merge_cells(0, 0, 0, 1).unwrap();

    table.set_column_widths(&[1000, 2000, 3000]).unwrap();

    let merged = table.cell_at(0, 0).unwrap();
    assert_eq!(merged.width, 1000 + 2000 + 100, "병합 셀 = 덮는 열 합 + (span−1)×간격");
    assert_eq!(table.cell_at(1, 1).unwrap().width, 2000);
    assert_eq!(table.cell_at(1, 2).unwrap().width, 3000);
    assert_eq!(table.common.width, 6000 + 4 * 100);
    assert_eq!(table.common.height, 5000);
    assert!(table.set_column_widths(&[1000]).is_err());
}

#[test]
fn split_cell_into_keeps_outer_size() {
    let mut table = make_display_table(2, 2, 5000);
    table.common.width = 7200;
    table.split_cell_into(0, 0, 1, 2, false, false).unwrap();
    assert_eq!(table.common.width, 7200);
    assert_eq!(table.common.height, 5000);
}

#[test]
fn row_edit_keeps_hwp5_ctrl_header_layout() {
    // HWP5 파서가 담는 CTRL_HEADER 와 같은 바이트로 시작해, 편집 뒤 다시 파싱해 본다.
    let mut table = make_display_table(2, 1, 5000);
    let mut common = table.common.clone();
    common.horizontal_offset = 777;
    common.vertical_offset = 555;
    table.raw_ctrl_data =
        crate::document_core::converters::common_obj_attr_writer::serialize_common_obj_attr(
            &common,
        );

    table.insert_row(0, true).unwrap();

    let reparsed = crate::parser::control::parse_common_obj_attr(&table.raw_ctrl_data);
    assert_eq!(reparsed.horizontal_offset, 777, "가로 오프셋에 폭을 덮어쓰지 않는다");
    assert_eq!(reparsed.vertical_offset, 555);
    assert_eq!(reparsed.width, 3600);
    assert_eq!(reparsed.height, 7500);
}

#[test]
fn cell_flags_live_in_list_header_bits() {
    let mut cell = Cell::new_empty(0, 0, 1000, 1000, 1);
    cell.list_header_width_ref = 0x04; // 제목 셀 비트는 지켜야 한다
    cell.set_cell_protect(true);
    cell.set_editable_in_form(true);
    assert!(cell.cell_protect() && cell.editable_in_form());
    assert_eq!(cell.list_header_width_ref, 0x04 | CELL_FLAG_PROTECT | CELL_FLAG_EDITABLE_IN_FORM);
    cell.set_cell_protect(false);
    assert!(!cell.cell_protect());
    assert_eq!(cell.list_header_width_ref, 0x04 | CELL_FLAG_EDITABLE_IN_FORM);
}

#[test]
fn inserted_row_copies_template_line_wrap() {
    let mut table = make_table(2, 1);
    for cell in &mut table.cells {
        cell.line_wrap = CELL_LINE_WRAP_SQUEEZE;
        cell.dirty_flag = true;
    }
    table.insert_row(0, true).unwrap();
    let inserted = table.cell_at(1, 0).unwrap();
    assert_eq!(inserted.line_wrap, CELL_LINE_WRAP_SQUEEZE);
    assert!(!inserted.dirty_flag);
}
