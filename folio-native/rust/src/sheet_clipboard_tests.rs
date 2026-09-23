use crate::{
    office::Package,
    sheet_clipboard::{decode, encode},
    workspace::Core,
};
use serde_json::{json, Value};
fn source() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([
      {"id":"sheet-1","name":"Source","cells":{"A1":"2","B1":"=A1+$A1+A$1+$A$1+'Other sheet'!A1+LEN(\"A1\")","C1":"One\nTwo\t\"three\"","A2":"7","B2":"=A2*2","A3":"9","B3":"old","C3":"erase","D8":"destination"},"cellStyles":{"B1":{"bold":true,"numFmt":"0.0000","fontName":"Georgia"},"C1":{"wrapText":true},"D8":{"italic":true}},"hiddenRows":[3]},
      {"id":"sheet-2","name":"Other sheet","cells":{"A1":"10","A2":"20","A3":"30","C5":"99"},"cellStyles":{"C5":{"italic":true}}}
    ]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    c
}
fn copy(c: &mut Core, range: &str) -> Value {
    c.dispatch(json!({"op":"copyCells","sheetId":"sheet-1","range":range}))
        .unwrap()
}
fn paste(c: &mut Core, copied: &Value, range: &str, mode: &str) -> Value {
    c.dispatch(json!({"op":"pasteCells","sheetId":"sheet-1","range":range,"token":copied["token"],"mode":mode})).unwrap()
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::temp_dir().join("folio-clipboard-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
#[test]
fn quoted_tsv_is_lossless_and_bounds_errors_are_explicit() {
    let rows = vec![
        vec![
            "a\tb".into(),
            "x\r\ny\nz".into(),
            "\"quoted\"".into(),
            "".into(),
        ],
        vec!["".into(), "🦉".into(), "  spaces  ".into(), "".into()],
    ];
    assert_eq!(decode(&encode(&rows)).unwrap(), rows);
    assert_eq!(
        decode("a\tb\r\nc\t\r\n").unwrap(),
        vec![vec!["a", "b"], vec!["c", ""]]
    );
    assert_eq!(decode("a\n\n").unwrap(), vec![vec!["a"], vec![""]]);
    assert_eq!(
        decode("\"\"\t\"a\"\"b\"\r\n").unwrap(),
        vec![vec!["", "a\"b"]]
    );
    assert_eq!(
        decode("a\tb\nc").unwrap(),
        vec![vec!["a", "b"], vec!["c", ""]]
    );
    assert!(decode("\"unclosed").is_err());
    assert!(decode("\"closed\"junk").is_err());
    assert!(decode(&"a".repeat(10001)).is_err());
    assert!(decode(&"\t".repeat(100001)).is_err());
}
#[test]
fn immutable_copy_overlaps_translates_mixed_refs_and_undo_restores_exact_source() {
    let mut c = source();
    let before_copy = c.workspace.clone();
    let copied = copy(&mut c, "A1:C2");
    assert_eq!(before_copy, c.workspace);
    assert_eq!(
        decode(copied["text"].as_str().unwrap()).unwrap()[0][2],
        "One\nTwo\t\"three\""
    );
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"sheet-1","address":"A1","value":"100"}]}))
        .unwrap();
    c.dispatch(json!({"op":"formatCells","sheetId":"sheet-1","addresses":["B1"],"style":{"italic":true,"fontName":"Menlo","numFmt":"0.0%"}})).unwrap();
    let before = c.workspace.clone();
    let result = paste(&mut c, &copied, "A2", "all");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A2"], "2");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A3"], "7");
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["B2"],
        "=A2+$A2+A$1+$A$1+'Other sheet'!A2+LEN(\"A1\")"
    );
    assert_eq!(result["calculated"]["sheet-1"]["B2"], 226.0);
    assert_eq!(c.workspace["sheets"][0]["cells"]["C3"], "");
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["B2"]["fontName"],
        "Georgia"
    );
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["B2"]["numFmt"],
        "0.0000"
    );
    assert_eq!(
        c.workspace["revision"],
        before["revision"].as_u64().unwrap() + 1
    );
    export(&mut c, "overlap.xlsx");
    let after = c.workspace.clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"], after["sheets"]);
}
#[test]
fn paste_special_values_formats_cross_sheet_tiling_and_transpose() {
    let mut c = source();
    let copied = copy(&mut c, "B1");
    let v=c.dispatch(json!({"op":"pasteCells","sheetId":"sheet-2","range":"C5","token":copied["token"],"mode":"values"})).unwrap();
    assert_eq!(v["calculated"]["sheet-2"]["C5"], 20.0);
    assert_eq!(c.workspace["sheets"][1]["cellStyles"]["C5"]["italic"], true);
    paste(&mut c, &copied, "D8", "formats");
    assert_eq!(c.workspace["sheets"][0]["cells"]["D8"], "destination");
    assert_eq!(c.workspace["sheets"][0]["cellStyles"]["D8"]["bold"], true);
    c.dispatch(json!({"op":"pasteCells","sheetId":"sheet-2","range":"C5","token":copied["token"],"mode":"formulas"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][1]["cells"]["C5"],
        "=B5+$A5+B$1+$A$1+'Other sheet'!B5+LEN(\"A1\")"
    );
    assert_eq!(c.workspace["sheets"][1]["cellStyles"]["C5"]["italic"], true);
    let copied = copy(&mut c, "A1:C2");
    c.dispatch(json!({"op":"pasteCells","sheetId":"sheet-1","range":"F4","token":copied["token"],"mode":"values","transpose":true,"skipBlanks":true})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["G4"], "7");
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["F6"],
        "One\nTwo\t\"three\""
    );
    assert!(c.workspace["sheets"][0]["cells"].get("G6").is_none());
    let copied = copy(&mut c, "A1:A2");
    paste(&mut c, &copied, "J1:K4", "all");
    for (a, v) in [("J1", "2"), ("K1", "2"), ("J2", "7"), ("K4", "7")] {
        assert_eq!(c.workspace["sheets"][0]["cells"][a], v);
    }
    export(&mut c, "special.xlsx");
}
#[test]
fn external_multiline_quotes_blank_edges_and_failed_ranges_are_atomic() {
    let mut c = source();
    c.dispatch(json!({"op":"pasteCells","sheetId":"sheet-1","range":"A10","text":"\"one\ntwo\"\t\"tab\tinside\"\t\"\"\r\n\"a\"\"b\"\tsecond\tend\r\n"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["A10"], "one\ntwo");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B10"], "tab\tinside");
    assert_eq!(c.workspace["sheets"][0]["cells"]["C10"], "");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A11"], "a\"b");
    let copied = copy(&mut c, "A1:C2");
    let before = c.workspace.clone();
    for r in [
        json!({"range":"XFD1048576","text":"a\tb"}),
        json!({"range":"A1","text":"\"bad"}),
        json!({"range":"A1:B3","token":copied["token"]}),
        json!({"range":"A1","token":"expired"}),
        json!({"range":"A1","text":"a","baseRevision":0}),
    ] {
        let mut r = r;
        r["op"] = json!("pasteCells");
        r["sheetId"] = json!("sheet-1");
        assert!(c.dispatch(r).is_err());
        assert_eq!(c.workspace, before);
    }
    export(&mut c, "external.xlsx");
}
#[test]
fn rich_source_xf_and_shared_runs_survive_paste_save_reopen_and_layout_staging() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/rich-text.xlsx")})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":id,"range":"A1:D5"}))
        .unwrap();
    let before = c.workspace.clone();
    let staged=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"pasteCells","sheetId":id,"range":"G10","token":copied["token"]}})).unwrap();
    assert_eq!(c.workspace, before);
    let mut sizes = json!({});
    for (id, rows) in staged["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(45);
        }
    }
    c.dispatch(json!({"op":"commitLayoutEdit","token":staged["layoutToken"],"sizes":sizes}))
        .unwrap();
    let result = export(&mut c, "rich.xlsx");
    let projection = crate::xlsx::import(&result).unwrap().0;
    for (a, b) in [("C2", "I11"), ("D2", "J11"), ("C3", "I12")] {
        assert_eq!(projection[0]["cells"][a], projection[0]["cells"][b]);
        let mut expected = projection[0]["styleDefaults"]["default"]
            .as_object()
            .unwrap()
            .clone();
        if let Some(properties) = projection[0]["cellStyles"][a].as_object() {
            expected.extend(properties.clone());
        }
        assert_eq!(json!(expected), projection[0]["cellStyles"][b]);
    }
    assert_eq!(projection[0]["cells"]["H11"], "=G11*$H$1");
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":id,"range":"S2:T2"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":id,"range":"G20","token":copied["token"]}))
        .unwrap();
    let result = export(&mut c, "rich-shared.xlsx");
    let projection = crate::xlsx::import(&result).unwrap().0;
    assert_eq!(projection[0]["cells"]["S2"], projection[0]["cells"]["G20"]);
    assert_eq!(
        projection[0]["cellStyles"]["S2"]["_richText"],
        projection[0]["cellStyles"]["G20"]["_richText"]
    );
    let dir = std::env::temp_dir().join("folio-clipboard-tests");
    let path = dir.join("rich.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let saved = c.workspace.clone();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace, saved);
    export(&mut c, "rich-reopened.xlsx");
}

#[test]
fn clipboard_preserves_literal_results_inherited_formats_and_rejects_unsafe_destinations() {
    let mut c = source();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"sheet-1","address":"A20","value":"=\"=SUM(A1:A2)\""}]})).unwrap();
    let copied = copy(&mut c, "A20");
    let pasted = paste(&mut c, &copied, "B20", "values");
    assert_eq!(pasted["calculated"]["sheet-1"]["B20"], "=SUM(A1:A2)");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B20"], "'=SUM(A1:A2)");
    c.dispatch(
        json!({"op":"pasteCells","sheetId":"sheet-1","range":"C20","text":"=2+2","mode":"values"}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["C20"], "'=2+2");
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/rich-text.xlsx")})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":id,"range":"P2"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":id,"range":"AA2","token":copied["token"]}))
        .unwrap();
    let pkg = export(&mut c, "inherited.xlsx");
    let sheets = crate::xlsx::import(&pkg).unwrap().0;
    let bases =
        crate::xlsx_styles::SheetStyleBases::read(&pkg, sheets[0]["officePath"].as_str()).unwrap();
    assert_eq!(bases.at("P2").unwrap(), bases.at("AA2").unwrap());
    let before = c.workspace.clone();
    for range in ["J2", "M2"] {
        assert!(c
            .dispatch(json!({"op":"pasteCells","sheetId":id,"range":range,"text":"bad"}))
            .is_err());
        assert_eq!(c.workspace, before);
    }
    // An unavailable opaque token never silently downgrades to the public text.
    assert!(c
        .dispatch(
            json!({"op":"pasteCells","sheetId":id,"range":"A1","token":"old","text":"fallback"})
        )
        .is_err());
    assert_eq!(c.workspace, before);
}
