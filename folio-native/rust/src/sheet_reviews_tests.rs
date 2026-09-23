use crate::{office::*, sheet_reviews as reviews, workspace::Core, xlsx};
use serde_json::{json, Value};
fn sheet() -> Value {
    json!({"id":"s","name":"Review","cells":{"A1":"2","A2":"1","B1":"label"}})
}
fn add(s: &Value, a: &str, kind: &str) -> Value {
    reviews::command(s,&json!({"action":"add","address":a,"kind":kind,"author":"Reviewer","text":"Check 😀 _x0041_ & values"})).unwrap()
}
fn change(s: &Value, a: &str, action: &str) -> Value {
    reviews::command(s,&json!({"action":action,"address":a,"kind":s[reviews::KEY][a]["kind"],"author":"Second reviewer","text":"Confirmed"})).unwrap()
}
fn export(sheets: &Value, pkg: Option<&Package>, baseline: Option<&Value>) -> Package {
    Package::from_bytes(xlsx::export(sheets, &json!({}), &json!({}), pkg, baseline).unwrap())
        .unwrap()
}
#[test]
fn sheet_review_notes_and_thread_roundtrip_and_deletion() {
    let s = add(&sheet(), "B1", "note");
    let s = add(&s, "A2", "comment");
    let s = change(&s, "A2", "reply");
    let s = change(&s, "A2", "resolve");
    let p = export(&json!([s]), None, None);
    let (round, _) = xlsx::import(&p).unwrap();
    assert_eq!(round[0][reviews::KEY]["B1"]["kind"], "note");
    assert_eq!(round[0][reviews::KEY]["A2"]["kind"], "comment");
    assert_eq!(
        round[0][reviews::KEY]["A2"]["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(round[0][reviews::KEY]["A2"]["resolved"], true);
    assert_eq!(
        round[0][reviews::KEY]["B1"]["messages"][0]["text"],
        "Check 😀 _x0041_ & values"
    );
    assert_eq!(
        xlsx::export(&round, &json!({}), &json!({}), Some(&p), Some(&round)).unwrap(),
        p.original
    );
    std::fs::write(
        std::env::temp_dir().join("folio-sheet-reviews.xlsx"),
        &p.original,
    )
    .unwrap();
    let edited = change(&round[0], "B1", "edit");
    let edited = change(&edited, "A2", "delete");
    let out = export(&json!([edited]), Some(&p), Some(&round));
    let r = xlsx::import(&out).unwrap().0;
    assert!(r[0][reviews::KEY].get("A2").is_none());
    assert_eq!(r[0][reviews::KEY]["B1"]["messages"][0]["text"], "Confirmed");
}
#[test]
fn sheet_review_core_structures_sort_copy_history_and_recovery() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":id,"address":"A1","value":"2"},{"sheetId":id,"address":"A2","value":"1"}]})).unwrap();
    for (address, kind) in [("A1", "comment"), ("A2", "note")] {
        c.dispatch(json!({"op":"sheetReview","sheetId":id,"workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"add","kind":kind,"address":address,"author":"R","text":address})).unwrap();
    }
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"sheetReview","sheetId":id,"workspaceId":c.workspace["id"],"baseRevision":0,"action":"delete","kind":"note","address":"A2"})).is_err());
    assert_eq!(c.workspace, before);
    let file = std::env::temp_dir().join(format!("folio-review-core-{}.xlsx", std::process::id()));
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":file}))
        .unwrap();
    c.dispatch(json!({"op":"importOffice","path":file}))
        .unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"structure","sheetId":id,"action":"insertRows","at":1,"count":2}))
        .unwrap();
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["A3"]["kind"],
        "comment"
    );
    assert_eq!(c.workspace["sheets"][0][reviews::KEY]["A4"]["kind"], "note");
    c.dispatch(json!({"op":"sortRange","sheetId":id,"range":"A3:A4","headers":false,"keys":[{"column":0,"descending":false}]})).unwrap();
    assert_eq!(c.workspace["sheets"][0][reviews::KEY]["A3"]["kind"], "note");
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":id,"range":"A3:A4"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":id,"range":"C3","mode":"comments","token":copied["token"]})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["C4"]["kind"],
        "comment"
    );
    assert!(c.workspace["sheets"][0]["cells"].get("C3").is_none());
    assert_ne!(
        c.workspace["sheets"][0][reviews::KEY]["C4"]["messages"][0]["id"],
        c.workspace["sheets"][0][reviews::KEY]["A4"]["messages"][0]["id"]
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert!(c.workspace["sheets"][0][reviews::KEY].get("C3").is_none());
    c.dispatch(json!({"op":"redo"})).unwrap();
    c.dispatch(json!({"op":"structure","sheetId":id,"action":"deleteColumns","at":1,"count":1}))
        .unwrap();
    assert!(c.workspace["sheets"][0][reviews::KEY].get("A3").is_none());
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["B4"]["kind"],
        "comment"
    );
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":file}))
        .unwrap();
    let round = xlsx::import(&Package::read(file.to_str().unwrap()).unwrap())
        .unwrap()
        .0;
    assert_eq!(round[0][reviews::KEY]["B4"]["kind"], "comment");
}

#[test]
fn sheet_review_missing_reply_refs_follow_legacy_anchor_and_delete_together() {
    let s = change(&add(&sheet(), "A2", "comment"), "A2", "reply");
    let mut p = export(&json!([s]), None, None);
    let thread = p
        .entries
        .keys()
        .find(|p| p.starts_with("xl/threadedComments/"))
        .unwrap()
        .clone();
    let xml = p.text(&thread).unwrap().to_string();
    let dom = parse(&xml).unwrap();
    let edits = dom
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .map(|n| {
            (
                n.range(),
                crate::xlsx_tables::attributes(
                    &xml,
                    n,
                    &[(
                        "ref",
                        if attr(n, "parentId").is_some() {
                            None
                        } else {
                            Some("A9".into())
                        },
                    )],
                ),
            )
        })
        .collect();
    p.set(&thread, replace_ranges(&xml, edits));
    p.original = p.bytes().unwrap();
    let file = std::env::temp_dir().join(format!("folio-review-ref-{}.xlsx", std::process::id()));
    std::fs::write(&file, &p.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":file}))
        .unwrap();
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["A2"]["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"structure","sheetId":id,"action":"deleteRows","at":2,"count":1}))
        .unwrap();
    assert!(reviews::entries(&c.workspace["sheets"][0]).next().is_none());
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":file}))
        .unwrap();
    let deleted = Package::read(file.to_str().unwrap()).unwrap();
    assert_eq!(
        parse(deleted.text(&thread).unwrap())
            .unwrap()
            .descendants()
            .filter(|n| n.tag_name().name() == "threadedComment")
            .count(),
        0
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["A2"]["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let recovery = file.with_extension("folio");
    c.dispatch(json!({"op":"save","path":recovery})).unwrap();
    let mut recovered = Core::new();
    recovered
        .dispatch(json!({"op":"load","path":recovery}))
        .unwrap();
    assert_eq!(
        recovered.workspace["sheets"][0][reviews::KEY],
        c.workspace["sheets"][0][reviews::KEY]
    );
    c.dispatch(json!({"op":"sortRange","sheetId":id,"range":"A1:A2","headers":false,"keys":[{"column":0,"descending":false}]})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0][reviews::KEY]["A1"]["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":file}))
        .unwrap();
    let sorted = Package::read(file.to_str().unwrap()).unwrap();
    for n in parse(sorted.text(&thread).unwrap())
        .unwrap()
        .descendants()
        .filter(|n| n.tag_name().name() == "threadedComment")
    {
        assert_eq!(attr(n, "ref"), Some("A1"));
    }
}

#[test]
fn sheet_review_rich_notes_preserve_runs_and_shape_types_on_copy() {
    let s = add(&sheet(), "B1", "note");
    let mut p = export(&json!([s]), None, None);
    let comments = p
        .entries
        .keys()
        .find(|p| p.starts_with("xl/comments/"))
        .unwrap()
        .clone();
    let xml = p.text(&comments).unwrap().to_string();
    let dom = parse(&xml).unwrap();
    let text = dom
        .descendants()
        .find(|n| n.tag_name().name() == "text")
        .unwrap();
    p.set(
        &comments,
        replace_ranges(
            &xml,
            vec![(
                text.range(),
                "<text><r><rPr><b/></rPr><t>Rich note</t></r></text>".into(),
            )],
        ),
    );
    p.original = p.bytes().unwrap();
    let baseline = xlsx::import(&p).unwrap().0;
    assert_eq!(
        baseline[0][reviews::KEY]["B1"]["messages"][0]["readOnly"],
        true
    );
    assert!(reviews::command(
        &baseline[0],
        &json!({"action":"edit","address":"B1","kind":"note","text":"Flatten"})
    )
    .is_err());
    let mut target = sheet();
    target["name"] = json!("Copied");
    target["id"] = json!("copy");
    reviews::copy_cells(&baseline[0], &mut target, &[("B1".into(), "C3".into())]).unwrap();
    let out = export(&json!([target]), None, None);
    let round = xlsx::import(&out).unwrap().0;
    assert_eq!(
        round[0][reviews::KEY]["C3"]["messages"][0]["text"],
        "Rich note"
    );
    assert!(round[0][reviews::KEY]["C3"]["sourceXml"]
        .as_str()
        .unwrap()
        .contains("<b/>"));
    let shape = round[0][reviews::KEY]["C3"]["sourceVml"].as_str().unwrap();
    let kind = round[0][reviews::KEY]["C3"]["sourceVmlType"]
        .as_str()
        .unwrap();
    assert_eq!(
        attr(parse(shape).unwrap().root_element(), "type")
            .unwrap()
            .trim_start_matches('#'),
        attr(parse(kind).unwrap().root_element(), "id").unwrap()
    );
    std::fs::write(
        std::env::temp_dir().join("folio-sheet-review-rich.xlsx"),
        &out.original,
    )
    .unwrap();
}

#[test]
fn sheet_review_protected_and_unrepresented_cells_reject_edits_atomically() {
    let s = add(&sheet(), "A2", "comment");
    let original = export(&json!([s]), None, None);
    let file = std::env::temp_dir().join(format!("folio-review-guard-{}.xlsx", std::process::id()));
    for protected in [true, false] {
        let mut p = original.clone();
        let address = if protected { "B1" } else { "A2" };
        if protected {
            let path = xlsx::import(&p).unwrap().0[0]["officePath"]
                .as_str()
                .unwrap()
                .to_string();
            let xml = p.text(&path).unwrap().to_string();
            let dom = parse(&xml).unwrap();
            let at = child(dom.root_element(), "sheetData").unwrap().range().end;
            p.set(
                &path,
                replace_ranges(
                    &xml,
                    vec![(
                        at..at,
                        "<sheetProtection sheet=\"1\" objects=\"1\"/>".into(),
                    )],
                ),
            );
        } else {
            let people = p
                .entries
                .keys()
                .find(|p| p.starts_with("xl/persons/"))
                .unwrap()
                .clone();
            p.set(&people,"<personList xmlns=\"http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments\"/>".into());
        }
        std::fs::write(&file, p.bytes().unwrap()).unwrap();
        let mut c = Core::new();
        c.dispatch(json!({"op":"importOffice","path":file}))
            .unwrap();
        let before = c.workspace.clone();
        let result = c.dispatch(json!({"op":"sheetReview","sheetId":c.workspace["sheets"][0]["id"],"workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"add","address":address,"kind":"note","author":"New reviewer","text":"Replacement"}));
        assert!(result.is_err(), "{protected}: {result:?}");
        assert_eq!(c.workspace, before);
    }
}
