use crate::{document_bookmarks as bm, docx, office::*, workspace::Core};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn doc(text: &str) -> Value {
    json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})
}
fn add(d: &Value, name: &str, start: usize, end: usize) -> Value {
    bm::command(
        d,
        &json!({"action":"set","name":name,"start":start,"end":end}),
    )
    .unwrap()
}
fn range(d: &Value, name: &str) -> (usize, usize) {
    let b = bm::entries(d).iter().find(|b| b["name"] == name).unwrap();
    (
        b["start"].as_u64().unwrap() as usize,
        b["end"].as_u64().unwrap() as usize,
    )
}

#[test]
fn table_row_sort_moves_ranges_with_rows_and_rebases_local_edits() {
    let rows: Vec<_> = ["A😀 target", "B target", "C target"]
        .iter()
        .enumerate()
        .map(|(i, text)| {
            json!({"type":"tableRow","attrs":{"officeId":format!("row-{i}")},"content":[
                {"type":"tableCell","content":doc(text)["content"]}
            ]})
        })
        .collect();
    let mut old = json!({"type":"doc","content":[doc("Before")["content"][0],
        {"type":"table","attrs":{"officeId":"table"},"content":rows},doc("After")["content"][0]]});
    let index = bm::Index::new(&old);
    let origin = index
        .paragraphs
        .iter()
        .find(|p| p.path == [1, 0, 0, 0])
        .unwrap()
        .start;
    let outside = index.paragraphs.last().unwrap().start;
    old = add(&old, "RowTarget", origin + 4, origin + 10);
    old = add(&old, "RowPoint", origin, origin);
    old = add(&old, "Outside", outside, outside + 5);
    old = crate::document_comments::command(&old, &json!({"action":"add","author":"Reviewer","text":"Keep this row", "start":origin+4,"end":origin+10})).unwrap();
    let mut next = old.clone();
    next["content"][1]["content"] = json!([rows[2], rows[0], rows[1]]);
    next["content"][1]["attrs"]["alignment"] = json!("right");
    next["content"][1]["content"][1]["content"][0]["content"][0]["content"][0]["text"] =
        json!("New A😀 target");
    bm::rebase(&old, &mut next, None).unwrap();
    crate::document_comments::rebase(&old, &mut next, None).unwrap();
    let index = bm::Index::new(&next);
    let destination = index
        .paragraphs
        .iter()
        .find(|p| p.path == [1, 1, 0, 0])
        .unwrap()
        .start;
    assert_eq!(
        range(&next, "RowTarget"),
        (destination + 8, destination + 14)
    );
    assert_eq!(range(&next, "RowPoint"), (destination, destination));
    assert_eq!(range(&next, "Outside"), (outside + 4, outside + 9));
    let comment = &crate::document_comments::entries(&next)[0];
    assert_eq!(
        (
            comment["start"].as_u64().unwrap(),
            comment["end"].as_u64().unwrap()
        ),
        ((destination + 8) as u64, (destination + 14) as u64)
    );
    assert_eq!(
        comment["messages"],
        crate::document_comments::entries(&old)[0]["messages"]
    );
}

#[test]
fn table_row_sort_without_source_ids_retains_ranges_and_rejects_reversed_spans() {
    let rows: Vec<_> = ["Zulu", "Alpha"].iter().map(|text|
        json!({"type":"tableRow","content":[{"type":"tableCell","content":doc(text)["content"]}]})
    ).collect();
    let old = json!({"type":"doc","content":[{"type":"table","content":rows}]});
    let marked = add(&old, "Row", 0, 4);
    let mut next = old.clone();
    next["content"][0]["content"] = json!([rows[1], rows[0]]);
    bm::rebase(&marked, &mut next, None).unwrap();
    assert_eq!(range(&next, "Row"), (6, 10));
    let spanning = add(&old, "Across", 0, 10);
    let mut rejected = next.clone();
    assert!(bm::rebase(&spanning, &mut rejected, None)
        .unwrap_err()
        .contains("reverse"));
    assert_eq!(range(&spanning, "Across"), (0, 10));
}
fn package(body: &str) -> Package {
    let mut p = Package::from_bytes(docx::export(&doc(""), None, None).unwrap()).unwrap();
    p.set(
        "word/document.xml",
        format!(
            "{XML_HEAD}<w:document xmlns:w=\"{W}\"><w:body>{body}<w:sectPr/></w:body></w:document>"
        ),
    );
    p.set("word/header1.xml",format!("<w:hdr xmlns:w=\"{W}\"><w:p><w:bookmarkStart w:id=\"90\" w:name=\"Header\"/><w:r><w:t>Header</w:t></w:r><w:bookmarkEnd w:id=\"90\"/></w:p></w:hdr>"));
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}
fn fixture() -> Package {
    package("<w:p w:rsidR=\"12345678\"><w:pPr><w:keepNext/></w:pPr><w:r><w:rPr><w:b/><w:smallCaps/></w:rPr><w:t>A😀 </w:t></w:r><w:bookmarkStart w:id=\"7\" w:name=\"Revenue\"/><w:hyperlink w:anchor=\"Revenue\"><w:r><w:rPr><w:i/><w:vanish/></w:rPr><w:t>sales</w:t><w:tab/><w:t>today</w:t></w:r></w:hyperlink><w:bookmarkEnd w:id=\"7\"/><w:bookmarkStart w:id=\"8\" w:name=\"Across\"/><w:r><w:t> end</w:t></w:r></w:p><w:p><w:r><w:t>Next</w:t><w:br/><w:t>line</w:t></w:r><w:bookmarkEnd w:id=\"8\"/><w:bookmarkStart w:id=\"9\" w:name=\"_Hidden\"/><w:bookmarkEnd w:id=\"9\"/></w:p><w:p><w:r><w:t>Unrelated</w:t></w:r></w:p>")
}
#[test]
fn bookmark_validation_names_unicode_points_and_duplicate_relocation() {
    let d = add(&doc("😀 alpha"), "Test", 3, 8);
    assert_eq!(range(&d, "Test"), (3, 8));
    let d = add(&d, "test", 2, 2);
    assert_eq!(bm::entries(&d).len(), 1);
    assert_eq!(range(&d, "test"), (2, 2));
    for (name, start, end) in [
        ("a b", 0, 1),
        ("1name", 0, 1),
        ("Name", 1, 2),
        ("Name", 3, 2),
        ("Name", 0, 99),
        ("_Hidden", 0, 1),
    ] {
        assert!(bm::command(
            &d,
            &json!({"action":"set","name":name,"start":start,"end":end})
        )
        .is_err());
    }
    let d = bm::command(&d, &json!({"action":"delete","name":"TEST"})).unwrap();
    assert!(bm::entries(&d).is_empty());
    assert_eq!(bm::Index::new(&d).text, "😀 alpha\n");
}
#[test]
fn bookmark_exact_edit_affinities_repeated_text_and_partial_deletion() {
    for (start, end, text, expected) in [
        (0, 0, "Z", Some((3, 5))),
        (2, 2, "Z", Some((2, 5))),
        (3, 3, "Z", Some((2, 5))),
        (4, 4, "Z", Some((2, 4))),
        (2, 3, "", Some((2, 3))),
        (3, 4, "", Some((2, 3))),
        (0, 3, "", Some((0, 1))),
        (3, 6, "", Some((2, 3))),
        (2, 4, "Z", None),
        (0, 6, "", None),
    ] {
        let old = add(&doc("aaaaaa"), "Mark", 2, 4);
        let mut raw = "aaaaaa".to_string();
        raw.replace_range(start..end, text);
        let mut next = doc(&raw);
        bm::rebase(
            &old,
            &mut next,
            Some(&json!([{"start":start,"end":end,"text":text}])),
        )
        .unwrap();
        if let Some(expected) = expected {
            assert_eq!(range(&next, "Mark"), expected, "{start}:{end} {text}");
        } else {
            assert!(bm::entries(&next).is_empty());
        }
    }
    let old = add(&doc("aaaa"), "Point", 2, 2);
    let mut next = doc("aaZaa");
    bm::rebase(
        &old,
        &mut next,
        Some(&json!([{"start":2,"end":2,"text":"Z"}])),
    )
    .unwrap();
    assert_eq!(range(&next, "Point"), (2, 2));
    let bad = bm::rebase(
        &old,
        &mut next,
        Some(&json!([{"start":0,"end":1,"text":"wrong"}])),
    );
    assert!(bad.is_err());
}
#[test]
fn bookmark_diff_handles_multiple_replacements_formatting_and_paragraph_splits() {
    let old = add(&doc("before TARGET after"), "Target", 7, 13);
    let mut next = doc("revised before TARGET after revision");
    bm::rebase(&old, &mut next, None).unwrap();
    assert_eq!(range(&next, "Target"), (15, 21));
    let old = add(&doc("hello world"), "Words", 0, 11);
    let mut next = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"hello"}]},{"type":"paragraph","content":[{"type":"text","text":"world","marks":[{"type":"bold"}]}]}]});
    bm::rebase(&old, &mut next, None).unwrap();
    assert_eq!(range(&next, "Words"), (0, 11));
    let mut same = next.clone();
    same["content"][0]["attrs"]["textAlign"] = json!("center");
    bm::rebase(&next, &mut same, None).unwrap();
    assert_eq!(bm::entries(&same), bm::entries(&next));
}
#[test]
fn bookmark_core_edits_history_proposals_and_stale_commands_are_atomic() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"document","document":doc("First target last")}))
        .unwrap();
    c.dispatch(json!({"op":"bookmark","action":"set","name":"Target","start":6,"end":12,"baseRevision":c.workspace["revision"]})).unwrap();
    let saved = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"bookmark","action":"delete","name":"Target","baseRevision":0}))
        .is_err());
    assert!(c.dispatch(json!({"op":"document","document":doc("wrong"),"textEdits":[{"start":0,"end":0,"text":"bad"}]})).is_err());
    assert_eq!(c.workspace, saved);
    c.dispatch(json!({"op":"document","document":doc("A First target last"),"textEdits":[{"start":0,"end":0,"text":"A "}]})).unwrap();
    assert_eq!(range(&c.workspace["document"], "Target"), (8, 14));
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], saved["document"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(range(&c.workspace["document"], "Target"), (8, 14));
    let proposal = c.dispatch(json!({"op":"propose","proposal":{"summary":"Add context","baseRevision":c.workspace["revision"],"appendParagraphs":["More context"]}})).unwrap();
    c.dispatch(json!({"op":"apply","proposal":proposal}))
        .unwrap();
    assert_eq!(range(&c.workspace["document"], "Target"), (8, 14));
    let proposal = c.dispatch(json!({"op":"propose","proposal":{"summary":"Replace prose","baseRevision":c.workspace["revision"],"replaceDocument":["New document"]}})).unwrap();
    c.dispatch(json!({"op":"apply","proposal":proposal}))
        .unwrap();
    assert!(bm::entries(&c.workspace["document"]).is_empty());
}
#[test]
fn docx_bookmark_import_cross_paragraph_hidden_hyperlink_and_exact_untouched_export() {
    let p = fixture();
    let (d, _) = docx::import(&p).unwrap();
    assert_eq!(range(&d, "Revenue"), (4, 15));
    assert_eq!(range(&d, "Across"), (15, 29));
    assert_eq!(range(&d, "_Hidden"), (29, 29));
    assert!(d.to_string().contains("#Revenue"));
    assert_eq!(docx::export(&d, Some(&p), Some(&d)).unwrap(), p.original);
}
#[test]
fn docx_bookmarks_unrelated_edits_keep_unmodeled_run_properties_and_other_parts() {
    let p = fixture();
    let original = docx::import(&p).unwrap().0;
    let mut edited = original.clone();
    edited["content"][2]["content"][0]["text"] = json!("An unrelated edit");
    bm::rebase(&original, &mut edited, None).unwrap();
    let output =
        Package::from_bytes(docx::export(&edited, Some(&p), Some(&original)).unwrap()).unwrap();
    let round = docx::import(&output).unwrap().0;
    for name in ["Revenue", "Across", "_Hidden"] {
        assert_eq!(range(&round, name), range(&edited, name));
    }
    for (path, bytes) in &p.entries {
        if path != "word/document.xml" {
            assert_eq!(&output.entries[path], bytes, "{path}");
        }
    }
    let xml = output.text("word/document.xml").unwrap();
    let dom = parse(xml).unwrap();
    for property in ["smallCaps", "vanish", "keepNext"] {
        assert!(
            dom.descendants().any(|n| n.has_tag_name((W, property))),
            "Missing {property}: {xml}"
        );
    }
    let link = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")))
        .unwrap();
    assert_eq!(link.attribute((W, "anchor")), Some("Revenue"));
    assert_eq!(
        dom.descendants()
            .filter(|n| n.has_tag_name((W, "bookmarkStart")))
            .count(),
        3
    );
}
#[test]
fn docx_new_overlapping_ranges_empty_paragraph_emoji_tabs_breaks_and_deleted_ranges() {
    let mut d = doc("A😀\t&Z");
    d["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"paragraph"}));
    d = add(&d, "Inside", 1, 3);
    d = add(&d, "Overlap", 0, 5);
    d = add(&d, "Empty", 7, 7);
    let out = Package::from_bytes(docx::export(&d, None, None).unwrap()).unwrap();
    let round = docx::import(&out).unwrap().0;
    for name in ["Inside", "Overlap", "Empty"] {
        assert_eq!(range(&round, name), range(&d, name));
    }
    let deleted = bm::command(&round, &json!({"action":"delete","name":"Inside"})).unwrap();
    let next =
        Package::from_bytes(docx::export(&deleted, Some(&out), Some(&round)).unwrap()).unwrap();
    assert!(!docx::import(&next).unwrap().0["attrs"]["bookmarks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["name"] == "Inside"));
}
#[test]
fn docx_bookmarks_in_table_cells_and_across_table_boundary() {
    let p = package("<w:p><w:bookmarkStart w:id=\"1\" w:name=\"AcrossTable\"/><w:r><w:t>Intro</w:t></w:r></w:p><w:tbl><w:tblGrid><w:gridCol w:w=\"2400\"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>Cell </w:t></w:r><w:bookmarkStart w:id=\"2\" w:name=\"CellWord\"/><w:r><w:t>value</w:t></w:r><w:bookmarkEnd w:id=\"2\"/></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>End</w:t></w:r><w:bookmarkEnd w:id=\"1\"/></w:p>");
    let original = docx::import(&p).unwrap().0;
    assert_eq!(range(&original, "CellWord"), (11, 16));
    let mut edited = original.clone();
    edited["content"][0]["content"][0]["text"] = json!("Long Intro");
    bm::rebase(&original, &mut edited, None).unwrap();
    let out =
        Package::from_bytes(docx::export(&edited, Some(&p), Some(&original)).unwrap()).unwrap();
    let round = docx::import(&out).unwrap().0;
    assert_eq!(range(&round, "CellWord"), (16, 21));
    assert_eq!(range(&round, "AcrossTable"), range(&edited, "AcrossTable"));
}
#[test]
fn advanced_or_orphan_bookmarks_remain_preserved_and_export_rejects_silent_loss() {
    let inline_object = package("<w:p><w:r><w:t>Note</w:t></w:r><w:bookmarkStart w:id=\"1\" w:name=\"Footnote\"/><w:r><w:footnoteReference w:id=\"1\"/></w:r><w:bookmarkEnd w:id=\"1\"/></w:p>");
    let (object_doc, object_report) = docx::import(&inline_object).unwrap();
    assert!(bm::entries(&object_doc).is_empty());
    assert!(object_report.to_string().contains("bookmarks"));
    assert_eq!(
        docx::export(&object_doc, Some(&inline_object), Some(&object_doc)).unwrap(),
        inline_object.original
    );
    let p = package("<w:p><w:bookmarkStart w:id=\"1\" w:name=\"Column\" w:colFirst=\"0\" w:colLast=\"1\"/><w:r><w:t>Original</w:t></w:r><w:bookmarkEnd w:id=\"1\"/><w:bookmarkEnd w:id=\"2\"/></w:p><w:p><w:r><w:t>Elsewhere</w:t></w:r></w:p>");
    let (original, report) = docx::import(&p).unwrap();
    assert!(bm::entries(&original).is_empty());
    assert!(report.to_string().contains("bookmarks"));
    let mut edited = original.clone();
    edited["content"][1]["content"][0]["text"] = json!("Changed elsewhere");
    assert!(docx::export(&edited, Some(&p), Some(&original)).is_ok());
    edited["content"][0]["content"][0]["text"] = json!("Changed bookmarked paragraph");
    assert!(docx::export(&edited, Some(&p), Some(&original))
        .unwrap_err()
        .contains("preserved Word bookmark"));
}
#[test]
fn legacy_hydration_does_not_resurrect_deleted_bookmarks() {
    let source = add(&doc("before target"), "Target", 7, 13);
    let mut legacy = doc("A before target");
    bm::hydrate(&mut legacy, &source).unwrap();
    assert_eq!(range(&legacy, "Target"), (9, 15));
    let mut empty = bm::command(&legacy, &json!({"action":"delete","name":"Target"})).unwrap();
    bm::hydrate(&mut empty, &source).unwrap();
    assert!(bm::entries(&empty).is_empty());
}

#[test]
fn bookmark_recovery_and_independent_reader_artifacts() {
    let folder = std::env::temp_dir().join("folio-word-bookmarks-tests");
    std::fs::create_dir_all(&folder).unwrap();
    let source = fixture();
    std::fs::write(folder.join("source.docx"), &source.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":folder.join("source.docx")}))
        .unwrap();
    c.dispatch(json!({"op":"bookmark","action":"set","name":"Start","start":0,"end":0,"baseRevision":c.workspace["revision"]})).unwrap();
    let mut next = c.workspace["document"].clone();
    next["content"][0]["content"][0]["text"] = json!("Prefix A😀 ");
    c.dispatch(
        json!({"op":"document","document":next,"textEdits":[{"start":0,"end":0,"text":"Prefix "}]}),
    )
    .unwrap();
    c.dispatch(json!({"op":"bookmark","action":"delete","name":"Across","baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(range(&c.workspace["document"], "Revenue"), (11, 22));
    assert_eq!(range(&c.workspace["document"], "_Hidden"), (36, 36));
    assert_eq!(range(&c.workspace["document"], "Start"), (0, 0));
    let state = c.workspace.clone();
    c.dispatch(json!({"op":"save","path":folder.join("bookmarks.folio")}))
        .unwrap();
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":folder.join("edited.docx")}))
        .unwrap();
    let mut reopened = Core::new();
    reopened
        .dispatch(json!({"op":"load","path":folder.join("bookmarks.folio")}))
        .unwrap();
    assert_eq!(reopened.workspace, state);
    reopened
        .dispatch(json!({"op":"exportOffice","format":"docx","path":folder.join("reopened.docx")}))
        .unwrap();
    let a = Package::read(folder.join("edited.docx").to_str().unwrap()).unwrap();
    let b = Package::read(folder.join("reopened.docx").to_str().unwrap()).unwrap();
    assert_eq!(a.entries, b.entries);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(range(&c.workspace["document"], "Across"), (22, 36));
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], state["document"]);
}

#[test]
fn bookmark_export_projection_mismatch_is_an_error_not_a_panic() {
    let mut d = doc("abc");
    d["content"][0]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"customInline","content":[{"type":"text","text":"extra"}]}));
    let marked = add(&d, "Custom", 1, 7);
    let result = docx::export(&marked, None, None);
    assert!(result.unwrap_err().contains("cannot be represented"));
    let mut duplicate = add(&add(&doc("abc"), "One", 0, 1), "Two", 1, 2);
    duplicate["attrs"]["bookmarks"][0]["sourceId"] = json!("1");
    duplicate["attrs"]["bookmarks"][1]["sourceId"] = json!("1");
    assert!(bm::validate(&duplicate).is_err());
}

#[test]
fn exact_proposal_edits_disambiguate_repeated_text_and_reject_wrong_workspaces() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"document","document":doc("aaaaaa")}))
        .unwrap();
    c.dispatch(json!({"op":"bookmark","action":"set","name":"Middle","start":2,"end":4,"baseRevision":c.workspace["revision"]})).unwrap();
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"bookmark","action":"delete","name":"Middle","baseRevision":c.workspace["revision"],"workspaceId":"different"})).is_err());
    assert_eq!(c.workspace, before);
    let p = c.dispatch(json!({"op":"propose","proposal":{"summary":"Exact insertion","baseRevision":c.workspace["revision"],"document":doc("aaaaaaa"),"textEdits":[{"start":0,"end":0,"text":"a"}]}})).unwrap();
    c.dispatch(json!({"op":"apply","proposal":p})).unwrap();
    assert_eq!(range(&c.workspace["document"], "Middle"), (3, 5));
    let saved = c.workspace["document"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":c.workspace["sheets"][0]["id"],"address":"A25","value":"42"}]})).unwrap();
    assert_eq!(c.workspace["document"], saved);
}

#[test]
fn hidden_names_are_set_only_when_asked_for() {
    let d = doc("Intro\nBody");
    let plain = json!({"action":"set","name":"_Intro","start":0,"end":5});
    assert!(bm::command(&d, &plain).is_err());
    let hidden = json!({"action":"set","name":"_Intro","start":0,"end":5,"hidden":true});
    let next = bm::command(&d, &hidden).unwrap();
    assert_eq!(range(&next, "_Intro"), (0, 5));
}

#[test]
fn a_link_to_a_hidden_heading_bookmark_round_trips() {
    let dir = std::env::temp_dir().join(format!("folio-heading-link-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.docx");
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"document","document":{"type":"doc","content":[
        {"type":"heading","attrs":{"level":1},"content":[{"type":"text","text":"Intro"}]},
        {"type":"paragraph","content":[{"type":"text","text":"Back to top"}]}]}}))
        .unwrap();
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":source}))
        .unwrap();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    c.dispatch(json!({"op":"bookmark","action":"set","name":"_Intro","hidden":true,"start":0,"end":5,"workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"]})).unwrap();
    let mut d = c.workspace["document"].clone();
    d["content"][1]["content"][0]["marks"] = json!([{"type":"link","attrs":{"href":"#_Intro"}}]);
    c.dispatch(json!({"op":"document","document":d,"baseRevision":c.workspace["revision"]}))
        .unwrap();
    let out = dir.join("linked.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let mut reopened = Core::new();
    let imported = reopened
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    let doc = &imported["workspace"]["document"];
    assert_eq!(range(doc, "_Intro"), (0, 5));
    assert_eq!(
        doc["content"][1]["content"][0]["marks"][0]["attrs"]["href"],
        "#_Intro"
    );
}
