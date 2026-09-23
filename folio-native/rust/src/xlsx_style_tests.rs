use crate::{
    office::{attr, child, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};

fn output() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-style-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    named_fixture("fill.xlsx")
}
fn named_fixture(name: &str) -> Core {
    let mut c = Core::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(name);
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn format(c: &mut Core, style: Value) {
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":style}))
        .unwrap();
}
fn style_id(pkg: &Package, cell: &str) -> usize {
    let xml = parse(pkg.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    xml.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "c" && attr(*n, "r") == Some(cell))
        .and_then(|n| attr(n, "s"))
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn a_bold_edit_keeps_the_original_font_alignment_number_format_and_protection() {
    let mut c = fixture();
    let original = export(&mut c, "original.xlsx");
    format(&mut c, json!({"bold":false}));
    let edited = export(&mut c, "bold.xlsx");
    let doc = parse(edited.text("xl/styles.xml").unwrap()).unwrap();
    let root = doc.root_element();
    let xf = child(root, "cellXfs")
        .unwrap()
        .children()
        .filter(|n| n.is_element())
        .nth(style_id(&edited, "B2"))
        .unwrap();
    let font = child(root, "fonts")
        .unwrap()
        .children()
        .filter(|n| n.is_element())
        .nth(attr(xf, "fontId").unwrap().parse().unwrap())
        .unwrap();
    assert_eq!(
        child(font, "name").and_then(|n| attr(n, "val")),
        Some("Cambria")
    );
    assert_eq!(child(font, "sz").and_then(|n| attr(n, "val")), Some("18"));
    assert_eq!(
        child(font, "u").and_then(|n| attr(n, "val")),
        Some("double")
    );
    assert!(child(font, "strike").is_some());
    assert_eq!(
        child(xf, "alignment").and_then(|n| attr(n, "textRotation")),
        Some("15")
    );
    assert_eq!(
        child(xf, "protection").and_then(|n| attr(n, "hidden")),
        Some("1")
    );
    let before = parse(original.text("xl/styles.xml").unwrap()).unwrap();
    let old = child(before.root_element(), "cellXfs")
        .unwrap()
        .children()
        .filter(|n| n.is_element())
        .nth(style_id(&original, "B2"))
        .unwrap();
    for key in [
        "numFmtId",
        "fillId",
        "borderId",
        "xfId",
        "applyAlignment",
        "applyProtection",
    ] {
        assert_eq!(attr(xf, key), attr(old, key), "{key}");
    }
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone.xlsx").original, original.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "redone.xlsx").original, edited.original);
    let path = output().join("bold.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "reopened.xlsx").original,
        edited.original
    );
    loaded
        .dispatch(json!({"op":"importOffice","path":output().join("bold.xlsx")}))
        .unwrap();
    assert_eq!(
        export(&mut loaded, "reimported.xlsx").original,
        edited.original
    );
}

#[test]
fn individual_format_edits_keep_the_other_source_properties_and_deduplicate() {
    for (name, style) in [
        ("italic", json!({"italic":false})),
        ("fill", json!({"fill":"#AABBCC"})),
        ("color", json!({"color":"#112233"})),
        ("number", json!({"numFmt":"0.00%"})),
    ] {
        let mut c = named_fixture("styles.xlsx");
        format(&mut c, style.clone());
        let edited = export(&mut c, &format!("{name}.xlsx"));
        let projected = crate::xlsx::import(&edited).unwrap().0;
        assert_eq!(projected[0]["cellStyles"]["B2"]["fontName"], "Cambria");
        assert_eq!(projected[0]["cellStyles"]["B2"]["fontSize"], 18.0);
        for (key, value) in style.as_object().unwrap() {
            assert_eq!(&projected[0]["cellStyles"]["B2"][key], value);
        }
    }
    let mut c = fixture();
    let addresses = (10..210).map(|r| format!("A{r}")).collect::<Vec<_>>();
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":addresses,"style":{"bold":true,"numFmt":"0.000001"}})).unwrap();
    let p = export(&mut c, "deduplicated.xlsx");
    let first = style_id(&p, "A10");
    for r in 11..210 {
        assert_eq!(style_id(&p, &format!("A{r}")), first);
    }
    let original = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/fill.xlsx"
    ))
    .unwrap();
    let before = parse(original.text("xl/styles.xml").unwrap()).unwrap();
    let after = parse(p.text("xl/styles.xml").unwrap()).unwrap();
    for name in ["fonts", "cellXfs", "numFmts"] {
        let count = |doc: &roxmltree::Document| {
            child(doc.root_element(), name)
                .unwrap()
                .children()
                .filter(|n| n.is_element())
                .count()
        };
        assert_eq!(count(&after), count(&before) + 1, "{name}");
    }
}

#[test]
fn inherited_styles_boolean_values_and_builtin_number_formats_survive_edits() {
    let mut c = named_fixture("styles.xlsx");
    let styles = &c.workspace["sheets"][0]["cellStyles"];
    assert_eq!(styles["F2"]["numFmt"], "mm-dd-yy");
    assert!(styles["G2"].get("numFmt").is_none());
    for key in ["bold", "italic", "strike"] {
        assert_eq!(styles["I2"][key], false);
    }
    assert_eq!(styles["L2"]["fontName"], "Cambria");
    assert_eq!(styles["P2"]["fontName"], "Georgia");
    assert_eq!(styles["R7"]["fontName"], "Verdana");
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["F2","G2","L2","P2","P10","R7"],"style":{"bold":false,"italic":false}})).unwrap();
    let p = export(&mut c, "inherited.xlsx");
    let d = parse(p.text("xl/styles.xml").unwrap()).unwrap();
    let xfs = child(d.root_element(), "cellXfs").unwrap();
    for (a, id) in [("F2", "14"), ("G2", "27")] {
        let xf = xfs
            .children()
            .filter(|n| n.is_element())
            .nth(style_id(&p, a))
            .unwrap();
        assert_eq!(attr(xf, "numFmtId"), Some(id));
    }
    let styles = crate::xlsx::import(&p).unwrap().0[0]["cellStyles"].clone();
    for (a, name) in [
        ("L2", "Cambria"),
        ("P2", "Georgia"),
        ("P10", "Georgia"),
        ("R7", "Verdana"),
    ] {
        assert_eq!(styles[a]["fontName"], name);
        assert_eq!(styles[a]["bold"], false);
        assert_eq!(styles[a]["italic"], false);
    }
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["G2"],"style":{"numFmt":"General"}})).unwrap();
    let p = export(&mut c, "explicit-general.xlsx");
    assert_eq!(
        crate::xlsx::import(&p).unwrap().0[0]["cellStyles"]["G2"]["numFmt"],
        "General"
    );
}

#[test]
fn prefixed_style_records_keep_namespace_identity_and_extension_children() {
    let mut c = named_fixture("styles-prefixed.xlsx");
    format(
        &mut c,
        json!({"bold":false,"fill":"#AABBCC","numFmt":"0.000001"}),
    );
    let p = export(&mut c, "prefixed.xlsx");
    let d = parse(p.text("xl/styles.xml").unwrap()).unwrap();
    let fonts = child(d.root_element(), "fonts").unwrap();
    let xfs = child(d.root_element(), "cellXfs").unwrap();
    let ns = "urn:folio:style:metadata";
    assert_eq!(fonts.attribute((ns, "count")), Some("keep-fonts"));
    assert_eq!(xfs.attribute((ns, "count")), Some("keep-xfs"));
    let xf = xfs
        .children()
        .filter(|n| n.is_element())
        .nth(style_id(&p, "B2"))
        .unwrap();
    assert_eq!(xf.attribute((ns, "fontId")), Some("keep-id"));
    assert_eq!(xf.attribute("quotePrefix"), Some("1"));
    let font = fonts
        .children()
        .filter(|n| n.is_element())
        .nth(
            xf.attributes()
                .find(|a| a.namespace().is_none() && a.name() == "fontId")
                .unwrap()
                .value()
                .parse()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(font.attribute((ns, "flag")), Some("keep-font"));
    assert!(
        xf.descendants()
            .any(|n| n.has_tag_name((ns, "payload"))
                && n.attribute("value") == Some("keep-extension"))
    );
    assert_eq!(
        child(font, "name").and_then(|n| attr(n, "val")),
        Some("Cambria")
    );
}

#[test]
fn older_workspace_metadata_hydrates_without_overwriting_user_edits() {
    let mut c = named_fixture("styles.xlsx");
    format(&mut c, json!({"bold":false}));
    let path = output().join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for location in ["sheets", "baseline"] {
        let sheets = if location == "sheets" {
            &mut saved["sheets"]
        } else {
            &mut saved["officeWorkbook"]["baseline"]
        };
        let styles = sheets[0]["cellStyles"].as_object_mut().unwrap();
        for style in styles.values_mut() {
            let style = style.as_object_mut().unwrap();
            for key in ["fontName", "fontSize", "strike", "underline"] {
                style.remove(key);
            }
        }
        for a in ["F2", "G2"] {
            styles.get_mut(a).unwrap()["numFmt"] = json!("General");
        }
        for a in ["P2", "P3", "P4", "R7"] {
            styles.remove(a);
        }
    }
    // An older client could already author direct formatting on an inherited cell.
    saved["sheets"][0]["cellStyles"]["P2"] = json!({"italic":false});
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    let styles = &loaded.workspace["sheets"][0]["cellStyles"];
    assert_eq!(styles["B2"]["bold"], false);
    assert_eq!(styles["B2"]["fontName"], "Cambria");
    assert_eq!(styles["F2"]["numFmt"], "mm-dd-yy");
    assert!(styles["G2"].get("numFmt").is_none());
    assert_eq!(styles["P2"]["fontName"], "Georgia");
    assert_eq!(styles["P2"]["italic"], false);
    export(&mut loaded, "legacy.xlsx");
    let mut invalid = saved;
    invalid["sheets"] = json!(null);
    std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let before = loaded.workspace.clone();
    assert!(loaded.dispatch(json!({"op":"load","path":path})).is_err());
    assert_eq!(loaded.workspace, before);
}

#[test]
fn font_authoring_roundtrips_every_effect_without_losing_other_cell_formatting() {
    let mut c = named_fixture("styles.xlsx");
    let before = export(&mut c, "font-original.xlsx");
    let style = json!({"fontName":"Georgia","fontSize":16.5,"bold":false,"italic":false,"strike":false,"underline":"doubleAccounting","verticalAlign":"subscript","color":"automatic"});
    format(&mut c, style.clone());
    let edited = export(&mut c, "font.xlsx");
    let read = crate::xlsx::import(&edited).unwrap().0;
    for (key, value) in style.as_object().unwrap() {
        assert_eq!(&read[0]["cellStyles"]["B2"][key], value, "{key}");
    }
    let path = output().join("font.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut reopened, "font-reopened.xlsx").original,
        edited.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "font-undone.xlsx").original, before.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "font-redone.xlsx").original, edited.original);
    for (i, underline) in [
        "none",
        "single",
        "double",
        "singleAccounting",
        "doubleAccounting",
    ]
    .iter()
    .enumerate()
    {
        c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":[format!("V{}",i+2)],"style":{"underline":underline,"fontSize":1.0+i as f64*0.5,"verticalAlign":if i==0 {"baseline"}else{"superscript"}}})).unwrap();
    }
    let p = export(&mut c, "font-effects.xlsx");
    let read = crate::xlsx::import(&p).unwrap().0;
    for (i, underline) in [
        "none",
        "single",
        "double",
        "singleAccounting",
        "doubleAccounting",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            read[0]["cellStyles"][format!("V{}", i + 2)]["underline"],
            *underline
        );
    }
}

#[test]
fn rich_font_edits_change_only_requested_run_properties_and_isolate_shared_strings() {
    let mut c = named_fixture("styles.xlsx");
    // The base XF is already not bold. Explicit false must still clear a
    // rich run's own bold flag, without changing its font or italic flag.
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["C2","S2"],"style":{"bold":false}})).unwrap();
    export(&mut c, "rich-bold.xlsx");
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["C2","S2"],"style":{"fontName":"Georgia","fontSize":16.5,"underline":"doubleAccounting","verticalAlign":"subscript","color":"automatic"}})).unwrap();
    let p = export(&mut c, "rich-font.xlsx");
    let xml = parse(p.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    let untouched = xml
        .descendants()
        .find(|n| n.is_element() && attr(*n, "r") == Some("T2"))
        .unwrap();
    assert_eq!(attr(untouched, "t"), Some("s"));
    let original = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/styles.xlsx"
    ))
    .unwrap();
    assert_eq!(
        p.entries["xl/sharedStrings.xml"],
        original.entries["xl/sharedStrings.xml"]
    );
    let path = output().join("rich-font.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "rich-font-reopened.xlsx").original,
        p.original
    );
}

#[test]
fn invalid_fonts_and_stale_format_requests_are_atomic_and_defaults_follow_structure() {
    let mut c = named_fixture("styles.xlsx");
    let before = c.workspace.clone();
    for style in [
        json!({"fontName":""}),
        json!({"fontName":" Arial"}),
        json!({"fontName":"A\nB"}),
        json!({"fontName":"a".repeat(256)}),
        json!({"fontSize":0}),
        json!({"fontSize":409.5}),
        json!({"fontSize":11.2}),
        json!({"fontSize":"12"}),
        json!({"underline":"triple"}),
        json!({"verticalAlign":"up"}),
        json!({"strike":"yes"}),
        json!({"color":"#123"}),
    ] {
        assert!(c
            .dispatch(
                json!({"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":style})
            )
            .is_err());
        assert_eq!(c.workspace, before);
    }
    assert!(c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":{"bold":false},"baseRevision":9999})).is_err());
    assert_eq!(c.workspace, before);
    for size in [1., 1.5, 409.] {
        format(&mut c, json!({"fontSize":size}));
    }
    let defaults = c.workspace["sheets"][0]["styleDefaults"].clone();
    assert_eq!(defaults["rows"]["7"]["fontName"], "Verdana");
    assert!(defaults["columns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["min"] == 16 && v["style"]["fontName"] == "Georgia"));
    c.dispatch(
        json!({"op":"structure","action":"insertColumns","sheetId":"office-1","at":1,"count":1}),
    )
    .unwrap();
    assert!(c.workspace["sheets"][0]["styleDefaults"]["columns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["min"] == 17 && v["style"]["fontName"] == "Georgia"));
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["styleDefaults"], defaults);
}

#[test]
fn rich_text_projection_keeps_partial_properties_unicode_and_excludes_phonetics() {
    let mut c = named_fixture("rich-text.xlsx");
    let calculated = c.snapshot()["calculated"]["office-1"].clone();
    assert_eq!(calculated["A20"], "Rich text 🌊");
    assert_eq!(calculated["H20"], "東京");
    for (a, value) in [("E20", "001"), ("F20", "=literal"), ("G20", "TRUE")] {
        assert_eq!(calculated[a], value);
    }
    let sheet = &c.workspace["sheets"][0];
    for a in ["A20", "B20", "C20"] {
        assert_eq!(sheet["cells"][a], "Rich text 🌊");
        let runs = &sheet["cellStyles"][a]["_richText"];
        assert_eq!(runs.as_array().unwrap().len(), 2);
        assert_eq!(runs[0]["text"], "Rich ");
        assert_eq!(runs[0]["style"]["fontName"], "Georgia");
        assert_eq!(runs[0]["style"]["fontSize"], 18.);
        assert_eq!(runs[0]["style"]["bold"], true);
        assert_eq!(runs[1]["style"]["bold"], false);
        assert_eq!(runs[1]["style"]["italic"], true);
        assert_eq!(runs[1]["style"]["strike"], false);
        assert_eq!(runs[1]["style"]["underline"], "single");
        assert_eq!(runs[1]["style"]["color"], "#0044BB");
    }
    let inherited = &sheet["cellStyles"]["D20"]["_richText"];
    assert_eq!(inherited[0]["style"], json!({}));
    assert_eq!(inherited[1]["style"]["verticalAlign"], "baseline");
    assert_eq!(inherited[1]["style"]["color"], "automatic");
    assert_eq!(inherited.as_array().unwrap().len(), 2);
    for (a, raw) in [
        ("E20", "'001"),
        ("F20", "'=literal"),
        ("G20", "'TRUE"),
        ("H20", "東京"),
    ] {
        assert_eq!(sheet["cells"][a], raw);
    }
    let before = c.workspace["sheets"].clone();
    let original = export(&mut c, "rich-display-original.xlsx");
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["B20"],"style":{"bold":false,"underline":"double"}})).unwrap();
    let edited = export(&mut c, "rich-display.xlsx");
    let imported = crate::xlsx::import(&edited).unwrap().0;
    for run in imported[0]["cellStyles"]["B20"]["_richText"]
        .as_array()
        .unwrap()
    {
        assert_eq!(run["style"]["bold"], false);
        assert_eq!(run["style"]["underline"], "double");
    }
    assert_eq!(
        imported[0]["cellStyles"]["C20"]["_richText"],
        before[0]["cellStyles"]["C20"]["_richText"]
    );
    let path = output().join("rich-display.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        loaded.workspace["sheets"][0]["cellStyles"]["B20"],
        c.workspace["sheets"][0]["cellStyles"]["B20"]
    );
    assert_eq!(
        export(&mut loaded, "rich-display-reopened.xlsx").original,
        edited.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before);
    assert_eq!(
        export(&mut c, "rich-display-undone.xlsx").original,
        original.original
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        export(&mut c, "rich-display-redone.xlsx").original,
        edited.original
    );
}

#[test]
fn old_workspaces_gain_rich_runs_without_losing_edits_and_fill_moves_them() {
    let mut c = named_fixture("styles.xlsx");
    let source = c.workspace["sheets"][0]["cellStyles"]["S2"]["_richText"].clone();
    let original = export(&mut c, "rich-legacy-original.xlsx");
    for sheet in c.workspace["sheets"].as_array_mut().unwrap() {
        for style in sheet["cellStyles"].as_object_mut().unwrap().values_mut() {
            style.as_object_mut().unwrap().remove("_richText");
        }
    }
    for sheet in c.workspace["officeWorkbook"]["baseline"]
        .as_array_mut()
        .unwrap()
    {
        for style in sheet["cellStyles"].as_object_mut().unwrap().values_mut() {
            style.as_object_mut().unwrap().remove("_richText");
        }
    }
    c.dispatch(
        json!({"op":"formatCells","sheetId":"office-1","addresses":["S2"],"style":{"bold":false}}),
    )
    .unwrap();
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"office-1","address":"T2","value":"Replacement"}]}),
    )
    .unwrap();
    crate::xlsx_styles::hydrate(&mut c.workspace, &original).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["S2"]["_richText"],
        source
    );
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["S2"]["_fontOverrides"]["bold"],
        false
    );
    assert_eq!(c.workspace["sheets"][0]["cells"]["T2"], "Replacement");
    let edited = export(&mut c, "rich-legacy.xlsx");
    let projection = crate::xlsx::import(&edited).unwrap().0;
    assert!(projection[0]["cellStyles"]["T2"].get("_richText").is_none());
    c.dispatch(json!({"op":"fillCells","sheetId":"office-1","range":"S2:S3","direction":"down"}))
        .unwrap();
    let filled = &c.workspace["sheets"][0];
    assert_eq!(filled["cells"]["S3"], "Shared title");
    assert_eq!(
        filled["cellStyles"]["S3"]["_richText"],
        filled["cellStyles"]["S2"]["_richText"]
    );
    assert_eq!(
        filled["cellStyles"]["S3"]["_richText"][0]["style"]["bold"],
        false
    );
    c.dispatch(
        json!({"op":"structure","action":"insertColumns","sheetId":"office-1","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["T3"]["_richText"][0]["style"]["fontName"],
        "Times New Roman"
    );
}

#[test]
fn legacy_phonetic_text_is_repaired_only_when_it_still_matches_the_original_import() {
    let mut c = named_fixture("rich-text.xlsx");
    let pkg = export(&mut c, "phonetic-original.xlsx");
    for (a, old) in [
        ("A20", "Rich text 🌊PHONETIC ONLY"),
        ("B20", "Rich text 🌊PHONETIC ONLY"),
        ("H20", "東京とうきょう"),
    ] {
        c.workspace["sheets"][0]["cells"][a] = json!(old);
        c.workspace["officeWorkbook"]["baseline"][0]["cells"][a] = json!(old);
    }
    c.workspace["sheets"][0]["cells"]["B20"] = json!("User edit");
    crate::xlsx_styles::hydrate(&mut c.workspace, &pkg).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["A20"], "Rich text 🌊");
    assert_eq!(c.workspace["sheets"][0]["cells"]["H20"], "東京");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B20"], "User edit");
    assert_eq!(
        c.workspace["officeWorkbook"]["baseline"][0]["cells"]["B20"],
        "Rich text 🌊"
    );
    let first = c.workspace.clone();
    crate::xlsx_styles::hydrate(&mut c.workspace, &pkg).unwrap();
    assert_eq!(c.workspace, first);
}

fn characters(c: &mut Core, address: &str, start: usize, end: usize, style: Value) {
    let sheet = &c.workspace["sheets"][0];
    let text = crate::sheet_text::literal(sheet["cells"][address].as_str().unwrap()).unwrap();
    c.dispatch(json!({"op":"formatCellText","sheetId":sheet["id"],"address":address,
        "baseRevision":c.workspace["revision"],"expectedText":text,"start":start,"end":end,"style":style})).unwrap();
}

#[test]
fn selected_character_formats_cross_runs_and_preserve_source_metadata_and_history() {
    let mut c = named_fixture("rich-text.xlsx");
    let original = export(&mut c, "characters-original.xlsx");
    let before = c.workspace["sheets"].clone();
    characters(
        &mut c,
        "B20",
        2,
        8,
        json!({"bold":false,"fontSize":16.5,"color":"#123456"}),
    );
    let edited = export(&mut c, "characters.xlsx");
    let imported = crate::xlsx::import(&edited).unwrap().0;
    let runs = &imported[0]["cellStyles"]["B20"]["_richText"];
    assert_eq!(
        runs.as_array()
            .unwrap()
            .iter()
            .map(|v| v["text"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["Ri", "ch ", "tex", "t 🌊"]
    );
    assert_eq!(runs[0]["style"]["bold"], true);
    assert_eq!(runs[0]["style"]["fontSize"], 18.);
    for index in [1, 2] {
        assert_eq!(runs[index]["style"]["bold"], false);
        assert_eq!(runs[index]["style"]["fontSize"], 16.5);
        assert_eq!(runs[index]["style"]["color"], "#123456");
    }
    assert_eq!(runs[2]["style"]["italic"], true);
    assert_eq!(runs[3]["style"]["fontSize"], 12.);
    assert_eq!(
        imported[0]["cellStyles"]["C20"],
        before[0]["cellStyles"]["C20"]
    );
    assert_eq!(
        edited.text("xl/sharedStrings.xml").unwrap(),
        original.text("xl/sharedStrings.xml").unwrap()
    );
    let path = output().join("characters.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "characters-reopened.xlsx").original,
        edited.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before);
    assert_eq!(
        export(&mut c, "characters-undone.xlsx").original,
        original.original
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        export(&mut c, "characters-redone.xlsx").original,
        edited.original
    );
}

#[test]
fn character_and_whole_cell_font_edits_compose_and_survive_package_rebasing() {
    let mut c = named_fixture("styles.xlsx");
    characters(&mut c, "S2", 0, 6, json!({"bold":false}));
    characters(&mut c, "S2", 2, 9, json!({"color":"#123456"}));
    let once = c.workspace["sheets"][0]["cellStyles"]["S2"]["_richTextEdits"].clone();
    for _ in 0..20 {
        characters(&mut c, "S2", 2, 9, json!({"color":"#123456"}));
    }
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["S2"]["_richTextEdits"],
        once
    );
    assert_eq!(once.as_array().unwrap().len(), 3);
    c.dispatch(
        json!({"op":"formatCells","sheetId":"office-1","addresses":["S2"],"style":{"bold":true}}),
    )
    .unwrap();
    let whole = export(&mut c, "characters-whole.xlsx");
    let whole = crate::xlsx::import(&whole).unwrap().0;
    assert!(whole[0]["cellStyles"]["S2"]["_richText"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["style"]["bold"] == true));
    characters(
        &mut c,
        "S2",
        3,
        5,
        json!({"bold":false,"underline":"double"}),
    );
    let expected = c.workspace["sheets"][0]["cellStyles"]["S2"]["_richText"].clone();
    c.dispatch(
        json!({"op":"structure","action":"insertColumns","sheetId":"office-1","at":1,"count":1}),
    )
    .unwrap();
    let moved = export(&mut c, "characters-moved.xlsx");
    let moved = crate::xlsx::import(&moved).unwrap().0;
    // Adjacent source runs can keep distinct inherited properties, but each
    // code point must retain the final requested formatting after rebasing.
    let chars = |runs: &Value| {
        runs.as_array()
            .unwrap()
            .iter()
            .flat_map(|r| {
                r["text"].as_str().unwrap().chars().map(move |ch| {
                    (
                        ch,
                        r["style"]["bold"].clone(),
                        r["style"]["underline"].clone(),
                    )
                })
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        chars(&moved[0]["cellStyles"]["T2"]["_richText"]),
        chars(&expected)
    );
    c.dispatch(json!({"op":"fillCells","sheetId":"office-1","range":"T2:T3","direction":"down"}))
        .unwrap();
    let filled = export(&mut c, "characters-filled.xlsx");
    let filled = crate::xlsx::import(&filled).unwrap().0;
    assert_eq!(
        filled[0]["cellStyles"]["T3"]["_richText"],
        filled[0]["cellStyles"]["T2"]["_richText"]
    );
}

#[test]
fn character_formatting_handles_new_text_literal_strings_and_rejects_invalid_requests_atomically() {
    let mut c = named_fixture("rich-text.xlsx");
    let before = c.workspace.clone();
    let base = json!({"op":"formatCellText","sheetId":"office-1","address":"A20","baseRevision":c.workspace["revision"],"expectedText":"Rich text 🌊","start":10,"end":12,"style":{"bold":true}});
    for (key, value) in [
        ("start", json!(-1)),
        ("start", json!(11)),
        ("end", json!(11)),
        ("end", json!(13)),
        ("start", json!(12)),
        ("baseRevision", json!(99999)),
        ("expectedText", json!("stale")),
        ("style", json!({"fontSize":12.3})),
        ("style", json!({"fill":"#112233"})),
    ] {
        let mut r = base.clone();
        r[key] = value;
        assert!(c.dispatch(r).is_err());
        assert_eq!(c.workspace, before);
    }
    c.dispatch(base).unwrap();
    for a in ["E20", "F20", "G20", "I20"] {
        characters(&mut c, a, 0, 1, json!({"color":"#123456"}));
    }
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"office-1","address":"Z20","value":"Plain text"}]}),
    )
    .unwrap();
    characters(&mut c, "Z20", 6, 10, json!({"italic":true}));
    export(&mut c, "characters-literals.xlsx");
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"office-1","address":"Z20","value":"Different"}]}),
    )
    .unwrap();
    let cleared = export(&mut c, "characters-replaced.xlsx");
    let projection = crate::xlsx::import(&cleared).unwrap().0;
    assert!(projection[0]["cellStyles"]["Z20"]
        .get("_richText")
        .is_none());
    characters(&mut c, "Z20", 0, 4, json!({"bold":true}));
    export(&mut c, "characters-reformatted.xlsx");
    let mut fresh = Core::new();
    let id = fresh.workspace["sheets"][0]["id"].clone();
    fresh
        .dispatch(
            json!({"op":"cells","cells":[{"sheetId":id,"address":"A40","value":"New rich text"}]}),
        )
        .unwrap();
    characters(
        &mut fresh,
        "A40",
        4,
        8,
        json!({"fontName":"Georgia","color":"#123456"}),
    );
    let pkg = export(&mut fresh, "characters-new.xlsx");
    assert_eq!(
        crate::xlsx::import(&pkg).unwrap().0[0]["cellStyles"]["A40"]["_richText"][1]["text"],
        "rich"
    );
}

#[test]
fn new_workbook_font_changes_keep_normal_font_and_export_ordered_properties() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"sheet-1","address":"A1","value":"Heading"},{"sheetId":"sheet-1","address":"B1","value":"Unchanged"}]})).unwrap();
    c.dispatch(json!({"op":"formatCells","sheetId":"sheet-1","addresses":["A1"],"style":{"bold":true,"italic":true,"underline":"double","verticalAlign":"superscript","color":"#123456"}})).unwrap();
    let p = export(&mut c, "new-font-order.xlsx");
    let d = parse(p.text("xl/styles.xml").unwrap()).unwrap();
    let fonts = child(d.root_element(), "fonts")
        .unwrap()
        .children()
        .filter(|n| n.is_element())
        .collect::<Vec<_>>();
    assert_eq!(fonts.len(), 2);
    assert!(child(fonts[0], "b").is_none());
    assert_eq!(attr(child(fonts[0], "name").unwrap(), "val"), Some("Aptos"));
    let names = fonts[1]
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["b", "i", "u", "vertAlign", "sz", "color", "name"]
    );
    assert_eq!(
        crate::xlsx::import(&p).unwrap().0[0]["cells"]["B1"],
        "Unchanged"
    );
}
