#![allow(clippy::unwrap_used)]
//! Folio: the functions added in functions/folio (X08.03).

use crate::test::util::new_empty_model;

fn cells(model: &crate::Model, refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| model._get_text(r)).collect()
}

#[test]
fn aggregate_functions_and_options() {
    let mut model = new_empty_model();
    for (i, v) in ["10", "20", "=1/0", "40", "text", "30"].iter().enumerate() {
        model._set(&format!("A{}", i + 1), v);
    }
    model._set("A7", "=SUBTOTAL(9,A1:A2)");
    model.set_row_hidden(0, 4, true).unwrap();
    model._set("B1", "=AGGREGATE(9,6,A1:A7)");
    model._set("B2", "=AGGREGATE(9,7,A1:A7)");
    model._set("B3", "=AGGREGATE(9,4,A1:A7)");
    model._set("B4", "=AGGREGATE(14,6,A1:A6,2)");
    model._set("B5", "=AGGREGATE(15,6,A1:A6,1)");
    model._set("B6", "=AGGREGATE(2,4,A1:A6)");
    model._set("B7", "=AGGREGATE(3,6,A1:A6)");
    model._set("B8", "=AGGREGATE(1,6,A1:A6)");
    model._set("B9", "=AGGREGATE(12,6,A1:A6)");
    model._set("B10", "=AGGREGATE(16,6,A1:A6,0.5)");
    model._set("B11", "=AGGREGATE(17,6,A1:A6,1)");
    model._set("B12", "=AGGREGATE(4,3,A1:A7)");
    model._set("B13", "=AGGREGATE(14,6,A1:A6/(A1:A6>15),1)");
    model._set("B14", "=AGGREGATE(20,6,A1:A6)");
    model._set("B15", "=AGGREGATE(14,6,A1:A6)");
    model._set("B16", "=AGGREGATE(9,6,A1:A6,A1)");
    model._set("B17", "=AGGREGATE(13,6,A1:A6)");
    model.evaluate();
    assert_eq!(
        cells(
            &model,
            &["B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9"]
        ),
        ["130", "90", "#DIV/0!", "30", "10", "4", "5", "25", "25"]
    );
    assert_eq!(
        cells(
            &model,
            &["B10", "B11", "B12", "B13", "B14", "B15", "B16", "B17"]
        ),
        ["25", "17.5", "30", "40", "#VALUE!", "#VALUE!", "110", "#N/A"]
    );
}

#[test]
fn text_functions() {
    let mut model = new_empty_model();
    model._set("A1", "=BAHTTEXT(1234.5)");
    model._set("A2", "=DBCS(\"ABC 1\")");
    model._set("A3", "=JIS(\"ｶﾞ\")");
    model._set("A4", "=ENCODEURL(\"a b&c\")");
    model._set("A5", "=HYPERLINK(\"https://example.com\",\"Example\")");
    model._set("A6", "=HYPERLINK(\"https://example.com\")");
    model._set("A7", "=HYPERLINK(\"x\",42)+1");
    model._set("A8", "=EUROCONVERT(100,\"DEM\",\"EUR\")");
    model._set("A9", "=EUROCONVERT(1,\"FRF\",\"DEM\",FALSE,3)");
    model._set("A10", "=EUROCONVERT(1,\"USD\",\"EUR\")");
    model._set("B1", "東京");
    model._set("A11", "=PHONETIC(B1:B3)");
    model._set("A12", "=PHONETIC(\"x\")");
    model.evaluate();
    assert_eq!(
        cells(&model, &["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8"]),
        [
            "หนึ่งพันสองร้อยสามสิบสี่บาทห้าสิบสตางค์",
            "ＡＢＣ\u{3000}１",
            "ガ",
            "a%20b%26c",
            "Example",
            "https://example.com",
            "43",
            "51.13"
        ]
    );
    assert_eq!(model._get_text("A9"), "0.3");
    assert_eq!(
        cells(&model, &["A10", "A11", "A12"]),
        ["#VALUE!", "東京", "#N/A"]
    );
    assert_eq!(model._get_formula("A4"), "=ENCODEURL(\"a b&c\")");
}

#[test]
fn filterxml_spills_matches_as_numbers_and_text() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "<r><i><n>Pen</n><p>1.5</p></i><i><n>Ink</n><p>4</p></i></r>",
    );
    model._set("B1", "=FILTERXML(A1,\"//n\")");
    model._set("C1", "=FILTERXML(A1,\"//p\")");
    model._set("D1", "=SUM(FILTERXML(A1,\"//p\"))");
    model._set("E1", "=FILTERXML(A1,\"//i[p>2]/n\")");
    model._set("F1", "=FILTERXML(A1,\"//missing\")");
    model._set("G1", "=FILTERXML(\"<a>\",\"//a\")");
    model.evaluate();
    assert_eq!(
        cells(&model, &["B1", "B2", "C1", "C2"]),
        ["Pen", "Ink", "1.5", "4"]
    );
    assert_eq!(
        cells(&model, &["D1", "E1", "F1", "G1"]),
        ["5.5", "Ink", "#VALUE!", "#VALUE!"]
    );
}

#[test]
fn function_names_as_values() {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("B1", "2");
    model._set("A2", "3");
    model._set("B2", "4");
    model._set("C1", "=BYROW(A1:B2,SUM)");
    model._set("D1", "=BYCOL(A1:B2,MAX)");
    model._set("D3", "=REDUCE(0,A1:B2,LAMBDA(a,b,a+b))");
    model._set("D4", "=MAP(A1:A2,SQRT)");
    model._set("D6", "=NOPE(1)");
    model._set("D8", "=BYROW(A1:B1,_xleta.SUM)");
    model._set("D7", "=COUNT({1,\"a\",2,TRUE})");
    model.evaluate();
    assert_eq!(
        cells(&model, &["C1", "C2", "D1", "E1"]),
        ["3", "7", "3", "4"]
    );
    assert_eq!(model._get_text("D3"), "10");
    assert_eq!(model._get_text("D4"), "1");
    assert_eq!(model._get_text("D6"), "#NAME?");
    assert_eq!(model._get_text("D7"), "2");
    assert_eq!(model._get_text("D8"), "3");
    assert_eq!(model._get_formula("C1"), "=BYROW(A1:B2,SUM)");
}

fn sales(model: &mut crate::Model) {
    let rows = [
        ["Region", "Item", "Sales"],
        ["East", "Pen", "10"],
        ["West", "Ink", "5"],
        ["East", "Ink", "7"],
        ["West", "Pen", "3"],
        ["East", "Pen", "2"],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            let col = ["A", "B", "C"][c];
            model._set(&format!("{col}{}", r + 1), v);
        }
    }
}

#[test]
fn groupby_groups_totals_sorts_and_filters() {
    let mut model = new_empty_model();
    sales(&mut model);
    // Headers are detected; grand total at the bottom.
    model._set("E1", "=GROUPBY(A1:A6,C1:C6,SUM)");
    // Two fields with subtotals, shown headers.
    model._set("H1", "=GROUPBY(A1:B6,C1:C6,SUM,3,2)");
    // Sorted by the value, descending, no totals.
    model._set("L1", "=GROUPBY(B2:B6,C2:C6,SUM,0,0,-2)");
    // Filtered, with a LAMBDA.
    model._set(
        "O1",
        "=GROUPBY(A2:A6,C2:C6,LAMBDA(v,COUNT(v)),0,0,,B2:B6=\"Pen\")",
    );
    model.evaluate();
    assert_eq!(
        cells(&model, &["E1", "F1", "E2", "F2", "E3", "F3"]),
        ["East", "19", "West", "8", "Total", "27"]
    );
    assert_eq!(
        cells(
            &model,
            &["H1", "I1", "J1", "H2", "I2", "J2", "H3", "I3", "J3", "H4", "I4", "J4"]
        ),
        ["Region", "Item", "Sales", "East", "Ink", "7", "East", "Pen", "12", "East", "", "19"]
    );
    assert_eq!(
        cells(&model, &["H5", "I5", "J5", "H7", "J7", "H8", "J8"]),
        ["West", "Ink", "5", "West", "8", "Total", "27"]
    );
    assert_eq!(
        cells(&model, &["L1", "M1", "L2", "M2", "L3"]),
        ["Pen", "15", "Ink", "12", ""]
    );
    assert_eq!(
        cells(&model, &["O1", "P1", "O2", "P2"]),
        ["East", "2", "West", "1"]
    );
}

#[test]
fn pivotby_crosses_rows_and_columns() {
    let mut model = new_empty_model();
    sales(&mut model);
    model._set("E1", "=PIVOTBY(A2:A6,B2:B6,C2:C6,SUM)");
    model._set("E6", "=PIVOTBY(A2:A6,B2:B6,C2:C6,SUM,0,0,,0)");
    model.evaluate();
    assert_eq!(
        cells(&model, &["E1", "F1", "G1", "H1"]),
        ["", "Ink", "Pen", "Total"]
    );
    assert_eq!(
        cells(&model, &["E2", "F2", "G2", "H2"]),
        ["East", "7", "12", "19"]
    );
    assert_eq!(
        cells(&model, &["E3", "F3", "G3", "H3"]),
        ["West", "5", "3", "8"]
    );
    assert_eq!(
        cells(&model, &["E4", "F4", "G4", "H4"]),
        ["Total", "12", "15", "27"]
    );
    assert_eq!(
        cells(&model, &["E6", "F6", "G6", "H6", "E7", "E9"]),
        ["", "Ink", "Pen", "", "East", ""]
    );
}

#[test]
fn forecast_ets_family() {
    let mut model = new_empty_model();
    let pattern = [10.0, 14.0, 8.0, 12.0];
    for i in 0..24 {
        model._set(&format!("A{}", i + 1), &format!("{}", i + 1));
        model._set(
            &format!("B{}", i + 1),
            &format!("{}", pattern[i % 4] + i as f64 * 0.5),
        );
    }
    model._set("D1", "=FORECAST.ETS(25,B1:B24,A1:A24)");
    model._set("D2", "=FORECAST.ETS.SEASONALITY(B1:B24,A1:A24)");
    model._set("D3", "=FORECAST.ETS.CONFINT(25,B1:B24,A1:A24)");
    model._set("D4", "=FORECAST.ETS.STAT(B1:B24,A1:A24,8)");
    model._set("D5", "=FORECAST.ETS(3,B1:B24,A1:A24)");
    model._set("D6", "=FORECAST.ETS(25,B1:B24,A1:A24,0)");
    model._set("D7", "=FORECAST.ETS.STAT(B1:B24,A1:A24,9)");
    model.evaluate();
    let d1: f64 = model._get_text("D1").parse().unwrap();
    assert!((d1 - 22.0).abs() < 0.5, "{d1}");
    assert_eq!(model._get_text("D2"), "4");
    let d3: f64 = model._get_text("D3").parse().unwrap();
    assert!(d3 >= 0.0 && d3 < 2.0, "{d3}");
    assert_eq!(model._get_text("D4"), "1");
    assert_eq!(model._get_text("D5"), "#NUM!");
    assert!(model._get_text("D6").parse::<f64>().is_ok());
    assert_eq!(model._get_text("D7"), "#NUM!");
}
