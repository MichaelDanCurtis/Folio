use crate::{docx, docx_page_setup as pages, docx_revisions as rev, office::*, workspace::Core};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn columns(equal: bool) -> Value {
    json!({"count":2,"equalWidth":equal,"space":720,"separator":true,"rtl":false,
        "columns":if equal {vec![]} else {vec![json!({"width":2640,"space":720}),json!({"width":6000,"space":0})]}})
}

#[test]
fn column_definitions_read_custom_counts_and_preserve_source_scopes() {
    let mut p = fixture();
    p.set("word/document.xml",p.text("word/document.xml").unwrap().replace("<q:cols q:num=\"2\" q:space=\"720\"/>",
        "<q:cols xmlns:ext=\"urn:folio-keep\" q:num=\"19\" q:equalWidth=\"0\" ext:keep=\"columns\"><q:col q:w=\"2in\" q:space=\"36pt\" ext:keep=\"first\"/><q:col q:w=\"5760\"/></q:cols>"));
    let before = pages::read(&p).unwrap();
    assert_eq!(before["sections"][0]["columns"], 2);
    assert_eq!(
        before["sections"][0]["columnLayout"]["columns"][0]["width"],
        2880
    );
    let mut settings = columns(false);
    settings["rtl"] = json!(true);
    let changed =
        pages::apply(&p, &request(&p, 0, json!({"columnLayout":settings})), None).unwrap();
    let after = pages::read(&changed).unwrap();
    assert_eq!(after["sections"][0]["columnLayout"], settings);
    assert_eq!(
        after["sections"][0]["geometry"],
        before["sections"][0]["geometry"]
    );
    assert_eq!(after["sections"][1], before["sections"][1]);
    let d = parse(changed.text("word/document.xml").unwrap()).unwrap();
    let cols = d
        .descendants()
        .find(|n| n.has_tag_name((W, "cols")))
        .unwrap();
    assert_eq!(cols.attribute(("urn:folio-keep", "keep")), Some("columns"));
    assert_eq!(
        child(cols, "col")
            .unwrap()
            .attribute(("urn:folio-keep", "keep")),
        Some("first")
    );
    assert_eq!(
        changed.entries["word/header1.xml"],
        p.entries["word/header1.xml"]
    );
    let equal = pages::apply(
        &changed,
        &request(&changed, 0, json!({"columnLayout":columns(true)})),
        None,
    )
    .unwrap();
    assert_eq!(
        pages::read(&equal).unwrap()["sections"][0]["columnLayout"],
        columns(true)
    );
    assert!(equal
        .text("word/document.xml")
        .unwrap()
        .contains("ext:keep=\"first\""));
    save(&changed, "columns-custom");
    save(&equal, "columns-equal");
}

#[test]
fn column_settings_validate_all_targets_and_noop_without_mutating_source() {
    let p = fixture();
    let original = p.bytes().unwrap();
    let current = pages::read(&p).unwrap()["sections"][0]["columnLayout"].clone();
    assert_eq!(
        pages::apply(&p, &request(&p, 0, json!({"columnLayout":current})), None)
            .unwrap()
            .bytes()
            .unwrap(),
        original
    );
    for (key, value) in [
        ("count", json!(0)),
        ("count", json!(46)),
        ("space", json!(-1)),
        ("space", json!(10000)),
        ("separator", json!("yes")),
        ("columns", json!([{"width":1,"space":0}])),
        ("unknown", json!(true)),
    ] {
        let mut invalid = columns(true);
        invalid[key] = value;
        assert!(pages::apply(&p, &request(&p, 0, json!({"columnLayout":invalid})), None).is_err());
    }
    let mut invalid = columns(false);
    invalid["columns"][0]["width"] = json!(0);
    assert!(pages::apply(&p, &request(&p, 0, json!({"columnLayout":invalid})), None).is_err());
    let mut all = request(&p, 0, json!({"columnLayout":columns(true)}));
    all["scope"] = json!("document");
    let changed = pages::apply(&p, &all, None).unwrap();
    assert!(pages::read(&changed).unwrap()["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["columnLayout"] == columns(true)));
    let mut simultaneous = request(&p, 0, json!({"width":4000,"columnLayout":columns(false)}));
    assert!(pages::apply(&p, &simultaneous, None).is_err());
    simultaneous["sectionKey"] = json!("stale");
    assert!(pages::apply(&p, &simultaneous, None).is_err());
    assert_eq!(p.bytes().unwrap(), original);
}

#[test]
fn recorded_column_formatting_accepts_and_rejects_with_exact_prior_settings() {
    let p = fixture();
    let before = pages::read(&p).unwrap();
    let settings = json!({"version":1,"author":"Morgan","enabled":true,"formatting":true});
    let tracked = pages::apply(
        &p,
        &request(&p, 0, json!({"columnLayout":columns(false)})),
        Some(&settings),
    )
    .unwrap();
    let list = rev::list(&tracked).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 1);
    assert_eq!(list["revisions"][0]["tag"], "sectPrChange");
    assert!(list["revisions"][0]["formatChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["property"] == "Column 1 width" && c["after"] == "1.833333 in"));
    assert!(list["revisions"][0]["formatChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["property"] == "Column 2 width" && c["after"] == "4.166667 in"));
    assert!(pages::apply(
        &tracked,
        &request(&tracked, 0, json!({"columnLayout":columns(true)})),
        None
    )
    .is_err());
    for action in ["accept", "reject"] {
        let output = rev::resolve(
            &tracked,
            &json!({"action":action,"keys":[list["revisions"][0]["key"]]}),
        )
        .unwrap();
        assert_eq!(
            pages::read(&output).unwrap()["sections"][0]["columnLayout"],
            if action == "accept" {
                columns(false)
            } else {
                before["sections"][0]["columnLayout"].clone()
            }
        );
        assert_eq!(
            output.entries["word/header1.xml"],
            p.entries["word/header1.xml"]
        );
        save(&output, &format!("columns-{action}"));
    }
    save(&tracked, "columns-tracked");
}

#[test]
fn columns_workspace_preview_pending_body_history_and_saved_recovery() {
    let path = std::env::temp_dir().join("folio-columns-source.docx");
    let p = fixture();
    std::fs::write(&path, p.bytes().unwrap()).unwrap();
    let mut core = Core::default();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut doc = core.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Pending column edit.");
    core.dispatch(json!({"op":"document","document":doc}))
        .unwrap();
    let before = core.workspace["document"].clone();
    let data = core.dispatch(json!({"op":"documentPageSetup"})).unwrap();
    let r = json!({"op":"documentPageSetupApply","workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"],"section":0,"sectionKey":data["sections"][0]["key"],"scope":"section","patch":{"columnLayout":columns(false)}});
    core.dispatch(r.clone()).unwrap();
    let changed = core.workspace["document"].clone();
    assert_eq!(
        changed["content"][0]["content"][0]["text"],
        "Pending column edit."
    );
    assert!(core.dispatch(r).is_err());
    let preview = core.dispatch(json!({"op":"documentPagePreview"})).unwrap();
    assert_eq!(preview["sections"][0]["columnLayout"], columns(false));
    assert_eq!(core.workspace["document"], changed);
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(core.workspace["document"], before);
    core.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(core.workspace["document"], changed);
    let saved = std::env::temp_dir().join("folio-columns.folio");
    core.dispatch(json!({"op":"save","path":saved})).unwrap();
    core.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(core.workspace["document"], changed);
    core.dispatch(json!({"op":"exportOffice","format":"docx","path":std::env::temp_dir().join("folio-columns-core.docx")})).unwrap();
}

#[test]
fn column_balancing_compatibility_uses_related_settings_and_never_rewrites_source() {
    let mut source = docx::new_package();
    source.set("word/_rels/document.xml.rels",source.text("word/_rels/document.xml.rels").unwrap().replace("</Relationships>","<Relationship Id=\"rIdSettings\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/></Relationships>"));
    source.set("[Content_Types].xml",source.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/></Types>"));
    source.set(
        "word/settings.xml",
        format!("<w:settings xmlns:w=\"{W}\"/>"),
    );
    let lines=(0..20).map(|i|format!("<w:p><w:pPr><w:spacing w:before=\"0\" w:after=\"0\"/></w:pPr><w:r><w:t>Line {i:02}.</w:t></w:r></w:p>")).collect::<String>();
    source.set("word/document.xml",format!("<w:document xmlns:w=\"{W}\"><w:body>{lines}<w:p><w:pPr><w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/><w:cols w:num=\"2\" w:space=\"720\" w:sep=\"1\"/></w:sectPr></w:pPr><w:r><w:t>End of columns.</w:t></w:r></w:p><w:p><w:r><w:t>Full-width text follows the balanced columns.</w:t></w:r></w:p><w:sectPr><w:type w:val=\"continuous\"/><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"));
    let settings = crate::docx_tabs::part(&source, "settings").unwrap();
    let before = source.bytes().unwrap();
    assert_eq!(
        crate::docx_headers::page_preview(&source).unwrap()["noColumnBalance"],
        false
    );
    assert_eq!(source.bytes().unwrap(), before);
    std::fs::write(
        std::env::temp_dir().join("folio-column-balance-source.docx"),
        before,
    )
    .unwrap();
    for (value, expected) in [
        ("", true),
        (" w:val=\"1\"", true),
        (" w:val=\"false\"", false),
        (" w:val=\"0\"", false),
    ] {
        let mut changed = source.clone();
        let root=format!("<w:settings xmlns:w=\"{W}\"><w:compat><w:noColumnBalance{value}/></w:compat></w:settings>");
        changed.set(&settings, root.clone());
        let bytes = changed.bytes().unwrap();
        assert_eq!(
            crate::docx_headers::page_preview(&changed).unwrap()["noColumnBalance"],
            expected
        );
        assert_eq!(changed.bytes().unwrap(), bytes);
        if value.is_empty() {
            std::fs::write(
                std::env::temp_dir().join("folio-column-balance-disabled.docx"),
                &bytes,
            )
            .unwrap();
            let rels = changed
                .text("word/_rels/document.xml.rels")
                .unwrap()
                .replace("Target=\"settings.xml\"", "Target=\"options/settings.xml\"");
            assert!(rels.contains("options/settings.xml"));
            changed.set("word/_rels/document.xml.rels", rels);
            changed.set("word/options/settings.xml", root);
            changed.set(&settings, format!("<w:settings xmlns:w=\"{W}\"/>"));
            assert_eq!(
                crate::docx_headers::page_preview(&changed).unwrap()["noColumnBalance"],
                true
            );
        }
    }
}
fn fixture() -> Package {
    let mut p = docx::new_package();
    p.set("word/document.xml",format!(r#"<q:document xmlns:q="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><q:body>
    <q:p><q:r><q:t>First section.</q:t></q:r></q:p>
    <q:p><q:pPr><q:sectPr><q:headerReference q:type="default" r:id="headerRef"/><q:pgSz q:w="12240" q:h="15840" q:code="1"/><q:pgMar q:top="1440" q:right="1440" q:bottom="1440" q:left="1440" q:header="720" q:footer="720" q:gutter="0"/><q:cols q:num="2" q:space="720"/><q:titlePg/></q:sectPr></q:pPr><q:r><q:t>Section boundary.</q:t></q:r></q:p>
    <q:p><q:r><q:t>Second section.</q:t></q:r></q:p>
    <q:sectPr><q:type q:val="continuous"/><q:pgSz q:w="16838" q:h="11906" q:orient="landscape"/><q:pgMar q:top="720" q:right="1080" q:bottom="720" q:left="1080" q:header="360" q:footer="360" q:gutter="0"/><q:pgNumType q:start="3"/></q:sectPr></q:body></q:document>"#));
    p.set(
        "word/header1.xml",
        format!(
            "<q:hdr xmlns:q=\"{W}\"><q:p><q:r><q:t>Preserved header.</q:t></q:r></q:p></q:hdr>"
        ),
    );
    let types=p.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/></Types>");
    p.set("[Content_Types].xml", types);
    let rels=p.text("word/_rels/document.xml.rels").unwrap().replace("</Relationships>","<Relationship Id=\"headerRef\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header1.xml\"/></Relationships>");
    p.set("word/_rels/document.xml.rels", rels);
    p.original = p.bytes().unwrap();
    p
}
fn request(p: &Package, index: usize, patch: Value) -> Value {
    json!({"section":index,"sectionKey":pages::read(p).unwrap()["sections"][index]["key"],"scope":"section","patch":patch})
}
fn save(p: &Package, name: &str) {
    std::fs::write(
        std::env::temp_dir().join(format!("folio-page-{name}.docx")),
        p.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn page_setup_reads_sections_and_patches_only_selected_geometry() {
    let p = fixture();
    let bytes = p.bytes().unwrap();
    let read = pages::read(&p).unwrap();
    assert_eq!(read["sections"].as_array().unwrap().len(), 2);
    assert_eq!(read["sections"][0]["endOfficeId"], "word-1");
    assert_eq!(read["sections"][1]["firstBlock"], 2);
    assert_eq!(read["sections"][0]["columns"], 2);
    let changed=pages::apply(&p,&request(&p,1,json!({"width":11906,"height":16838,"orientation":"portrait","left":1800,"gutter":360,"start":"oddPage"})),None).unwrap();
    let after = pages::read(&changed).unwrap();
    assert_eq!(after["sections"][0], read["sections"][0]);
    assert_eq!(after["sections"][1]["geometry"]["left"], 1800);
    assert_eq!(after["sections"][1]["geometry"]["start"], "oddPage");
    let x = changed.text("word/document.xml").unwrap();
    let d = parse(x).unwrap();
    assert_eq!(
        d.descendants()
            .filter(|n| n.has_tag_name((W, "headerReference")))
            .count(),
        1
    );
    assert_eq!(
        d.descendants()
            .find(|n| n.has_tag_name((W, "pgNumType")))
            .unwrap()
            .attribute((W, "start")),
        Some("3")
    );
    assert_eq!(
        changed.entries["word/header1.xml"],
        p.entries["word/header1.xml"]
    );
    assert_eq!(p.bytes().unwrap(), bytes);
    let mut all = request(&p, 0, json!({"top":1080}));
    all["scope"] = json!("document");
    let both = pages::apply(&p, &all, None).unwrap();
    assert!(pages::read(&both).unwrap()["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["geometry"]["top"] == 1080));
    assert_eq!(
        pages::read(&both).unwrap()["sections"][1]["geometry"]["orientation"],
        "landscape"
    );
    save(&p, "source");
    save(&changed, "selected");
    save(&both, "all");
}
#[test]
fn page_setup_defaults_units_invalid_scope_and_noop_are_atomic() {
    let mut p = fixture();
    p.set(
        "word/document.xml",
        format!("<document xmlns=\"{W}\"><body><p><r><t>Plain.</t></r></p></body></document>"),
    );
    let data = pages::read(&p).unwrap();
    assert_eq!(data["sections"][0]["sizeExplicit"], false);
    let added = pages::apply(
        &p,
        &request(
            &p,
            0,
            json!({"width":15840,"height":12240,"orientation":"landscape"}),
        ),
        None,
    )
    .unwrap();
    assert_eq!(
        pages::read(&added).unwrap()["sections"][0]["geometry"]["width"],
        15840
    );
    save(&added, "created");
    let mut units = fixture();
    units.set(
        "word/document.xml",
        units
            .text("word/document.xml")
            .unwrap()
            .replace("q:w=\"12240\"", "q:w=\"8.5in\"")
            .replace("q:top=\"1440\"", "q:top=\"72pt\""),
    );
    assert_eq!(
        pages::read(&units).unwrap()["sections"][0]["geometry"]["width"],
        12240
    );
    assert_eq!(
        pages::read(&units).unwrap()["sections"][0]["geometry"]["top"],
        1440
    );
    let p = fixture();
    let original = p.bytes().unwrap();
    let noop = pages::apply(&p, &request(&p, 0, json!({"left":1440})), None).unwrap();
    assert_eq!(noop.bytes().unwrap(), original);
    for patch in [
        json!({"width":0}),
        json!({"top":15840}),
        json!({"header":-1}),
        json!({"left":0.5}),
        json!({"orientation":"sideways"}),
        json!({"start":"invalid"}),
        json!({"unknown":true}),
    ] {
        assert!(pages::apply(&p, &request(&p, 0, patch), None).is_err());
        assert_eq!(p.bytes().unwrap(), original);
    }
    let mut stale = request(&p, 0, json!({"left":2000}));
    stale["sectionKey"] = json!("stale");
    assert!(pages::apply(&p, &stale, None).is_err());
    let mut all = request(&p, 0, json!({"top":12500}));
    all["scope"] = json!("document");
    assert!(pages::apply(&p, &all, None).is_err());
    assert_eq!(p.bytes().unwrap(), original);
}
#[test]
fn page_setup_recorded_section_changes_review_without_losing_header_links() {
    let p = fixture();
    let settings = json!({"version":1,"author":"Morgan","enabled":true,"formatting":true});
    let tracked = pages::apply(
        &p,
        &request(
            &p,
            0,
            json!({"width":15840,"height":12240,"orientation":"landscape","left":720}),
        ),
        Some(&settings),
    )
    .unwrap();
    let list = rev::list(&tracked).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 1);
    assert_eq!(list["revisions"][0]["tag"], "sectPrChange");
    assert_eq!(list["revisions"][0]["author"], "Morgan");
    assert!(!list["revisions"][0]["formatChanges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["property"] == "header reference"));
    for (action, width) in [("accept", 15840), ("reject", 12240)] {
        let reviewed = rev::resolve(
            &tracked,
            &json!({"action":action,"keys":[list["revisions"][0]["key"]]}),
        )
        .unwrap();
        assert_eq!(
            pages::read(&reviewed).unwrap()["sections"][0]["geometry"]["width"],
            width
        );
        let d = parse(reviewed.text("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "headerReference")))
                .count(),
            1
        );
        assert!(rev::list(&reviewed).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .is_empty());
        save(&reviewed, &format!("tracked-{action}"));
    }
    assert!(pages::apply(
        &tracked,
        &request(&tracked, 0, json!({"left":1000})),
        Some(&settings)
    )
    .is_err());
    save(&tracked, "tracked");
}
#[test]
fn page_setup_workspace_pending_text_history_recovery_and_stale_guards() {
    let path = std::env::temp_dir().join("folio-page-core-source.docx");
    std::fs::write(&path, fixture().bytes().unwrap()).unwrap();
    let mut c = Core::default();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let imported = c
        .dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut doc = imported["workspace"]["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Pending ordinary edit.");
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let before = c.dispatch(json!({"op":"get"})).unwrap();
    let data = c.dispatch(json!({"op":"documentPageSetup"})).unwrap();
    assert_eq!(c.dispatch(json!({"op":"get"})).unwrap(), before);
    let request = json!({"op":"documentPageSetupApply","workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"],"section":0,"sectionKey":data["sections"][0]["key"],"scope":"section","patch":{"top":1080,"left":1080}});
    let applied = c.dispatch(request.clone()).unwrap();
    assert_eq!(
        applied["workspace"]["document"]["content"][0]["content"][0]["text"],
        "Pending ordinary edit."
    );
    assert!(c.dispatch(request).is_err());
    let undone = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        undone["workspace"]["document"],
        before["workspace"]["document"]
    );
    let redone = c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        redone["workspace"]["document"],
        applied["workspace"]["document"]
    );
    let save = std::env::temp_dir().join("folio-page-workspace.folio");
    c.dispatch(json!({"op":"save","path":save})).unwrap();
    let loaded = c.dispatch(json!({"op":"load","path":save})).unwrap();
    assert_eq!(
        loaded["workspace"]["document"],
        applied["workspace"]["document"]
    );
    let output = std::env::temp_dir().join("folio-page-core-export.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
        .unwrap();
    let p = Package::from_bytes(std::fs::read(output).unwrap()).unwrap();
    assert_eq!(
        pages::read(&p).unwrap()["sections"][0]["geometry"]["left"],
        1080
    );
}

#[test]
fn page_setup_continuous_inheritance_and_namespace_extensions() {
    let mut p = fixture();
    let x = p.text("word/document.xml").unwrap().to_string();
    let d = parse(&x).unwrap();
    let first = d
        .descendants()
        .find(|n| n.has_tag_name((W, "sectPr")))
        .unwrap();
    let patch = format!("<q:sectPr><q:type q:val=\"continuous\"/></q:sectPr>");
    p.set(
        "word/document.xml",
        replace_ranges(&x, vec![(first.range(), patch)]),
    );
    let r = pages::read(&p).unwrap();
    assert_eq!(r["sections"][0]["inheritedPageGeometry"], true);
    assert_eq!(r["sections"][0]["geometry"], r["sections"][1]["geometry"]);
    let updated = pages::apply(&p, &request(&p, 0, json!({"left":900})), None).unwrap();
    assert_eq!(
        pages::read(&updated).unwrap()["sections"][0]["geometry"]["width"],
        16838
    );
    assert_eq!(
        pages::read(&updated).unwrap()["sections"][0]["geometry"]["right"],
        1080
    );
    assert_eq!(
        pages::read(&updated).unwrap()["sections"][1],
        r["sections"][1]
    );
    save(&updated, "inherited");
    let x = format!("<document xmlns=\"{W}\" xmlns:a=\"{W}\" xmlns:z=\"urn:folio:test\"><body><p><r><t>Default namespace.</t></r></p><sectPr><pgSz a:w=\"12240\" a:h=\"15840\" z:retained=\"yes\"/><pgMar a:top=\"1440\" a:right=\"1440\" a:bottom=\"1440\" a:left=\"1440\" a:header=\"720\" a:footer=\"720\" a:gutter=\"0\"/></sectPr></body></document>");
    p.set("word/document.xml", x);
    let updated = pages::apply(
        &p,
        &request(
            &p,
            0,
            json!({"width":15840,"height":12240,"orientation":"landscape"}),
        ),
        None,
    )
    .unwrap();
    let d = parse(updated.text("word/document.xml").unwrap()).unwrap();
    assert_eq!(
        d.descendants()
            .find(|n| n.has_tag_name((W, "pgSz")))
            .unwrap()
            .attribute(("urn:folio:test", "retained")),
        Some("yes")
    );
}

#[test]
fn page_setup_split_paragraph_retains_one_boundary_and_removal_is_rejected() {
    let p = fixture();
    let (before, _) = docx::import(&p).unwrap();
    let mut next = before.clone();
    let mut split = next["content"][1].clone();
    split["content"][0]["text"] = json!("Section ");
    next["content"][1]["content"][0]["text"] = json!("boundary.");
    next["content"].as_array_mut().unwrap().insert(1, split);
    pages::check_model(&before, &next).unwrap();
    let updated =
        Package::from_bytes(docx::export(&next, Some(&p), Some(&before)).unwrap()).unwrap();
    let layout = pages::read(&updated).unwrap();
    assert_eq!(layout["sections"].as_array().unwrap().len(), 2);
    assert_eq!(layout["sections"][0]["endOfficeId"], "word-2");
    let d = parse(updated.text("word/document.xml").unwrap()).unwrap();
    let boundary = d
        .descendants()
        .find(|n| n.has_tag_name((W, "sectPr")))
        .unwrap();
    assert!(
        crate::workspace::document_text(&docx::import(&updated).unwrap().0)
            .contains("Section \nboundary.")
    );
    assert!(raw(
        updated.text("word/document.xml").unwrap(),
        boundary.parent().unwrap().parent().unwrap()
    )
    .contains("boundary."));
    save(&updated, "split");
    next["content"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["attrs"]["officeId"] != "word-1");
    assert!(pages::check_model(&before, &next).is_err());
    assert!(docx::export(&next, Some(&p), Some(&before)).is_err());
}

#[test]
fn page_setup_core_recording_review_recovery_and_live_workbook() {
    let input = std::env::temp_dir().join("folio-page-recording-source.docx");
    std::fs::write(&input, fixture().bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":input}))
        .unwrap();
    c.dispatch(json!({"op":"documentTracking","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"author":"Alex","enabled":true,"formatting":true})).unwrap();
    let before = c.workspace["document"].clone();
    let data = c.dispatch(json!({"op":"documentPageSetup"})).unwrap();
    c.dispatch(json!({"op":"documentPageSetupApply","workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"],"section":0,"sectionKey":data["sections"][0]["key"],"scope":"section","patch":{"left":720}})).unwrap();
    let list = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    assert_eq!(list["revisions"][0]["author"], "Alex");
    assert_eq!(list["revisions"].as_array().unwrap().len(), 1);
    let recovery = std::env::temp_dir().join("folio-page-tracked.folio");
    c.dispatch(json!({"op":"save","path":recovery})).unwrap();
    let applied = c.workspace["document"].clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], applied);
    let mut c = Core::new();
    c.dispatch(json!({"op":"load","path":recovery})).unwrap();
    assert_eq!(c.workspace["document"], applied);
    c.dispatch(json!({"op":"documentRevision","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"reject","keys":[list["revisions"][0]["key"]]})).unwrap();
    assert_eq!(
        c.workspace["document"]["attrs"][pages::KEY]["sections"][0]["geometry"]["left"],
        1440
    );
    let out = std::env::temp_dir().join("folio-page-core-tracked-reject.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    // A separately imported untracked document can retain a live workbook block.
    c.dispatch(json!({"op":"importOffice","path":input}))
        .unwrap();
    let sheet = c.workspace["sheets"][0]["id"].clone();
    let linked = json!({"type":"workbookBlock","attrs":{"sheetId":sheet,"range":"A1:B2","title":"Live figures"}});
    let mut doc = c.workspace["document"].clone();
    doc["content"].as_array_mut().unwrap().push(linked.clone());
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let data = c.dispatch(json!({"op":"documentPageSetup"})).unwrap();
    c.dispatch(json!({"op":"documentPageSetupApply","workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"],"section":1,"sectionKey":data["sections"][1]["key"],"scope":"document","patch":{"top":1080}})).unwrap();
    assert_eq!(c.workspace["document"]["content"][3], linked);
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"Still live after page setup"}]})).unwrap();
    let out = std::env::temp_dir().join("folio-page-live.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let exported = Package::read(out.to_str().unwrap()).unwrap();
    assert!(exported
        .text("word/document.xml")
        .unwrap()
        .contains("Still live after page setup"));
}

#[test]
fn page_setup_protection_and_pending_whole_document_are_atomic() {
    let mut p = fixture();
    let types = p.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/></Types>");
    p.set("[Content_Types].xml", types);
    p.set("word/settings.xml",format!("<w:settings xmlns:w=\"{W}\"><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/></w:settings>"));
    let bytes = p.bytes().unwrap();
    assert!(pages::apply(&p, &request(&p, 0, json!({"left":720})), None)
        .err()
        .unwrap()
        .contains("protection"));
    assert_eq!(p.bytes().unwrap(), bytes);
    let p = fixture();
    let tracked = pages::apply(
        &p,
        &request(&p, 1, json!({"left":720})),
        Some(&json!({"version":1,"enabled":true,"formatting":true,"author":"Alex"})),
    )
    .unwrap();
    let mut all = request(&tracked, 0, json!({"top":1080}));
    all["scope"] = json!("document");
    let bytes = tracked.bytes().unwrap();
    assert!(pages::apply(&tracked, &all, None).is_err());
    assert_eq!(tracked.bytes().unwrap(), bytes);
}

#[test]
fn page_review_describes_only_changed_geometry_in_readable_units() {
    let p = fixture();
    let settings = json!({"version":1,"enabled":true,"formatting":true,"author":"Morgan"});
    let gutter = pages::apply(&p, &request(&p, 0, json!({"gutter":360})), Some(&settings)).unwrap();
    let bytes = gutter.bytes().unwrap();
    let rows = rev::list(&gutter).unwrap();
    assert_eq!(
        rows["revisions"][0]["formatChanges"],
        json!([{"property":"Gutter","before":"0 in","after":"0.25 in"}])
    );
    assert_eq!(gutter.bytes().unwrap(), bytes);
    let changed=pages::apply(&p,&request(&p,0,json!({"width":15840,"height":12240,"orientation":"landscape","top":-360,"header":360,"footer":1080,"start":"oddPage"})),Some(&settings)).unwrap();
    let rows = rev::list(&changed).unwrap();
    let fields = rows["revisions"][0]["formatChanges"].as_array().unwrap();
    for (property, before, after) in [
        ("Page width", "8.5 in", "11 in"),
        ("Page height", "11 in", "8.5 in"),
        ("Top margin", "1 in", "-0.25 in"),
        ("Header from edge", "0.5 in", "0.25 in"),
        ("Footer from edge", "0.5 in", "0.75 in"),
        ("Section start", "Inherited / default", "Odd page"),
        ("Orientation", "Inherited / default", "Landscape"),
        (
            "Printer paper setting",
            "Paper code 1",
            "Inherited / default",
        ),
    ] {
        let row = fields.iter().find(|f| f["property"] == property).unwrap();
        assert_eq!(row["before"], before, "{property}");
        assert_eq!(row["after"], after, "{property}");
    }
    assert_eq!(fields.len(), 8);
    let mut unit = fixture();
    let x = unit
        .text("word/document.xml")
        .unwrap()
        .replace("q:left=\"1440\"", "q:left=\"2.54cm\"");
    unit.set("word/document.xml", x);
    let changed = pages::apply(
        &unit,
        &request(&unit, 0, json!({"left":1})),
        Some(&settings),
    )
    .unwrap();
    let rows = rev::list(&changed).unwrap();
    assert_eq!(
        rows["revisions"][0]["formatChanges"],
        json!([{"property":"Left margin","before":"1 in","after":"0.000694 in"}])
    );
}

#[test]
fn page_review_keeps_unrecognized_values_distinguishable() {
    let mut p = docx::new_package();
    p.set("word/document.xml", format!(r#"<w:document xmlns:w="{W}"><w:body><w:p/><w:sectPr>
      <w:type w:val="futureStartB"/><w:pgSz w:orient="futureOrientationB"/><w:pgMar w:left="badB"/>
      <w:sectPrChange w:id="1" w:author="Morgan"><w:sectPr>
        <w:type w:val="futureStartA"/><w:pgSz w:orient="futureOrientationA"/><w:pgMar w:left="badA"/>
      </w:sectPr></w:sectPrChange>
    </w:sectPr></w:body></w:document>"#));
    let bytes = p.bytes().unwrap();
    let rows = rev::list(&p).unwrap();
    assert_eq!(
        rows["revisions"][0]["formatChanges"],
        json!([
          {"property":"Left margin","before":"Unrecognized measurement: badA","after":"Unrecognized measurement: badB"},
          {"property":"Orientation","before":"Other orientation: futureOrientationA","after":"Other orientation: futureOrientationB"},
          {"property":"Section start","before":"Other section start: futureStartA","after":"Other section start: futureStartB"}
        ])
    );
    assert_eq!(p.bytes().unwrap(), bytes);
}
