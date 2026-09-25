use crate::number_format::*;
use serde_json::json;
#[test]
fn numeric_width_projection_follows_the_selected_section_and_value_type() {
    for (format, value, expected) in [
        ("General", json!(25.76), Some("general")),
        ("[Red][<0]General;0.00", json!(-25.76), Some("general")),
        ("[Red][<0]General;0.00", json!(25.76), Some("fixed")),
        ("0.00%", json!(0.5), Some("fixed")),
        ("yyyy-mm-dd", json!(45000), Some("fixed")),
        ("yyyy-mm-dd", json!(-1), Some("invalidDate")),
        ("General", json!("25.76"), None),
        ("0.00", json!(true), None),
        ("@", json!(25.76), None),
        (";;;", json!(25.76), None),
        ("\"000 text\"", json!(25.76), None),
        ("\"unclosed", json!(25.76), None),
    ] {
        let display = render(&value, format, false, &[]);
        assert_eq!(
            display["numberWidth"].as_str(),
            expected,
            "{format}: {display}"
        );
    }
}
#[test]
fn format_basics() {
    for (code, value, expected) in [
        ("0.0000", 6.0, "6.0000"),
        ("General", 1234.5, "1234.5"),
        ("0.00%", 0.12345, "12.35%"),
        (";;;", 12.0, ""),
        ("yyyy-mm-dd", 60.0, "1900-02-29"),
        ("h:mm:ss", 0.9999999, "0:00:00"),
        ("[h]:mm:ss", 2.5, "60:00:00"),
        ("# ??/??", 2.125, "2  1/8 "),
    ] {
        let got = render(&json!(value), code, false, &[]);
        assert_eq!(got["text"], expected, "{code}: {got}");
        assert!(got["warning"].is_null(), "{code}: {got}");
    }
}

use crate::{office::Package, workspace::Core};
use serde_json::Value;

#[test]
fn sections_text_colors_spacing_and_invalid_formats() {
    for (v, expected, color) in [
        (125.0, "125.0 large", None),
        (-1.25, "(1.25)", Some("#FF0000")),
        (12.5, "12.500", None),
    ] {
        let got = render(
            &json!(v),
            "[>=100]0.0\" large\";[Red][<0](0.00);0.000",
            false,
            &[],
        );
        assert_eq!(got["text"], expected);
        assert_eq!(got["color"].as_str(), color);
    }
    assert_eq!(
        render(&json!("hello"), "0;0;0;\"Note: \"@", false, &[])["text"],
        "Note: hello"
    );
    assert_eq!(
        render(&json!(true), "0;0;0;\"Note: \"@", false, &[])["text"],
        "TRUE"
    );
    assert_eq!(
        render(&json!(15), "[Color3]0.00", false, &[])["color"],
        "#FF0000"
    );
    let mut colors = vec![String::new(); 11];
    colors[10] = "FF123456".into();
    assert_eq!(
        render(&json!(15), "[Color3]0.00", false, &colors)["color"],
        "#123456"
    );
    let got = render(
        &json!(1234.5),
        r#"_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)"#,
        false,
        &[],
    );
    assert_eq!(got["text"], " $1,234.50 ");
    assert_eq!(
        got["segments"],
        json!([{"kind":"skip","text":"("},{"kind":"text","text":"$"},{"kind":"fill","text":" "},{"kind":"text","text":"1,234.50"},{"kind":"skip","text":")"}])
    );
    assert_eq!(
        render(&json!(7), r#""_*[Red]"0"#, false, &[])["text"],
        "_*[Red]7"
    );
    for bad in [
        "\"unclosed",
        "0[Red",
        "0;0;0;@;0",
        "0\\",
        "0_",
        "# ?/99999999999999999999999999999",
    ] {
        assert!(validate(bad).is_err(), "{bad}");
        assert!(
            render(&json!(1.25), bad, false, &[])["warning"].is_string(),
            "{bad}"
        );
    }
    assert!(validate(&"0".repeat(256)).is_err());
}

#[test]
fn date_systems_and_time_carry_keep_the_underlying_serial() {
    for (value, code, expected) in [
        (0., "yyyy-mm-dd", "1900-01-00"),
        (1., "yyyy-mm-dd", "1900-01-01"),
        (59., "yyyy-mm-dd", "1900-02-28"),
        (61., "yyyy-mm-dd", "1900-03-01"),
        (60.9999999, "yyyy-mm-dd h:mm:ss", "1900-03-01 0:00:00"),
        (59.9999999, "yyyy-mm-dd h:mm:ss", "1900-02-29 0:00:00"),
        (0.5 + 65. / 86400., "mm:ss", "01:05"),
        (2.9999999, "[h]:mm:ss", "72:00:00"),
        (0.4999999, "h:mm:ss AM/PM", "12:00:00 PM"),
        (0.9999999, "h:mm:ss AM/PM", "12:00:00 AM"),
        (2958465.5, "yyyy-mm-dd h:mm", "9999-12-31 12:00"),
    ] {
        assert_eq!(
            render(&json!(value), code, false, &[])["text"],
            expected,
            "{value} {code}"
        );
    }
    assert_eq!(
        render(&json!(0), "yyyy-mm-dd", true, &[])["text"],
        "1904-01-01"
    );
    assert_eq!(
        render(&json!(1), "yyyy-mm-dd", true, &[])["text"],
        "1904-01-02"
    );
    assert_eq!(
        render(&json!(-1), "yyyy-mm-dd", false, &[])["text"],
        "########"
    );
    assert_eq!(
        render(&json!(2957004), "yyyy-mm-dd", true, &[])["text"],
        "########"
    );
}

fn imported(date1904: bool) -> Core {
    let mut c = Core::new();
    let name = if date1904 {
        "number-formats-1904.xlsx"
    } else {
        "number-formats.xlsx"
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(name);
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c
}
fn format(c: &mut Core, code: &str) -> Value {
    c.dispatch(
        json!({"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":{"numFmt":code}}),
    )
    .unwrap()
}
#[test]
fn imported_formats_survive_edit_copy_history_layout_and_reopening() {
    let mut c = imported(false);
    let first = c.snapshot();
    assert_eq!(first["formatted"]["office-1"]["B2"]["text"], "6.0000");
    assert_eq!(first["formatted"]["office-1"]["E2"]["text"], "4.0000");
    assert_eq!(first["formatted"]["office-1"]["E25"]["text"], "25.00%");
    assert_eq!(first["formatted"]["office-1"]["B21"]["text"], "7.0000");
    let copy = c
        .dispatch(json!({"op":"copyCells","sheetId":"office-1","range":"B2:B3"}))
        .unwrap();
    assert_eq!(copy["text"], "6.0000\r\n1234.5");
    let next = format(&mut c, "0.000000");
    assert_eq!(next["evaluatedCells"], first["evaluatedCells"]);
    assert_eq!(next["calculated"], first["calculated"]);
    assert_eq!(next["formatted"]["office-1"]["B2"]["text"], "6.000000");
    let undone = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(undone["formatted"], first["formatted"]);
    let redone = c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(redone["formatted"], next["formatted"]);
    let staged=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":{"numFmt":"0.0000000000"}}})).unwrap();
    assert_eq!(
        staged["formatted"]["office-1"]["B2"]["text"],
        "6.0000000000"
    );
    assert!(staged["layoutRows"]["office-1"]
        .as_array()
        .unwrap()
        .contains(&json!(2)));
    // Preview is read-only and does not invalidate the in-flight layout.
    c.dispatch(json!({"op":"previewNumberFormat","format":"0.000","value":6}))
        .unwrap();
    let measurements = staged["layoutRows"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, rows)| {
            (
                id.clone(),
                Value::Object(
                    rows.as_array()
                        .unwrap()
                        .iter()
                        .map(|r| (r.to_string(), json!(40.5)))
                        .collect(),
                ),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let committed = c.dispatch(
        json!({"op":"commitLayoutEdit","token":staged["layoutToken"],"sizes":measurements}),
    );
    // The dispatch protocol is intentionally asserted rather than bypassed.
    let committed = committed.unwrap();
    assert_eq!(committed["formatted"], staged["formatted"]);
    let folder = std::env::temp_dir().join("folio-number-format-tests");
    std::fs::create_dir_all(&folder).unwrap();
    let exported = folder.join("formatted.xlsx");
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":exported}))
        .unwrap();
    let path = folder.join("workspace.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let before = c.snapshot();
    std::fs::write(
        folder.join("display.json"),
        serde_json::to_vec_pretty(&before).unwrap(),
    )
    .unwrap();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let reopened = c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(before["formatted"], reopened["formatted"]);
    assert_eq!(before["calculated"], reopened["calculated"]);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join("reopened.xlsx")}))
        .unwrap();
    assert_eq!(
        std::fs::read(exported).unwrap(),
        std::fs::read(folder.join("reopened.xlsx")).unwrap()
    );
    let mut old_mac = imported(true);
    assert_eq!(
        old_mac.snapshot()["formatted"]["office-1"]["B22"]["text"],
        "1904-01-01"
    );
    let original = Package::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/fixtures/number-formats-1904.xlsx")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    old_mac
        .dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join("date1904.xlsx")}))
        .unwrap();
    let output = Package::read(folder.join("date1904.xlsx").to_str().unwrap()).unwrap();
    assert_eq!(
        original.text("xl/workbook.xml").unwrap(),
        output.text("xl/workbook.xml").unwrap()
    );
}

#[test]
fn independent_reference_corpus() {
    let formats = [
        "General",
        "0",
        "0.0000",
        "#,##0.00",
        "0.00%",
        "0;[Red](0);\"—\"",
        ";;;",
        "[>=100]0.0\" large\";[Red][<0](0.00);0.000",
        "# ??/??",
        "# ?/8",
        "0.00E+00",
        "##0.0E+0",
        "#,##0,,\"M\"",
        "[$€-407]#,##0.00",
        "0;0;0;\"Note: \"@",
        "_($* #,##0.00_);_($* (#,##0.00);_($* \"-\"??_);_(@_)",
        "yyyy-mm-dd",
        "h:mm:ss AM/PM",
        "mm:ss",
        "[h]:mm:ss",
    ];
    let mut cases = vec![];
    for code in formats {
        for n in -500..=500 {
            let value = n as f64 / 8.0;
            let result = render(&json!(value), code, false, &[]);
            cases.push(json!({"format":code,"value":value,"text":result["text"],"warning":result["warning"]}));
        }
    }
    let folder = std::env::temp_dir().join("folio-number-format-tests");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("reference-cases.json"),
        serde_json::to_vec(&cases).unwrap(),
    )
    .unwrap();
}

#[test]
fn errors_are_not_text_and_format_changes_cannot_hide_them() {
    let mut c = Core::new();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([{"id":"s","name":"Sheet","cells":{"A1":"=1/0","A2":"=\"#DIV/0!\"","A3":"42"},"cellStyles":{"A1":{"numFmt":";;;"},"A2":{"numFmt":"0;0;0;\"Result: \"@"}}}]);
    let first = c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    assert_eq!(first["formatted"]["s"]["A1"]["text"], "#DIV/0!");
    assert_eq!(first["formatted"]["s"]["A2"]["text"], "Result: #DIV/0!");
    let changed = c
        .dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"A1","value":"=A3"}]}))
        .unwrap();
    assert_eq!(changed["formatted"]["s"]["A1"]["text"], "");
}

#[test]
fn layout_markers_never_consume_literal_unicode_text() {
    let value = "custom \u{e000} glyph \u{e001}";
    let got = render(&json!(value), "_(@_)", false, &[]);
    assert_eq!(got["text"], format!(" {value} "));
    assert_eq!(got["segments"][1]["text"], value);
}

#[test]
fn am_pm_letters_and_fractional_seconds_follow_excel() {
    // 14:35:27.01 and 04:05:06.01, from Excel's own results in the Office
    // corpus (DateFormatTests.xlsx).
    let afternoon = (14.0 * 3600.0 + 35.0 * 60.0 + 27.01) / 86400.0;
    let morning = (4.0 * 3600.0 + 5.0 * 60.0 + 6.01) / 86400.0;
    let text = |code: &str, v: f64| ssf_rs::checked::number(code, v, false).unwrap();
    assert_eq!(text("hh:mm:ss a/p", afternoon), "02:35:27 p");
    assert_eq!(text("hh:mm:ss A/P", morning), "04:05:06 A");
    assert_eq!(text("h:mm:ss AM/PM", afternoon), "2:35:27 PM");
    assert_eq!(text("h:m:s.00 A/P", morning), "4:5:6.01 A");
    assert_eq!(text("h:m:s.00", morning), "4:5:6.01");
    assert_eq!(text("[h]:mm:ss.000", 3.0 + morning), "76:05:06.010");
}
