use crate::{
    docx, docx_tables,
    office::{parse, raw, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-word-table-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture(name: &str) -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/{name}.docx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    c
}
fn output(c: &mut Core, name: &str) -> Package {
    let p = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":p}))
        .unwrap();
    Package::read(p.to_str().unwrap()).unwrap()
}
fn change(c: &mut Core, doc: Value) {
    c.dispatch(json!({"op":"document","document":doc,"baseRevision":c.workspace["revision"]}))
        .unwrap();
}
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[test]
fn sorted_rows_keep_original_wrappers_properties_and_rich_content() {
    for prefix in ["w", "q"] {
        let mut package = docx::new_package();
        let mut rows = vec![];
        for (i, text) in ["Heading", "Zulu", "Alpha"].iter().enumerate() {
            rows.push(format!("<{prefix}:tr {prefix}:rsidR=\"0000000{i}\"><{prefix}:trPr><{prefix}:cantSplit/><{prefix}:trHeight {prefix}:val=\"{}\"/></{prefix}:trPr><{prefix}:tc><{prefix}:tcPr><{prefix}:tcW {prefix}:w=\"2400\" {prefix}:type=\"dxa\"/><{prefix}:shd {prefix}:val=\"clear\" {prefix}:fill=\"FFF2CC\"/></{prefix}:tcPr><{prefix}:p {prefix}:rsidR=\"11111111\"><{prefix}:pPr><{prefix}:keepNext/></{prefix}:pPr><{prefix}:r><{prefix}:rPr><{prefix}:b/><{prefix}:smallCaps/></{prefix}:rPr><{prefix}:t>{text}</{prefix}:t></{prefix}:r></{prefix}:p></{prefix}:tc></{prefix}:tr>", 300 + i * 20));
        }
        package.set("word/document.xml", format!("<{prefix}:document xmlns:{prefix}=\"{W}\"><{prefix}:body><{prefix}:tbl><{prefix}:tblPr/><{prefix}:tblGrid><{prefix}:gridCol {prefix}:w=\"2400\"/></{prefix}:tblGrid>{}</{prefix}:tbl><{prefix}:sectPr/></{prefix}:body></{prefix}:document>", rows.join("")));
        let package = Package::from_bytes(package.bytes().unwrap()).unwrap();
        let before = docx::import(&package).unwrap().0;
        let mut next = before.clone();
        let source = next["content"][0]["content"].as_array_mut().unwrap();
        source.swap(1, 2);
        let output =
            Package::from_bytes(docx::export(&next, Some(&package), Some(&before)).unwrap())
                .unwrap();
        let xml = output.text("word/document.xml").unwrap();
        let parsed = parse(xml).unwrap();
        let actual: Vec<_> = parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "tr")))
            .map(|n| raw(xml, n))
            .collect();
        assert_eq!(
            actual,
            [rows[0].as_str(), rows[2].as_str(), rows[1].as_str()]
        );
        assert_eq!(
            crate::workspace::document_text(&docx::import(&output).unwrap().0),
            "Heading\nAlpha\nZulu"
        );
        std::fs::write(
            folder().join(format!("sorted-rows-{prefix}-source.docx")),
            package.original,
        )
        .unwrap();
        std::fs::write(
            folder().join(format!("sorted-rows-{prefix}.docx")),
            output.original,
        )
        .unwrap();
    }
}

#[test]
fn row_breaks_preserve_extensions_boolean_spellings_and_property_order() {
    use crate::docx_table_format as fmt;
    for (val, expected) in [
        ("", true),
        ("on", true),
        ("true", true),
        ("1", true),
        ("off", false),
        ("false", false),
        ("0", false),
    ] {
        let val = if val.is_empty() {
            String::new()
        } else {
            format!(" q:val=\"{val}\"")
        };
        let props = format!("<q:trPr xmlns:q=\"{W}\" xmlns:x=\"urn:retained\"><q:cantSplit{val} x:note=\"keep\"><x:extra/></q:cantSplit><q:trHeight q:val=\"300\"/><q:tblHeader/><x:other/></q:trPr>");
        let xml = format!("<q:tr xmlns:q=\"{W}\">{props}</q:tr>");
        let mut before = json!({"type":"tableRow", "attrs":{}});
        fmt::read(parse(&xml).unwrap().root_element(), &mut before);
        assert_eq!(before["attrs"]["cantSplit"], expected);
        assert_eq!(
            fmt::patch(props.clone(), "", "trPr", &before, Some(&before), None),
            props
        );
        let mut after = before.clone();
        after["attrs"]["cantSplit"] = json!(!expected);
        let out = fmt::patch(props, "", "trPr", &after, Some(&before), None);
        let doc = parse(&out).unwrap();
        let flag = doc
            .descendants()
            .find(|n| n.has_tag_name((W, "cantSplit")))
            .unwrap();
        assert_eq!(
            flag.attribute((W, "val")),
            Some(if expected { "off" } else { "on" })
        );
        assert_eq!(flag.attribute(("urn:retained", "note")), Some("keep"));
        assert!(flag
            .children()
            .any(|n| n.has_tag_name(("urn:retained", "extra"))));
        let tags = doc
            .root_element()
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>();
        assert_eq!(tags, ["cantSplit", "trHeight", "tblHeader", "other"]);
    }
}

#[test]
fn row_breaks_roundtrip_save_history_and_reject_invalid_values_without_mutation() {
    for name in ["word-tables", "word-tables-prefixed"] {
        let mut c = fixture(name);
        let original = output(&mut c, &format!("row-breaks-{name}-original.docx"));
        let baseline = c.workspace["document"].clone();
        let mut doc = baseline.clone();
        doc["content"][1]["content"][0]["attrs"]["cantSplit"] = json!(false);
        doc["content"][1]["content"][1]["attrs"]["cantSplit"] = json!(true);
        change(&mut c, doc);
        let changed = c.workspace.clone();
        for invalid in [Value::Null, json!("true"), json!(1), json!({})] {
            let mut doc = changed["document"].clone();
            doc["content"][1]["content"][0]["attrs"]["cantSplit"] = invalid;
            assert!(c
                .dispatch(
                    json!({"op":"document", "document":doc, "baseRevision":c.workspace["revision"]})
                )
                .is_err());
            assert_eq!(c.workspace, changed);
        }
        let edited = output(&mut c, &format!("row-breaks-{name}-edited.docx"));
        let imported = docx::import(&edited).unwrap().0;
        assert_eq!(
            imported["content"][1]["content"][0]["attrs"]["cantSplit"],
            false
        );
        assert_eq!(
            imported["content"][1]["content"][1]["attrs"]["cantSplit"],
            true
        );
        for tag in ["tblPr", "tblGrid", "p", "vMerge", "trHeight", "tblHeader"] {
            assert_eq!(properties(&original, tag), properties(&edited, tag));
        }
        for (part, bytes) in &original.entries {
            if part != "word/document.xml" {
                assert_eq!(edited.entries.get(part), Some(bytes));
            }
        }
        let path = folder().join(format!("row-breaks-{name}.folio"));
        c.dispatch(json!({"op":"save", "path":path})).unwrap();
        let mut restored = Core::new();
        restored
            .dispatch(json!({"op":"load", "path":path}))
            .unwrap();
        assert_eq!(
            output(&mut restored, &format!("row-breaks-{name}-restored.docx")).original,
            edited.original
        );
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["document"], baseline);
        assert_eq!(
            output(&mut c, &format!("row-breaks-{name}-undo.docx")).original,
            original.original
        );
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert_eq!(
            output(&mut c, &format!("row-breaks-{name}-redo.docx")).original,
            edited.original
        );
    }
}

#[test]
fn row_breaks_style_chain_banding_and_direct_override_remain_separate() {
    let mut c = fixture("word-tables");
    let mut pkg = output(&mut c, "row-break-style-base.docx");
    let styles = format!("<w:styles xmlns:w=\"{W}\"><w:style w:type=\"table\" w:styleId=\"Base\" w:default=\"1\"><w:name w:val=\"Base\"/><w:trPr><w:cantSplit/></w:trPr></w:style><w:style w:type=\"table\" w:styleId=\"Child\"><w:name w:val=\"Child\"/><w:basedOn w:val=\"Base\"/><w:tblStylePr w:type=\"band1Horz\"><w:trPr><w:cantSplit w:val=\"off\"/></w:trPr></w:tblStylePr><w:tblStylePr w:type=\"firstRow\"><w:trPr><w:cantSplit/></w:trPr></w:tblStylePr></w:style></w:styles>");
    pkg.set("word/styles.xml", styles.clone());
    let rows = (0..5)
        .map(|i| {
            format!(
                "<w:tr>{}<w:tc><w:p><w:r><w:t>Row{i}</w:t></w:r></w:p></w:tc></w:tr>",
                if i == 0 {
                    "<w:trPr><w:cantSplit w:val=\"off\"/></w:trPr>"
                } else {
                    ""
                }
            )
        })
        .collect::<String>();
    pkg.set("word/document.xml", format!("<w:document xmlns:w=\"{W}\"><w:body><w:tbl><w:tblPr><w:tblStyle w:val=\"Child\"/><w:tblLook w:val=\"0020\"/></w:tblPr><w:tblGrid><w:gridCol w:w=\"2400\"/></w:tblGrid>{rows}</w:tbl><w:sectPr/></w:body></w:document>"));
    let doc = docx::import(&pkg).unwrap().0;
    let rs = doc["content"][0]["content"].as_array().unwrap();
    assert_eq!(
        rs.iter()
            .map(|r| r["attrs"]["inheritedCantSplit"].as_bool().unwrap())
            .collect::<Vec<_>>(),
        [true, false, true, false, true]
    );
    assert_eq!(rs[0]["attrs"]["cantSplit"], false);
    assert!(rs[1]["attrs"].get("cantSplit").is_none());
    // Defaults apply even when no table style is named; cycles terminate.
    let xml = pkg
        .text("word/document.xml")
        .unwrap()
        .replace("<w:tblStyle w:val=\"Child\"/>", "");
    pkg.set("word/document.xml", xml);
    let doc = docx::import(&pkg).unwrap().0;
    assert!(doc["content"][0]["content"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["attrs"]["inheritedCantSplit"] == true));
    pkg.set(
        "word/styles.xml",
        styles.replace(
            "<w:name w:val=\"Base\"/>",
            "<w:name w:val=\"Base\"/><w:basedOn w:val=\"Child\"/>",
        ),
    );
    assert!(docx::import(&pkg).is_ok());
}

#[test]
fn word_table_headers_roundtrip_save_undo_and_reject_invalid_flags_atomically() {
    for name in ["word-tables", "word-tables-prefixed"] {
        let mut c = fixture(name);
        let baseline = c.workspace["document"].clone();
        assert_eq!(
            baseline["content"][1]["content"][0]["attrs"]["header"],
            true
        );
        assert!(baseline["content"][1]["content"][1]["attrs"]
            .get("header")
            .is_none());
        let original = output(&mut c, &format!("headers-{name}-original.docx"));
        let mut next = baseline.clone();
        next["content"][1]["content"][0]["attrs"]["header"] = json!(false);
        next["content"][1]["content"][1]["attrs"]["header"] = json!(true);
        change(&mut c, next);
        let changed = c.workspace.clone();
        for invalid in [json!("true"), json!(1), Value::Null, json!({})] {
            let mut bad = changed["document"].clone();
            bad["content"][1]["content"][1]["attrs"]["header"] = invalid;
            assert!(c
                .dispatch(
                    json!({"op":"document", "document":bad, "baseRevision":c.workspace["revision"]})
                )
                .is_err());
            assert_eq!(c.workspace, changed);
        }
        let edited = output(&mut c, &format!("headers-{name}-edited.docx"));
        let reopened = docx::import(&edited).unwrap().0;
        assert_eq!(
            reopened["content"][1]["content"][0]["attrs"]["header"],
            false
        );
        assert_eq!(
            reopened["content"][1]["content"][1]["attrs"]["header"],
            true
        );
        for tag in ["tblPr", "tblGrid", "p", "vMerge"] {
            assert_eq!(
                properties(&original, tag),
                properties(&edited, tag),
                "changed {tag}"
            );
        }
        // Rebuilt table cells acquire explicit namespace scopes, without
        // changing the cell properties or their unsupported descendants.
        fn signature(n: roxmltree::Node) -> String {
            let mut attrs = n
                .attributes()
                .map(|a| (a.namespace(), a.name(), a.value()))
                .collect::<Vec<_>>();
            attrs.sort();
            format!(
                "{:?}:{attrs:?}:{:?}:{}",
                n.tag_name(),
                n.text(),
                n.children()
                    .filter(|c| c.is_element())
                    .map(signature)
                    .collect::<String>()
            )
        }
        let canonical = |p: &Package| {
            let d = parse(p.text("word/document.xml").unwrap()).unwrap();
            d.descendants()
                .filter(|n| n.has_tag_name((W, "tcPr")))
                .map(signature)
                .collect::<Vec<_>>()
        };
        assert_eq!(canonical(&original), canonical(&edited));
        for (part, bytes) in &original.entries {
            if part != "word/document.xml" {
                assert_eq!(edited.entries.get(part), Some(bytes));
            }
        }
        let path = folder().join(format!("headers-{name}.folio"));
        c.dispatch(json!({"op":"save", "path":path})).unwrap();
        let mut restored = Core::new();
        restored
            .dispatch(json!({"op":"load", "path":path}))
            .unwrap();
        assert_eq!(
            output(&mut restored, &format!("headers-{name}-restored.docx")).original,
            edited.original
        );
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["document"], baseline);
        assert_eq!(
            output(&mut c, &format!("headers-{name}-undo.docx")).original,
            original.original
        );
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert_eq!(
            output(&mut c, &format!("headers-{name}-redo.docx")).original,
            edited.original
        );
    }
}

#[test]
fn word_table_headers_boolean_spelling_and_property_extensions_survive_changes() {
    use crate::docx_table_format as fmt;
    for (value, expected) in [
        ("", true),
        ("1", true),
        ("true", true),
        ("on", true),
        ("0", false),
        ("false", false),
        ("off", false),
    ] {
        let val = if value.is_empty() {
            String::new()
        } else {
            format!(" x:val=\"{value}\"")
        };
        let props = format!("<x:trPr xmlns:x=\"{W}\" xmlns:z=\"urn:folio:test\" z:row=\"retained\"><x:cantSplit/><x:tblHeader{val} z:custom=\"retained\"><z:opaque/></x:tblHeader><z:extra/></x:trPr>");
        let xml = format!("<x:tr xmlns:x=\"{W}\">{props}</x:tr>");
        let mut before = json!({"type":"tableRow", "attrs":{}});
        fmt::read(parse(&xml).unwrap().root_element(), &mut before);
        assert_eq!(before["attrs"]["header"], expected);
        assert_eq!(
            fmt::patch(props.clone(), "", "trPr", &before, Some(&before), None),
            props
        );
        let mut after = before.clone();
        after["attrs"]["header"] = json!(!expected);
        let output = fmt::patch(props, "", "trPr", &after, Some(&before), None);
        let d = parse(&output).unwrap();
        let header = d
            .descendants()
            .find(|n| n.has_tag_name((W, "tblHeader")))
            .unwrap();
        assert_eq!(
            header.attribute((W, "val")),
            Some(if expected { "off" } else { "on" })
        );
        assert_eq!(
            header.attribute(("urn:folio:test", "custom")),
            Some("retained")
        );
        assert_eq!(
            d.root_element().attribute(("urn:folio:test", "row")),
            Some("retained")
        );
        assert!(header
            .children()
            .any(|n| n.has_tag_name(("urn:folio:test", "opaque"))));
        assert!(d.descendants().any(|n| n.has_tag_name((W, "cantSplit"))));
        assert!(d
            .descendants()
            .any(|n| n.has_tag_name(("urn:folio:test", "extra"))));
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "tblHeader")))
                .count(),
            1
        );
    }
}

#[test]
fn word_table_headers_legacy_hydration_keeps_explicit_false() {
    let mut c = fixture("word-tables");
    let path = folder().join("headers-legacy.folio");
    c.dispatch(json!({"op":"save", "path":path})).unwrap();
    let disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for explicit in [false, true] {
        let mut legacy = disk.clone();
        legacy["officeDocument"]["baseline"]["content"][1]["content"][0]["attrs"]
            .as_object_mut()
            .unwrap()
            .remove("header");
        let a = legacy["document"]["content"][1]["content"][0]["attrs"]
            .as_object_mut()
            .unwrap();
        a.remove("header");
        if explicit {
            a.insert("header".into(), json!(false));
        }
        std::fs::write(&path, legacy.to_string()).unwrap();
        let mut restored = Core::new();
        restored
            .dispatch(json!({"op":"load", "path":path}))
            .unwrap();
        assert_eq!(
            restored.workspace["document"]["content"][1]["content"][0]["attrs"]["header"],
            !explicit
        );
        assert_eq!(
            restored.workspace["officeDocument"]["baseline"]["content"][1]["content"][0]["attrs"]
                ["header"],
            true
        );
        let package = output(&mut restored, &format!("headers-legacy-{explicit}.docx"));
        assert_eq!(
            docx::import(&package).unwrap().0["content"][1]["content"][0]["attrs"]["header"],
            !explicit
        );
    }
}

#[test]
fn word_table_headers_native_document_reimports_rich_headings_and_page_fields() {
    let text = |s: String| json!({"type":"paragraph", "content":[{"type":"text", "text":s}]});
    let cell = |content: Value| json!({"type":"tableCell", "attrs":{"colspan":1,"rowspan":1}, "content":[content]});
    let mut rows = vec![
        json!({"type":"tableRow", "attrs":{"header":true}, "content":[{
            "type":"tableCell", "attrs":{"colspan":2,"rowspan":1,"fill":"D9EAF7"},
            "content":[{"type":"paragraph", "content":[{"type":"text", "text":"Quarterly accounts", "marks":[{"type":"bold"}]}]}]
        }]}),
        json!({"type":"tableRow", "attrs":{"header":true}, "content":[cell(text("Account".into())), cell(json!({
            "type":"paragraph", "content":[{"type":"text","text":"Amount - page "},
                {"type":"pageField", "text":"1", "attrs":{"instruction":" PAGE ", "kind":"PAGE", "locked":false, "dirty":true}},
                {"type":"text","text":" of "},
                {"type":"pageField", "text":"1", "attrs":{"instruction":" NUMPAGES ", "kind":"NUMPAGES", "locked":false, "dirty":true}}]
        }))]}),
    ];
    for i in 0..45 {
        rows.push(json!({"type":"tableRow", "content":[cell(text(format!("Account {i:02}"))), cell(text(format!("{}", i * 100)))]}));
    }
    let mut c = Core::new();
    change(
        &mut c,
        json!({"type":"doc", "content":[{"type":"table", "attrs":{"columnWidths":[3600,3600]}, "content":rows}]}),
    );
    let package = output(&mut c, "headers-preview-source.docx");
    let imported = docx::import(&package).unwrap().0;
    let table = &imported["content"][0];
    assert_eq!(table["content"].as_array().unwrap().len(), 47);
    assert_eq!(table["content"][0]["content"][0]["attrs"]["colspan"], 2);
    assert_eq!(table["content"][0]["attrs"]["header"], true);
    assert_eq!(table["content"][1]["attrs"]["header"], true);
    assert_eq!(
        table["content"][0]["content"][0]["content"][0]["content"][0]["marks"],
        json!([{"type":"bold"}])
    );
    assert!(crate::workspace::document_text(&imported).contains("Account 44"));
    assert_eq!(properties(&package, "fldSimple").len(), 2);
}

fn properties(p: &Package, name: &str) -> Vec<String> {
    let xml = p.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    d.descendants()
        .filter(|n| n.has_tag_name((W, name)))
        .map(|n| raw(xml, n).to_string())
        .collect()
}
#[test]
fn word_tables_merged_import_rich_edit_preserves_source_properties_and_undo() {
    let mut c = fixture("word-tables");
    let doc = c.workspace["document"].clone();
    let table = &doc["content"][1];
    assert_eq!(table["type"], "table");
    let (cols, cells) = docx_tables::grid(table).unwrap();
    assert_eq!(cols, 4);
    assert_eq!(cells.len(), 8);
    assert_eq!(cells[0].rows, 2);
    assert_eq!(cells[0].cols, 2);
    assert_eq!(
        table["attrs"]["columnWidths"],
        json!([1300, 1900, 2100, 1700])
    );
    c.dispatch(json!({"op":"title","title":"Word table verification sample"}))
        .unwrap();
    c.dispatch(json!({"op":"save","path":folder().join("native-preview.folio")}))
        .unwrap();
    let original = output(&mut c, "original.docx");
    let mut next = doc.clone();
    next["content"][1]["content"][2]["content"][1]["content"][0]["content"][0]["text"] =
        json!("Edited rich cell");
    change(&mut c, next);
    let edited = output(&mut c, "edited.docx");
    for name in [
        "tblPr", "tblGrid", "trPr", "shd", "vAlign", "keepNext", "ind",
    ] {
        assert_eq!(
            properties(&original, name),
            properties(&edited, name),
            "changed {name}"
        );
    }
    for (part, bytes) in &original.entries {
        if part != "word/document.xml" {
            assert_eq!(Some(bytes), edited.entries.get(part));
        }
    }
    let reopened = docx::import(&edited).unwrap().0;
    let cell = &reopened["content"][1]["content"][2]["content"][1];
    assert_eq!(cell["content"][0]["content"][0]["text"], "Edited rich cell");
    assert_eq!(
        cell["content"][0]["content"][0]["marks"],
        json!([{"type":"italic"}])
    );
    assert_eq!(
        cell["content"][1]["content"][0]["marks"],
        json!([{"type":"bold"}])
    );
    c.dispatch(json!({"op":"save","path":folder().join("edited.folio")}))
        .unwrap();
    let mut loaded = Core::new();
    loaded
        .dispatch(json!({"op":"load","path":folder().join("edited.folio")}))
        .unwrap();
    assert_eq!(
        output(&mut loaded, "reopened.docx").original,
        edited.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(output(&mut c, "undone.docx").original, original.original);
}
#[test]
fn word_tables_native_rowspans_rich_paragraphs_and_invalid_edits_are_atomic() {
    let mut c = Core::new();
    let doc = json!({"type":"doc","content":[{"type":"table","attrs":{"columnWidths":[1400,2800]},"content":[
     {"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":1,"rowspan":2},"content":[{"type":"paragraph","content":[{"type":"text","text":"Merged","marks":[{"type":"bold"}]}]},{"type":"paragraph","content":[{"type":"text","text":"Second","marks":[{"type":"italic"}]}]}]},{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"Top right"}]}]}]},
     {"type":"tableRow","content":[{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"Bottom right"}]}]}]}
    ]}]});
    change(&mut c, doc);
    let original = c.workspace.clone();
    let mut invalid = original["document"].clone();
    invalid["content"][0]["content"][0]["content"][0]["attrs"]["rowspan"] = json!(3);
    assert!(c
        .dispatch(
            json!({"op":"document","document":invalid,"baseRevision":c.workspace["revision"]})
        )
        .is_err());
    assert_eq!(c.workspace, original);
    let html = c.dispatch(json!({"op":"exportHtml"})).unwrap();
    let html = html["text"].as_str().unwrap();
    assert!(html.contains("rowspan=\"2\""));
    std::fs::write(folder().join("native.html"), html).unwrap();
    let export = output(&mut c, "native.docx");
    let imported = docx::import(&export).unwrap().0;
    let (_, grid) = docx_tables::grid(&imported["content"][0]).unwrap();
    assert_eq!(grid.len(), 3);
    assert_eq!(grid[0].rows, 2);
    assert_eq!(grid[2].col, 1);
}
#[test]
fn word_tables_alternate_prefix_and_malformed_merge_preservation() {
    let mut c = fixture("word-tables-prefixed");
    assert_eq!(c.workspace["document"]["content"][1]["type"], "table");
    let mut doc = c.workspace["document"].clone();
    doc["content"][1]["content"][2]["content"][0]["content"][0]["content"][0]["text"] =
        json!("Prefix edit");
    change(&mut c, doc);
    let out = output(&mut c, "prefixed.docx");
    assert!(
        crate::workspace::document_text(&docx::import(&out).unwrap().0).contains("Prefix edit")
    );
    let mut bad = out.clone();
    let source = bad.text("word/document.xml").unwrap();
    let d = parse(source).unwrap();
    let start = d
        .descendants()
        .find(|n| n.has_tag_name((W, "vMerge")) && n.attribute((W, "val")) == Some("restart"))
        .unwrap();
    let value = start
        .attributes()
        .find(|a| a.namespace() == Some(W) && a.name() == "val")
        .unwrap();
    let name = source[value.range()].split('=').next().unwrap();
    let xml = crate::office::replace_ranges(
        source,
        vec![(value.range(), format!("{name}=\"continue\""))],
    );
    bad.set("word/document.xml", xml);
    let bad = Package::from_bytes(bad.bytes().unwrap()).unwrap();
    let doc = docx::import(&bad).unwrap().0;
    assert_eq!(doc["content"][1]["type"], "officeBlock");
    assert_eq!(
        docx::export(&doc, Some(&bad), Some(&doc)).unwrap(),
        bad.original
    );
}

#[test]
fn word_tables_legacy_workspaces_hydrate_only_missing_source_identity() {
    let mut c = fixture("word-tables");
    let original = output(&mut c, "legacy-original.docx");
    let mut doc = c.workspace["document"].clone();
    doc["content"][1]["content"][2]["content"][1]["content"][0]["content"][0]["text"] =
        json!("Legacy saved edit");
    change(&mut c, doc);
    let path = folder().join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    fn strip(v: &mut Value) {
        if v["type"] == "table" {
            if let Some(a) = v["attrs"].as_object_mut() {
                a.remove("columnWidths");
            }
        }
        if matches!(
            v["type"].as_str(),
            Some("tableRow" | "tableCell" | "tableHeader")
        ) {
            if let Some(a) = v["attrs"].as_object_mut() {
                a.remove("officeId");
            }
        }
        if let Some(a) = v.get_mut("content").and_then(Value::as_array_mut) {
            for n in a {
                strip(n);
            }
        }
    }
    strip(&mut disk["document"]);
    strip(&mut disk["officeDocument"]["baseline"]);
    std::fs::write(&path, disk.to_string()).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        loaded.workspace["document"]["content"][1]["attrs"]["columnWidths"],
        json!([1300, 1900, 2100, 1700])
    );
    let out = output(&mut loaded, "legacy.docx");
    assert!(
        crate::workspace::document_text(&docx::import(&out).unwrap().0)
            .contains("Legacy saved edit")
    );
    for prop in [
        "tblPr", "tblGrid", "trPr", "tcW", "shd", "vAlign", "keepNext", "ind",
    ] {
        assert_eq!(properties(&original, prop), properties(&out, prop));
    }
}

#[test]
fn word_tables_layout_format_roundtrip_save_undo_and_atomic_validation() {
    for name in ["word-tables", "word-tables-prefixed"] {
        let mut c = fixture(name);
        let original = output(&mut c, &format!("layout-{name}-original.docx"));
        let mut doc = c.workspace["document"].clone();
        let t = &mut doc["content"][1];
        assert_eq!(t["content"][0]["content"][0]["attrs"]["fill"], "D9EAF7");
        assert_eq!(
            t["content"][0]["content"][0]["attrs"]["verticalAlign"],
            "center"
        );
        t["attrs"]["layout"] = json!("fixed");
        t["attrs"]["alignment"] = json!("right");
        t["attrs"]["columnWidths"] = json!([1800, 1800, 2100, 1700]);
        t["content"][0]["attrs"]["height"] = json!({"rule":"exact","value":480});
        t["content"][1]["attrs"]["height"] = json!({"rule":"atLeast","value":900});
        t["content"][2]["attrs"]["height"] = json!({"rule":"auto","value":0});
        t["content"][0]["content"][0]["attrs"]["fill"] = json!("FFF2CC");
        t["content"][0]["content"][0]["attrs"]["verticalAlign"] = json!("bottom");
        change(&mut c, doc);
        let edited = output(&mut c, &format!("layout-{name}.docx"));
        let imported = docx::import(&edited).unwrap().0;
        let t = &imported["content"][1];
        assert_eq!(t["attrs"]["layout"], "fixed");
        assert_eq!(t["attrs"]["alignment"], "right");
        assert_eq!(t["attrs"]["columnWidths"], json!([1800, 1800, 2100, 1700]));
        assert_eq!(
            t["content"][0]["attrs"]["height"],
            json!({"rule":"exact","value":480})
        );
        assert_eq!(
            t["content"][1]["attrs"]["height"],
            json!({"rule":"atLeast","value":900})
        );
        assert_eq!(
            t["content"][2]["attrs"]["height"],
            json!({"rule":"auto","value":0})
        );
        let a = &t["content"][0]["content"][0]["attrs"];
        assert_eq!(a["fill"], "FFF2CC");
        assert_eq!(a["verticalAlign"], "bottom");
        let continuation = a["mergeContinuations"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(continuation["attrs"]["fill"], "FFF2CC");
        for part in original
            .entries
            .keys()
            .filter(|k| *k != "word/document.xml")
        {
            assert_eq!(original.entries[part], edited.entries[part]);
        }
        for property in ["tblHeader", "keepNext", "ind"] {
            assert_eq!(
                properties(&original, property),
                properties(&edited, property)
            );
        }
        let path = folder().join(format!("layout-{name}.folio"));
        c.dispatch(json!({"op":"save","path":path})).unwrap();
        let mut loaded = Core::new();
        loaded.dispatch(json!({"op":"load","path":path})).unwrap();
        assert_eq!(
            output(&mut loaded, &format!("layout-{name}-reopened.docx")).original,
            edited.original
        );
        let before = c.workspace.clone();
        for (key, value) in [
            ("fill", json!("bad\"color")),
            ("verticalAlign", json!("sideways")),
        ] {
            let mut bad = before["document"].clone();
            bad["content"][1]["content"][0]["content"][0]["attrs"][key] = value;
            assert!(c.dispatch(json!({"op":"document","document":bad})).is_err());
            assert_eq!(c.workspace, before);
        }
        let mut bad = before["document"].clone();
        bad["content"][1]["content"][0]["attrs"]["height"] = json!({"rule":"exact","value":0});
        assert!(c.dispatch(json!({"op":"document","document":bad})).is_err());
        assert_eq!(c.workspace, before);
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(
            output(&mut c, &format!("layout-{name}-undone.docx")).original,
            original.original
        );
        c.dispatch(json!({"op":"redo"})).unwrap();
        let mut reset = c.workspace["document"].clone();
        reset["content"][1]["content"][0]["content"][0]["attrs"]["fill"] = json!("auto");
        reset["content"][1]["content"][0]["attrs"]["height"] = json!({"rule":"auto","value":0});
        change(&mut c, reset);
        let reset = docx::import(&output(&mut c, &format!("layout-{name}-reset.docx")))
            .unwrap()
            .0;
        assert_eq!(
            reset["content"][1]["content"][0]["content"][0]["attrs"]["fill"],
            "auto"
        );
        assert_eq!(
            reset["content"][1]["content"][0]["attrs"]["height"]["rule"],
            "auto"
        );
    }
}

#[test]
fn word_tables_legacy_format_hydration_keeps_explicit_overrides() {
    let mut c = fixture("word-tables");
    let mut doc = c.workspace["document"].clone();
    doc["content"][1]["attrs"]["layout"] = json!("fixed");
    doc["content"][1]["content"][0]["attrs"]["height"] = json!({"rule":"exact","value":600});
    change(&mut c, doc);
    let exported = output(&mut c, "layout-hydration-source.docx");
    let mut imported = Core::new();
    imported
        .dispatch(json!({"op":"importOffice","path":folder().join("layout-hydration-source.docx")}))
        .unwrap();
    let path = folder().join("layout-legacy.folio");
    imported.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for key in ["document", "officeDocument"] {
        let doc = if key == "document" {
            &mut disk[key]
        } else {
            &mut disk[key]["baseline"]
        };
        let t = &mut doc["content"][1];
        t["attrs"].as_object_mut().unwrap().remove("layout");
        t["content"][0]["attrs"]
            .as_object_mut()
            .unwrap()
            .remove("height");
        t["content"][0]["content"][0]["attrs"]
            .as_object_mut()
            .unwrap()
            .remove("fill");
    }
    disk["document"]["content"][1]["content"][2]["content"][1]["attrs"]["fill"] = json!("auto");
    std::fs::write(&path, disk.to_string()).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    let t = &loaded.workspace["document"]["content"][1];
    assert_eq!(t["attrs"]["layout"], "fixed");
    assert_eq!(t["content"][0]["attrs"]["height"]["value"], 600);
    assert_eq!(t["content"][0]["content"][0]["attrs"]["fill"], "D9EAF7");
    assert_eq!(t["content"][2]["content"][1]["attrs"]["fill"], "auto");
    let actual = output(&mut loaded, "layout-legacy.docx");
    assert_ne!(actual.original, exported.original);
}

#[test]
fn word_table_format_edits_retain_local_property_namespaces_and_attributes() {
    let mut package = Package::read(&format!(
        "{}/../test/fixtures/word-tables.docx",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut xml = package.text("word/document.xml").unwrap().to_string();
    for kind in ["tblPr", "trPr", "tcPr"] {
        xml = xml.replacen(
            &format!("<w:{kind}>"),
            &format!("<w:{kind} xmlns:local=\"urn:folio:test:retained\" local:flag=\"{kind}\">"),
            1,
        );
    }
    package.set("word/document.xml", xml);
    let package = Package::from_bytes(package.bytes().unwrap()).unwrap();
    let original = docx::import(&package).unwrap().0;
    let mut edited = original.clone();
    let t = &mut edited["content"][1];
    t["attrs"]["alignment"] = json!("right");
    t["content"][0]["attrs"]["height"] = json!({"rule":"exact","value":480});
    t["content"][0]["content"][0]["attrs"]["fill"] = json!("FFF2CC");
    let bytes = docx::export(&edited, Some(&package), Some(&original)).unwrap();
    let output = Package::from_bytes(bytes).unwrap();
    let d = parse(output.text("word/document.xml").unwrap()).unwrap();
    for kind in ["tblPr", "trPr", "tcPr"] {
        let n = d.descendants().find(|n| n.has_tag_name((W, kind))).unwrap();
        assert_eq!(n.attribute(("urn:folio:test:retained", "flag")), Some(kind));
    }
}
