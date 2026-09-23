#![allow(clippy::unwrap_used)]
use crate::{cell::CellValue, test::util::new_empty_model};
#[test]
fn validation_expressions_keep_context_bounds_types_and_cell_contents() {
    let mut m = new_empty_model();
    m._set("A1", "3");
    m._set("A2", "=A1+2");
    m._set("B1", "=A2*2");
    m.evaluate();
    let before = m.to_bytes();
    assert_eq!(
        m.evaluate_expression_cells("ROW()+COLUMN()", 0, 7, 2, 10)
            .unwrap(),
        vec![vec![CellValue::Number(9.0)]]
    );
    assert_eq!(
        m.evaluate_expression_cells("A1:A2", 0, 7, 2, 10).unwrap(),
        vec![vec![CellValue::Number(3.0)], vec![CellValue::Number(5.0)]]
    );
    assert_eq!(
        m.evaluate_expression_cells("{TRUE,\"hello\",2}", 0, 7, 2, 10)
            .unwrap(),
        vec![vec![
            CellValue::Boolean(true),
            CellValue::String("hello".into()),
            CellValue::Number(2.0)
        ]]
    );
    assert!(m.evaluate_expression_cells("1/0", 0, 1, 1, 10).is_err());
    assert!(m.evaluate_expression_cells("A1:A100", 0, 1, 1, 10).is_err());
    assert!(m.evaluate_expression_cells("1", 0, 0, 1, 10).is_err());
    assert!(m.evaluate_expression_cells("1", 4, 1, 1, 10).is_err());
    assert_eq!(before, m.to_bytes());
    assert_eq!(m._get_text("B1"), "10");
}
