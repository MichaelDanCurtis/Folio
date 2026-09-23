use crate::{docx, docx_revisions as rev, office::*, workspace::Core};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn meta(id: u32) -> String {
    format!("q:id=\"{id}\" q:author=\"Alex\" q:date=\"2026-09-20T18:00:00Z\"")
}
fn package(body: &str) -> Package {
    let mut p = docx::new_package();
    p.set(
        "word/document.xml",
        format!("<q:document xmlns:q=\"{W}\"><q:body>{body}<q:sectPr/></q:body></q:document>"),
    );
    p.entries
        .insert("word/media/retained.bin".into(), vec![9, 8, 7, 6]);
    p.original = p.bytes().unwrap();
    p
}
fn side(from: bool, id: u32, text: &str) -> String {
    let tag = if from { "moveFrom" } else { "moveTo" };
    format!("<q:{tag}RangeStart {} q:name=\"budgetMove\"/><q:{tag} {}><q:r><q:rPr><q:b/><q:color q:val=\"185ABD\"/></q:rPr><q:t>{text}</q:t></q:r></q:{tag}><q:{tag}RangeEnd q:id=\"{id}\"/>",meta(id),meta(id+1))
}
fn input() -> Package {
    package(&format!("<q:p><q:r><q:t>Planning notes</q:t></q:r></q:p><q:p><q:r><q:t xml:space=\"preserve\">Before: </q:t></q:r>{}<q:r><q:t xml:space=\"preserve\"> stays.</q:t></q:r></q:p><q:p><q:r><q:t xml:space=\"preserve\">After: </q:t></q:r>{}<q:r><q:t xml:space=\"preserve\"> here.</q:t></q:r></q:p><q:p><q:ins q:id=\"30\" q:author=\"Morgan\" q:date=\"2026-09-20T18:01:00Z\"><q:r><q:t>Unrelated note.</q:t></q:r></q:ins></q:p>",side(true,10,"Budget 😀"),side(false,20,"Budget 😀")))
}
fn resolve(p: &Package, index: usize, action: &str) -> Package {
    let list = rev::list(p).unwrap();
    rev::resolve(
        p,
        &json!({"action":action,"keys":[list["revisions"][index]["key"]]}),
    )
    .unwrap()
}
fn text(n: &Value) -> String {
    n["text"].as_str().map(String::from).unwrap_or_else(|| {
        n["content"]
            .as_array()
            .map(|a| a.iter().map(text).collect())
            .unwrap_or_default()
    })
}
fn paragraphs(p: &Package, mode: &str) -> Vec<String> {
    rev::display(p, mode).unwrap()["document"]["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["type"] == "paragraph")
        .map(text)
        .collect()
}
fn save(p: &Package, name: &str) {
    std::fs::write(
        std::env::temp_dir().join(format!("folio-moves-{name}.docx")),
        p.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn named_moves_review_both_locations_and_keep_unrelated_revisions() {
    let p = input();
    let original = p.bytes().unwrap();
    let list = rev::list(&p).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 3);
    for i in 0..2 {
        let r = &list["revisions"][i];
        assert_eq!(r["canAccept"], true);
        assert_eq!(r["canReject"], true);
        assert_eq!(r["move"]["memberCount"], 2);
        assert_eq!(r["move"]["counterpartKey"], list["revisions"][1 - i]["key"]);
        let accepted = resolve(&p, i, "accept");
        let rejected = resolve(&p, i, "reject");
        for q in [&accepted, &rejected] {
            assert_eq!(
                rev::list(q).unwrap()["revisions"].as_array().unwrap().len(),
                1
            );
            assert_eq!(q.entries["word/media/retained.bin"], vec![9, 8, 7, 6]);
        }
        assert_eq!(
            paragraphs(&accepted, "current"),
            vec![
                "Planning notes",
                "Before:  stays.",
                "After: Budget 😀 here.",
                "Unrelated note."
            ]
        );
        assert_eq!(
            paragraphs(&rejected, "current"),
            vec![
                "Planning notes",
                "Before: Budget 😀 stays.",
                "After:  here.",
                "Unrelated note."
            ]
        );
        for q in [&accepted, &rejected] {
            let x = q.text("word/document.xml").unwrap();
            assert!(!x.contains("moveFrom"));
            assert!(!x.contains("moveTo"));
            assert!(x.contains("185ABD"));
        }
        save(&accepted, "accepted");
        save(&rejected, "rejected");
    }
    assert_eq!(p.bytes().unwrap(), original);
    save(&p, "input");
    assert_eq!(
        paragraphs(&p, "original"),
        vec![
            "Planning notes",
            "Before: Budget 😀 stays.",
            "After:  here.",
            ""
        ]
    );
    let unrelated = resolve(&p, 2, "accept");
    assert_eq!(
        rev::list(&unrelated).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    save(&unrelated, "unrelated-reviewed");
}
fn paragraph_side(from: bool, id: u32) -> String {
    let tag = if from { "moveFrom" } else { "moveTo" };
    format!("<q:{tag}RangeStart {} q:name=\"paragraphMove\"/><q:p><q:pPr><q:rPr><q:{tag} {}/></q:rPr></q:pPr><q:{tag} {}><q:r><q:t>Moving paragraph.</q:t></q:r></q:{tag}></q:p><q:{tag}RangeEnd q:id=\"{id}\"/>",meta(id),meta(id+1),meta(id+2))
}
#[test]
fn named_moves_include_whole_paragraph_marks_at_story_end_and_in_cells() {
    let p = package(&format!(
        "<q:p><q:r><q:t>Before.</q:t></q:r></q:p>{}<q:p><q:r><q:t>Stay.</q:t></q:r></q:p>{}",
        paragraph_side(true, 0),
        paragraph_side(false, 10)
    ));
    let list = rev::list(&p).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 4);
    for r in list["revisions"].as_array().unwrap() {
        assert_eq!(r["canAccept"], true);
        assert_eq!(r["canReject"], true);
        assert_eq!(r["move"]["memberCount"], 4);
    }
    assert_eq!(
        paragraphs(&p, "current"),
        vec!["Before.", "Stay.", "Moving paragraph."]
    );
    assert_eq!(
        paragraphs(&p, "original"),
        vec!["Before.", "Moving paragraph.", "Stay."]
    );
    let a = resolve(&p, 0, "accept");
    let r = resolve(&p, 3, "reject");
    assert_eq!(
        paragraphs(&a, "current"),
        vec!["Before.", "Stay.", "Moving paragraph."]
    );
    assert_eq!(
        paragraphs(&r, "current"),
        vec!["Before.", "Moving paragraph.", "Stay."]
    );
    save(&p, "paragraph-input");
    save(&a, "paragraph-accepted");
    save(&r, "paragraph-rejected");
    let cells=package(&format!("<q:tbl><q:tblPr/><q:tblGrid><q:gridCol q:w=\"3000\"/><q:gridCol q:w=\"3000\"/></q:tblGrid><q:tr><q:tc>{}</q:tc><q:tc>{}</q:tc></q:tr></q:tbl>",paragraph_side(true,0),paragraph_side(false,10)));
    let a = resolve(&cells, 0, "accept");
    let r = resolve(&cells, 3, "reject");
    for pkg in [&a, &r] {
        let d = parse(pkg.text("word/document.xml").unwrap()).unwrap();
        for cell in d.descendants().filter(|n| n.has_tag_name((W, "tc"))) {
            assert!(cell
                .children()
                .filter(|n| n.is_element())
                .last()
                .unwrap()
                .has_tag_name((W, "p")));
        }
        assert_eq!(
            d.descendants()
                .filter(|n| n.has_tag_name((W, "t")))
                .filter_map(|n| n.text())
                .collect::<String>(),
            "Moving paragraph."
        );
    }
    save(&a, "cells-accepted");
    save(&r, "cells-rejected");
}
#[test]
fn named_moves_reject_ambiguous_or_incomplete_ranges_without_partial_changes() {
    let p = input();
    let xml = p.text("word/document.xml").unwrap();
    for broken in [
        xml.replace("budgetMove\"/><q:moveTo", "differentMove\"/><q:moveTo"),
        xml.replace("<q:moveFromRangeEnd q:id=\"10\"/>", ""),
        xml.replace(
            "q:name=\"budgetMove\"",
            "q:colFirst=\"0\" q:name=\"budgetMove\"",
        ),
        xml.replace(
            "<q:moveFromRangeEnd q:id=\"10\"/>",
            "<q:moveFromRangeEnd q:id=\"10\" q:displacedByCustomXml=\"next\"/>",
        ),
        xml.replace(
            "<q:moveFromRangeEnd q:id=\"10\"/>",
            "<q:moveFromRangeEnd q:id=\"10\"/><q:moveFromRangeEnd q:id=\"10\"/>",
        ),
        xml.replace(
            "<q:moveFrom ",
            &format!(
                "<q:moveFromRangeStart {} q:name=\"overlap\"/><q:moveFrom ",
                meta(80)
            ),
        )
        .replace(
            "<q:moveFromRangeEnd q:id=\"10\"/>",
            "<q:moveFromRangeEnd q:id=\"80\"/><q:moveFromRangeEnd q:id=\"10\"/>",
        ),
    ] {
        let mut copy = p.clone();
        copy.set("word/document.xml", broken);
        let bytes = copy.bytes().unwrap();
        let list = rev::list(&copy).unwrap();
        assert_eq!(list["revisions"][0]["canAccept"], false);
        assert!(rev::resolve(
            &copy,
            &json!({"action":"accept","keys":[list["revisions"][0]["key"]]})
        )
        .is_err());
        assert_eq!(copy.bytes().unwrap(), bytes);
    }
    // Removing an enclosing deletion on only one side must not orphan its pair.
    let from = format!(
        "<q:del {}>{}</q:del>",
        meta(40),
        side(true, 10, "Budget 😀")
    );
    let crossing = package(&format!(
        "<q:p>{from}</q:p><q:p>{}</q:p>",
        side(false, 20, "Budget 😀")
    ));
    let list = rev::list(&crossing).unwrap();
    let before = crossing.bytes().unwrap();
    let error = rev::resolve(
        &crossing,
        &json!({"action":"accept","keys":[list["revisions"][0]["key"]]}),
    )
    .err()
    .unwrap();
    assert!(error.contains("tracked move") || error.contains("named range"));
    assert_eq!(crossing.bytes().unwrap(), before);
    let missing_markers = xml.replace("<q:moveFromRangeEnd q:id=\"10\"/>", "");
    assert!(rev::check_retained(xml, &missing_markers).is_err());
    // A text box is a separate story even when serialized within the body.
    let textbox = format!("<q:p><q:r><q:pict xmlns:v=\"urn:schemas-microsoft-com:vml\"><v:shape><v:textbox><q:txbxContent><q:p>{}</q:p></q:txbxContent></v:textbox></v:shape></q:pict></q:r></q:p>", side(false,20,"Budget 😀"));
    let split_story = package(&format!(
        "<q:p>{}</q:p>{textbox}",
        side(true, 10, "Budget 😀")
    ));
    let split_list = rev::list(&split_story).unwrap();
    assert_eq!(split_list["revisions"].as_array().unwrap().len(), 2);
    assert!(split_list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["canAccept"] == false && r["canReject"] == false));
}

#[test]
fn named_moves_multiple_members_and_nested_deletions_keep_retained_markup() {
    let make_side = |from: bool, id: u32| {
        let tag = if from { "moveFrom" } else { "moveTo" };
        let text_tag = if from { "delText" } else { "t" };
        format!("<q:{tag}RangeStart {} q:name=\"severalRuns\"/><q:{tag} {}><q:r><q:{text_tag}>One </q:{text_tag}></q:r><q:del {}><q:r><q:delText>earlier</q:delText></q:r></q:del></q:{tag}><q:{tag} {}><q:r><q:{text_tag}>two</q:{text_tag}></q:r></q:{tag}><q:{tag}RangeEnd q:id=\"{id}\"/>",meta(id),meta(id+1),meta(id+2),meta(id+3))
    };
    let p = package(&format!(
        "<q:p>{}</q:p><q:p>{}</q:p>",
        make_side(true, 0),
        make_side(false, 10)
    ));
    let list = rev::list(&p).unwrap();
    let moves = list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["move"].is_object())
        .collect::<Vec<_>>();
    assert_eq!(moves.len(), 4);
    for r in moves {
        assert_eq!(r["move"]["memberCount"], 4);
        for action in ["accept", "reject"] {
            let resolved = rev::resolve(&p, &json!({"action":action,"keys":[r["key"]]})).unwrap();
            let pending = rev::list(&resolved).unwrap();
            assert_eq!(pending["revisions"].as_array().unwrap().len(), 1);
            assert_eq!(pending["revisions"][0]["tag"], "del");
            let x = resolved.text("word/document.xml").unwrap();
            let d = parse(x).unwrap();
            let deleted = d
                .descendants()
                .filter(|n| n.has_tag_name((W, "delText")))
                .collect::<Vec<_>>();
            assert_eq!(deleted.len(), 1);
            assert_eq!(deleted[0].text(), Some("earlier"));
            let expected = if action == "accept" {
                vec!["", "One two"]
            } else {
                vec!["One two", ""]
            };
            assert_eq!(paragraphs(&resolved, "current"), expected);
            save(&resolved, &format!("nested-{action}"));
        }
    }
    save(&p, "nested-input");
}

#[test]
fn named_moves_story_scopes_and_hidden_boundaries_survive_ordinary_export() {
    let p = package(&format!(
        "{}<q:p><q:r><q:t>Stay.</q:t></q:r></q:p>{}",
        paragraph_side(true, 0),
        paragraph_side(false, 10)
    ));
    let path = std::env::temp_dir().join("folio-moves-hidden-input.docx");
    std::fs::write(&path, p.bytes().unwrap()).unwrap();
    let mut core = Core::default();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    let imported = core
        .dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut doc = imported["workspace"]["document"].clone();
    let blocks = doc["content"].as_array_mut().unwrap();
    assert_eq!(
        blocks
            .iter()
            .filter(|n| n["attrs"]["hiddenReviewMarker"] == true)
            .count(),
        4
    );
    let stay = blocks.iter_mut().find(|n| text(n) == "Stay.").unwrap();
    stay["content"][0]["text"] = json!("Stay here.");
    core.dispatch(json!({"op":"document","document":doc}))
        .unwrap();
    let output = std::env::temp_dir().join("folio-moves-hidden-export.docx");
    core.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
        .unwrap();
    let exported = Package::from_bytes(std::fs::read(output).unwrap()).unwrap();
    let changes = rev::list(&exported).unwrap();
    assert!(changes["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["canAccept"] == true && r["canReject"] == true));
    assert_eq!(
        paragraphs(&exported, "current"),
        vec!["Stay here.", "Moving paragraph."]
    );
    let note =
        |id, body: String| format!("<q:footnote q:id=\"{id}\"><q:p>{body}</q:p></q:footnote>");
    let mut notes = input();
    let types=notes.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/footnotes.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml\"/></Types>");
    notes.set("[Content_Types].xml", types);
    notes.set(
        "word/footnotes.xml",
        format!(
            "<q:footnotes xmlns:q=\"{W}\">{}{}</q:footnotes>",
            note(
                1,
                format!("{}{}", side(true, 1, "First"), side(false, 4, "First"))
            ),
            note(
                2,
                format!("{}{}", side(true, 7, "Second"), side(false, 10, "Second"))
            )
        ),
    );
    let list = rev::list(&notes).unwrap();
    let key = list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["part"] == "word/footnotes.xml")
        .unwrap()["key"]
        .clone();
    let result = rev::resolve(&notes, &json!({"action":"accept","keys":[key]})).unwrap();
    assert_eq!(
        rev::list(&result).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    let x = result.text("word/footnotes.xml").unwrap();
    let d = parse(x).unwrap();
    let second = d
        .descendants()
        .find(|n| n.has_tag_name((W, "footnote")) && n.attribute((W, "id")) == Some("2"))
        .unwrap();
    assert_eq!(
        second
            .descendants()
            .filter(|n| crate::docx_moves::is_move(*n))
            .count(),
        2
    );
    let mut split = notes.clone();
    split.set(
        "word/footnotes.xml",
        format!(
            "<q:footnotes xmlns:q=\"{W}\">{}{}</q:footnotes>",
            note(1, side(true, 1, "First")),
            note(2, side(false, 4, "First"))
        ),
    );
    assert!(rev::list(&split).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["part"] == "word/footnotes.xml")
        .all(|r| r["canAccept"] == false));
}
#[test]
fn named_moves_core_history_recovery_and_export_keep_pairing() {
    let path = std::env::temp_dir().join("folio-moves-core-source.docx");
    std::fs::write(&path, input().bytes().unwrap()).unwrap();
    let mut core = Core::default();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let list = core.dispatch(json!({"op":"documentRevisions"})).unwrap();
    core.dispatch(json!({"op":"documentRevision","workspaceId":list["workspaceId"],"baseRevision":list["baseRevision"],"action":"reject","keys":[list["revisions"][1]["key"]]})).unwrap();
    assert_eq!(
        core.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        core.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"],
        list["revisions"]
    );
    core.dispatch(json!({"op":"redo"})).unwrap();
    let saved = std::env::temp_dir().join("folio-moves-workspace.folio");
    core.dispatch(json!({"op":"save","path":saved})).unwrap();
    core.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(
        core.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let output = std::env::temp_dir().join("folio-moves-core-export.docx");
    core.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
        .unwrap();
    let exported = Package::from_bytes(std::fs::read(output).unwrap()).unwrap();
    assert_eq!(
        paragraphs(&exported, "current"),
        vec![
            "Planning notes",
            "Before: Budget 😀 stays.",
            "After:  here.",
            "Unrelated note."
        ]
    );
}
