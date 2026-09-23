use crate::{docx, docx_page_setup as pages, docx_sections as sections, office::*};
use serde_json::json;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn fixture() -> Package {
    let mut p = docx::new_package();
    p.set("word/document.xml",format!(r#"<q:document xmlns:q="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><q:body><q:p><q:pPr><q:jc q:val="center"/></q:pPr><q:bookmarkStart q:id="7" q:name="All"/><q:r><q:rPr><q:b/></q:rPr><q:t>Alpha 😀</q:t></q:r><q:hyperlink r:id="link"><q:r><q:t> beta</q:t></q:r></q:hyperlink><q:bookmarkEnd q:id="7"/></q:p><q:p><q:r><q:t>Second paragraph.</q:t></q:r></q:p><q:sectPr><q:headerReference q:type="default" r:id="head"/><q:footerReference q:type="default" r:id="foot"/><q:pgSz q:w="12240" q:h="15840"/><q:pgMar q:top="1440" q:bottom="1440" q:left="1440" q:right="1440" q:header="720" q:footer="720" q:gutter="0"/></q:sectPr></q:body></q:document>"#));
    p.set(
        "word/header1.xml",
        format!("<w:hdr xmlns:w=\"{W}\"><w:p><w:r><w:t>Header retained</w:t></w:r></w:p></w:hdr>"),
    );
    p.set(
        "word/footer1.xml",
        format!("<w:ftr xmlns:w=\"{W}\"><w:p><w:r><w:t>Footer retained</w:t></w:r></w:p></w:ftr>"),
    );
    let types=p.text("[Content_Types].xml").unwrap().replace("</Types>",r#"<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/></Types>"#);
    p.set("[Content_Types].xml", types);
    let rels=p.text("word/_rels/document.xml.rels").unwrap().replace("</Relationships>",r#"<Relationship Id="head" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="foot" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/><Relationship Id="link" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org/" TargetMode="External"/></Relationships>"#);
    p.set("word/_rels/document.xml.rels", rels);
    p
}
#[test]
fn section_break_insert_split_remove_preserves_rich_content_and_effective_settings() {
    for start in ["nextPage", "continuous", "evenPage", "oddPage"] {
        let p = fixture();
        let (doc, _) = docx::import(&p).unwrap();
        let before = p.bytes().unwrap();
        let mut r = json!({"action":"insert","path":[0],"offset":8,"start":start});
        let context = sections::context(&p, &doc, &r).unwrap();
        assert_eq!(context["canInsert"], true);
        r["targetKey"] = context["targetKey"].clone();
        let changed = sections::apply(&p, &doc, &r, None).unwrap();
        assert_eq!(p.bytes().unwrap(), before);
        let layout = pages::read(&changed).unwrap();
        assert_eq!(layout["sections"].as_array().unwrap().len(), 2);
        assert_eq!(layout["sections"][1]["geometry"]["start"], start);
        let x = changed.text("word/document.xml").unwrap();
        let d = parse(x).unwrap();
        let body = pages::document_body(&d).unwrap();
        let ps = body
            .children()
            .filter(|p| p.has_tag_name((W, "p")))
            .collect::<Vec<_>>();
        assert_eq!(ps.len(), 3);
        assert_eq!(xml_text(ps[0]), "Alpha 😀");
        assert_eq!(xml_text(ps[1]), " beta");
        assert!(ps[0].descendants().any(|n| n.has_tag_name((W, "b"))));
        assert!(ps[1]
            .descendants()
            .any(|n| n.has_tag_name((W, "hyperlink"))));
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "headerReference")))
                .count(),
            1
        );
        let (new_doc, _) = docx::import(&changed).unwrap();
        let removed = sections::apply(
            &changed,
            &new_doc,
            &json!({"action":"remove","section":0,"sectionKey":layout["sections"][0]["key"]}),
            None,
        )
        .unwrap();
        let layout = pages::read(&removed).unwrap();
        assert_eq!(layout["sections"].as_array().unwrap().len(), 1);
        let (joined, _) = docx::import(&removed).unwrap();
        assert_eq!(
            crate::workspace::document_text(&joined),
            crate::workspace::document_text(&doc)
        );
        let d = parse(removed.text("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "bookmarkStart")))
                .count(),
            1
        );
        assert!(d
            .descendants()
            .any(|n| n.has_tag_name((W, "headerReference"))));
        std::fs::write(
            std::env::temp_dir().join(format!("folio-section-{start}.docx")),
            changed.bytes().unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn section_break_rejects_stale_unicode_invalid_and_recorded_edits_without_mutation() {
    let p = fixture();
    let (doc, _) = docx::import(&p).unwrap();
    let before = p.bytes().unwrap();
    let r = json!({"action":"insert","path":[0],"offset":7,"start":"continuous"});
    assert_eq!(sections::context(&p, &doc, &r).unwrap()["canInsert"], false);
    assert!(sections::apply(&p, &doc, &r, None).is_err());
    let mut r = json!({"action":"insert","path":[0],"offset":8,"start":"continuous"});
    r["targetKey"] = sections::context(&p, &doc, &r).unwrap()["targetKey"].clone();
    assert!(sections::apply(
        &p,
        &doc,
        &r,
        Some(&json!({"enabled":true,"author":"Morgan"}))
    )
    .err()
    .unwrap()
    .contains("Recording section"));
    r["targetKey"] = json!("stale");
    assert!(sections::apply(&p, &doc, &r, None).is_err());
    assert_eq!(p.bytes().unwrap(), before);
}

#[test]
fn section_break_workspace_history_pending_text_live_bindings_and_recovery() {
    use crate::workspace::Core;
    let source = std::env::temp_dir().join("folio-section-source.docx");
    std::fs::write(&source, fixture().bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    let sheet = c.workspace["sheets"][0]["id"].clone();
    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Pending Alpha 😀");
    doc["content"].as_array_mut().unwrap().insert(0,json!({"type":"workbookBlock","attrs":{"sheetId":sheet,"range":"A1:B2","title":"Live numbers"}}));
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    for (name, start, end) in [("AfterLive", 2, 23), ("SplitPoint", 18, 18)] {
        c.dispatch(json!({"op":"bookmark","action":"set","name":name,"start":start,"end":end,"baseRevision":c.workspace["revision"]})).unwrap();
    }
    c.dispatch(json!({"op":"documentComment","action":"add","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"author":"Reviewer","text":"Check beta","start":18,"end":23})).unwrap();
    let before = c.workspace.clone();
    let data = c
        .dispatch(json!({"op":"documentSectionContext","path":[1],"offset":16}))
        .unwrap();
    assert_eq!(c.workspace, before);
    assert_eq!(data["canInsert"], true);
    c.dispatch(json!({"op":"documentSectionBreak","action":"insert","path":[1],"offset":16,"start":"continuous","targetKey":data["targetKey"],"workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"]})).unwrap();
    assert_eq!(
        c.workspace["document"]["content"][0],
        before["document"]["content"][0]
    );
    assert_eq!(
        c.workspace["document"]["attrs"][pages::KEY]["sections"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        crate::workspace::document_text(&c.workspace["document"]["content"][1]),
        "Pending Alpha 😀"
    );
    let inserted = c.workspace.clone();
    let bookmarks = crate::document_bookmarks::entries(&inserted["document"]);
    assert_eq!(
        bookmarks.iter().find(|b| b["name"] == "AfterLive").unwrap()["end"],
        24
    );
    let point = bookmarks
        .iter()
        .find(|b| b["name"] == "SplitPoint")
        .unwrap();
    assert_eq!(
        (point["start"].clone(), point["end"].clone()),
        (json!(18), json!(18))
    );
    let comment = &crate::document_comments::entries(&inserted["document"])[0];
    assert_eq!(
        (comment["start"].clone(), comment["end"].clone()),
        (json!(18), json!(24))
    );
    let undo = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before["document"]);
    assert_eq!(
        undo["documentHistoryChange"]["textEdits"],
        json!([{"start":18,"end":19,"text":""}])
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], inserted["document"]);
    let saved = std::env::temp_dir().join("folio-section-recovery.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(c.workspace["document"], inserted["document"]);
    let data = c.dispatch(json!({"op":"documentPageSetup"})).unwrap();
    c.dispatch(json!({"op":"documentSectionBreak","action":"remove","section":0,"sectionKey":data["sections"][0]["key"],"workspaceId":data["workspaceId"],"baseRevision":data["baseRevision"]})).unwrap();
    assert_eq!(
        crate::workspace::document_text(&c.workspace["document"]["content"][1]),
        "Pending Alpha 😀 beta"
    );
    assert_eq!(
        c.workspace["document"]["content"][0],
        before["document"]["content"][0]
    );
    let bookmarks = crate::document_bookmarks::entries(&c.workspace["document"]);
    assert_eq!(
        bookmarks.iter().find(|b| b["name"] == "AfterLive").unwrap()["end"],
        23
    );
    assert_eq!(
        crate::document_comments::entries(&c.workspace["document"])[0]["end"],
        23
    );
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"Live after removing a section"}]})).unwrap();
    let out = std::env::temp_dir().join("folio-section-core.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let exported = Package::read(out.to_str().unwrap()).unwrap();
    assert!(exported
        .text("word/document.xml")
        .unwrap()
        .contains("Live after removing a section"));
}

#[test]
fn section_break_start_end_blank_and_existing_boundary_keep_counts_and_text() {
    for offset in [0, 13] {
        let p = fixture();
        let (doc, _) = docx::import(&p).unwrap();
        let mut r = json!({"action":"insert","path":[0],"offset":offset,"start":"oddPage"});
        r["targetKey"] = sections::context(&p, &doc, &r).unwrap()["targetKey"].clone();
        let first = sections::apply(&p, &doc, &r, None).unwrap();
        let (doc, _) = docx::import(&first).unwrap();
        let mut r = json!({"action":"insert","path":[0],"offset":0,"start":"evenPage"});
        r["targetKey"] = sections::context(&first, &doc, &r).unwrap()["targetKey"].clone();
        let second = sections::apply(&first, &doc, &r, None).unwrap();
        let layout = pages::read(&second).unwrap();
        assert_eq!(layout["sections"].as_array().unwrap().len(), 3);
        assert_eq!(layout["sections"][1]["geometry"]["start"], "evenPage");
        assert_eq!(layout["sections"][2]["geometry"]["start"], "oddPage");
        let d = parse(second.text("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "bookmarkStart")))
                .count(),
            1
        );
    }
}

#[test]
fn section_break_removal_adopts_following_geometry_and_retains_inherited_headers() {
    let p = fixture();
    let (doc, _) = docx::import(&p).unwrap();
    let mut r = json!({"action":"insert","path":[0],"offset":8,"start":"continuous"});
    r["targetKey"] = sections::context(&p, &doc, &r).unwrap()["targetKey"].clone();
    let p = sections::apply(&p, &doc, &r, None).unwrap();
    let catalog = pages::read(&p).unwrap();
    let p=pages::apply(&p,&json!({"section":1,"sectionKey":catalog["sections"][1]["key"],"scope":"section","patch":{"left":720,"width":15840,"height":12240,"orientation":"landscape"}}),None).unwrap();
    let (doc, _) = docx::import(&p).unwrap();
    let catalog = pages::read(&p).unwrap();
    let merged = sections::apply(
        &p,
        &doc,
        &json!({"action":"remove","section":0,"sectionKey":catalog["sections"][0]["key"]}),
        None,
    )
    .unwrap();
    let layout = pages::read(&merged).unwrap();
    assert_eq!(layout["sections"][0]["geometry"]["left"], 720);
    assert_eq!(
        layout["sections"][0]["geometry"]["orientation"],
        "landscape"
    );
    let x = merged.text("word/document.xml").unwrap();
    assert!(x.find("headerReference").unwrap() < x.find("footerReference").unwrap());
    std::fs::write(
        std::env::temp_dir().join("folio-section-merged.docx"),
        merged.bytes().unwrap(),
    )
    .unwrap();
}

#[test]
fn section_break_preserves_tabs_breaks_run_attributes_and_empty_paragraphs() {
    let mut p = fixture();
    let x = p.text("word/document.xml").unwrap().replace(
        "<q:t>Alpha 😀</q:t>",
        "<q:t xml:space=\"preserve\">A </q:t><q:tab/><q:t>B</q:t><q:br/><q:t>😀</q:t>",
    );
    p.set("word/document.xml", x);
    let (doc, _) = docx::import(&p).unwrap();
    for offset in [0, 2, 3, 4, 5, 7, 12] {
        let mut r = json!({"action":"insert","path":[0],"offset":offset,"start":"nextPage"});
        r["targetKey"] = sections::context(&p, &doc, &r).unwrap()["targetKey"].clone();
        let changed = sections::apply(&p, &doc, &r, None).unwrap();
        let (split, _) = docx::import(&changed).unwrap();
        let mut expected = crate::document_bookmarks::Index::new(&doc)
            .text
            .encode_utf16()
            .collect::<Vec<_>>();
        expected.insert(offset, 10);
        assert_eq!(
            crate::document_bookmarks::Index::new(&split).text,
            String::from_utf16(&expected).unwrap()
        );
        let x = changed.text("word/document.xml").unwrap();
        let dom = parse(x).unwrap();
        assert_eq!(
            dom.descendants()
                .filter(|n| n.has_tag_name((W, "tab")))
                .count(),
            1
        );
        assert_eq!(
            dom.descendants()
                .filter(|n| n.has_tag_name((W, "br")))
                .count(),
            1
        );
        for part in [
            "word/header1.xml",
            "word/footer1.xml",
            "word/_rels/document.xml.rels",
        ] {
            assert_eq!(changed.entries[part], p.entries[part]);
        }
    }
    let p = docx::new_package();
    let doc = docx::import(&p).unwrap().0;
    let p = docx::export_review(&doc, Some(&p), Some(&doc)).unwrap();
    let context = sections::context(&p, &doc, &json!({"path":[0],"offset":0})).unwrap();
    let changed = sections::apply(&p, &doc, &json!({"action":"insert","path":[0],"offset":0,"start":"continuous","targetKey":context["targetKey"]}), None).unwrap();
    assert_eq!(
        pages::read(&changed).unwrap()["sections"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn section_break_guards_preserved_objects_protection_and_pending_changes() {
    for element in [
        "<q:drawing/>",
        "<q:fldChar q:fldCharType=\"begin\"/>",
        "<q:ins q:id=\"8\" q:author=\"Reviewer\"><q:r><q:t>added</q:t></q:r></q:ins>",
    ] {
        let mut p = fixture();
        let x = p.text("word/document.xml").unwrap().replacen(
            "<q:bookmarkEnd",
            &(element.to_owned() + "<q:bookmarkEnd"),
            1,
        );
        p.set("word/document.xml", x);
        let doc = docx::import(&p).unwrap().0;
        assert_eq!(
            sections::context(&p, &doc, &json!({"path":[0],"offset":0})).unwrap()["canInsert"],
            false
        );
    }
    let mut p = fixture();
    let doc = docx::import(&p).unwrap().0;
    let context = sections::context(&p, &doc, &json!({"path":[0],"offset":0})).unwrap();
    let r = json!({"action":"insert","path":[0],"offset":0,"start":"continuous","targetKey":context["targetKey"]});
    p.set("word/settings.xml", format!("<w:settings xmlns:w=\"{W}\"><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/></w:settings>"));
    let types = p.text("[Content_Types].xml").unwrap().replace("</Types>", "<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/></Types>");
    p.set("[Content_Types].xml", types);
    let before = p.bytes().unwrap();
    assert_eq!(sections::context(&p, &doc, &r).unwrap()["protected"], true);
    assert!(sections::apply(&p, &doc, &r, None)
        .err()
        .unwrap()
        .contains("protection"));
    assert_eq!(p.bytes().unwrap(), before);
}

#[test]
fn section_break_continuous_inheritance_materializes_when_new_start_requires_it() {
    let mut p = fixture();
    let x = p
        .text("word/document.xml")
        .unwrap()
        .replacen(
            "<q:jc q:val=\"center\"/>",
            "<q:jc q:val=\"center\"/><q:sectPr><q:type q:val=\"continuous\"/></q:sectPr>",
            1,
        )
        .replace(
            "q:w=\"12240\" q:h=\"15840\"",
            "q:w=\"15840\" q:h=\"12240\" q:orient=\"landscape\"",
        );
    p.set("word/document.xml", x);
    let doc = docx::import(&p).unwrap().0;
    let context = sections::context(&p, &doc, &json!({"path":[0],"offset":8})).unwrap();
    assert_eq!(context["sections"][0]["inheritedPageGeometry"], true);
    let changed = sections::apply(&p,&doc,&json!({"action":"insert","path":[0],"offset":8,"start":"nextPage","targetKey":context["targetKey"]}),None).unwrap();
    let data = pages::read(&changed).unwrap();
    assert_eq!(data["sections"].as_array().unwrap().len(), 3);
    for section in data["sections"].as_array().unwrap() {
        assert_eq!(section["geometry"]["width"], 15840);
        assert_eq!(section["geometry"]["orientation"], "landscape");
    }
    assert_eq!(data["sections"][1]["sizeExplicit"], true);
    std::fs::write(
        std::env::temp_dir().join("folio-section-inherited.docx"),
        changed.bytes().unwrap(),
    )
    .unwrap();
}
