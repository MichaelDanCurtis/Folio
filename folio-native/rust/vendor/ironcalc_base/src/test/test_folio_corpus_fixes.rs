#![allow(clippy::unwrap_used)]
//! Folio: behaviours from the Office corpus (Excel-correct results part 2).

use crate::test::util::new_empty_model;

#[test]
fn qualified_names_ref_errors_and_other_sheet_intersection() {
    let mut model = new_empty_model();
    model.add_sheet("Defines").unwrap();
    model._set("Defines!A1", "Test A1");
    model._set("Defines!B5", "row five");
    model
        .new_defined_name("NR_To_A1", Some(1), "Defines!$A$1")
        .unwrap();
    model._set("A1", "=Defines!NR_To_A1");
    model._set("A2", "=Defines!Missing");
    model._set("A3", "='Defines'!#REF!");
    model._set("C5", "=@Defines!B1:B10");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "Test A1");
    assert_eq!(model._get_formula("A1"), "=Defines!NR_To_A1");
    assert_eq!(model._get_text("A2"), "#NAME?");
    assert_eq!(model._get_text("A3"), "#REF!");
    assert_eq!(model._get_text("C5"), "row five");
}

#[test]
fn mina_maxa_and_matrix_blanks() {
    let mut model = new_empty_model();
    model._set("B1", "TRUE");
    model._set("B2", "534");
    model._set("B3", "2");
    model._set("C1", "text");
    model._set("A1", "=MINA(B1,B2,B3)");
    model._set("A2", "=MAXA(C1,C1)");
    model._set("A3", "=MDETERM(E1:F2)");
    model._set("A4", "=MINVERSE(E1:F2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "1");
    assert_eq!(model._get_text("A2"), "0");
    assert_eq!(model._get_text("A3"), "#VALUE!");
    assert_eq!(model._get_text("A4"), "#VALUE!");
}

#[test]
fn text_of_text_and_logical_values_uses_the_text_section() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXT(\"jello\",\"-@-\")");
    model._set("A2", "=TEXT(TRUE,\";;;@-\")");
    model._set("A3", "=TEXT(12.3,\"£000.00\")");
    model._set("A4", "=TEXT(123456.789,\"0.0E+0\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "-jello-");
    assert_eq!(model._get_text("A2"), "TRUE-");
    assert_eq!(model._get_text("A3"), "£012.30");
    assert_eq!(model._get_text("A4"), "1.2E+5");
}

#[test]
fn text_codes_from_cells_starting_with_a_minus() {
    let mut model = new_empty_model();
    // Folio passes text from a file with a leading apostrophe.
    model._set("B1", "'-@-");
    model._set("C1", "jello");
    model._set("A1", "=TEXT(C1,CONCATENATE(\";;;\",B1))");
    model.evaluate();
    assert_eq!(model._get_text("B1"), "-@-");
    assert_eq!(model._get_text("A1"), "-jello-");
}
