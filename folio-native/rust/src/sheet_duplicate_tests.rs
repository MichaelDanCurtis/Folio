//! Duplicate sheet (X03.06).
use crate::sheet_duplicate::copy_name;
use crate::workspace::Core;
use serde_json::{json, Value};

#[test]
fn copies_are_named_like_excel() {
    let taken = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(copy_name("Budget", &taken(&["Budget"])), "Budget (2)");
    assert_eq!(
        copy_name("Budget", &taken(&["Budget", "budget (2)"])),
        "Budget (3)"
    );
    assert_eq!(
        copy_name("Budget (2)", &taken(&["Budget", "Budget (2)"])),
        "Budget (3)"
    );
    let long = "A".repeat(31);
    let copy = copy_name(&long, &taken(&[&long]));
    assert_eq!(copy.chars().count(), 31);
    assert!(copy.ends_with(" (2)"));
}

fn names(core: &Core) -> Vec<String> {
    core.workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_copy_goes_after_its_source_as_one_undo_step() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    core.dispatch(json!({"op":"addSheet"})).unwrap();
    let id = core.workspace["sheets"][0]["id"].clone();
    core.dispatch(json!({"op":"cells","cells":[
        {"sheetId":id,"address":"A1","value":"5"},
        {"sheetId":id,"address":"B1","value":"=A1*2"}
    ]}))
    .unwrap();
    core.dispatch(json!({"op":"formatCells","sheetId":id,"addresses":["A1"],
        "style":{"bold":true}}))
        .unwrap();
    core.dispatch(json!({"op":"sheetView","sheetId":id,"action":"zoom","zoom":150}))
        .unwrap();
    core.dispatch(
        json!({"op":"sheetReview","sheetId":id,"workspaceId":core.workspace["id"],
        "baseRevision":core.workspace["revision"],"action":"add","kind":"note","address":"A1",
        "author":"R","text":"Check"}),
    )
    .unwrap();
    let before = core.workspace["sheets"].clone();
    core.dispatch(json!({"op":"duplicateSheet","sheetId":id,
        "baseRevision":core.workspace["revision"]}))
        .unwrap();
    let first = names(&core)[0].clone();
    assert_eq!(
        names(&core),
        vec![
            first.clone(),
            format!("{first} (2)"),
            names(&core)[2].clone()
        ]
    );
    let copy = core.workspace["sheets"][1].clone();
    assert_ne!(copy["id"], id);
    assert_eq!(copy["cells"]["B1"], "=A1*2");
    assert_eq!(copy["cellStyles"]["A1"]["bold"], true);
    assert_eq!(copy["zoom"], 150);
    assert_eq!(copy["cellReviews"]["A1"]["kind"], "note");
    let copy_id = copy["id"].as_str().unwrap().to_string();
    assert_eq!(core.snapshot()["calculated"][&copy_id]["B1"], json!(10.0));

    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(core.workspace["sheets"], before, "one undo step");
    core.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(names(&core)[1], format!("{first} (2)"));
}

#[test]
fn copied_tables_get_new_names_and_the_copy_round_trips() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/tables.xlsx")}))
        .unwrap();
    let (index, source) = core.workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .find(|(_, s)| s["tables"].as_array().is_some_and(|t| !t.is_empty()))
        .map(|(i, s)| (i, s.clone()))
        .unwrap();
    let old: Vec<String> = source["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    core.dispatch(json!({"op":"cells","cells":[
        {"sheetId":source["id"],"address":"Z1","value":format!("=ROWS({}[#All])", old[0])}
    ]}))
    .unwrap();
    let source = core.workspace["sheets"][index].clone();
    core.dispatch(json!({"op":"duplicateSheet","sheetId":source["id"]}))
        .unwrap();
    assert_eq!(
        core.workspace["sheets"][index + 1]["cells"]["Z1"],
        format!(
            "=ROWS({}2[#All])",
            old[0].trim_end_matches(|c: char| c.is_ascii_digit())
        )
    );
    let copy = core.workspace["sheets"][index + 1].clone();
    assert_eq!(
        copy["name"],
        format!("{} (2)", source["name"].as_str().unwrap())
    );
    let new: Vec<String> = copy["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(new.len(), old.len());
    for (o, n) in old.iter().zip(&new) {
        assert_ne!(o, n);
    }
    // Formulas in the copy that name a table name the copy's table.
    for (a, v) in source["cells"].as_object().unwrap() {
        let v = v.as_str().unwrap();
        if old.iter().any(|o| v.contains(&format!("{o}["))) {
            let copied = copy["cells"][a].as_str().unwrap();
            assert!(
                new.iter().any(|n| copied.contains(&format!("{n}["))),
                "{a}: {copied}"
            );
        }
    }

    let out = std::env::temp_dir().join(format!("folio-duplicate-{}.xlsx", std::process::id()));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    assert_eq!(names(&again), names(&core));
    let reopened: &Value = &again.workspace["sheets"][index + 1];
    assert_eq!(reopened["cells"], copy["cells"]);
    let reopened_tables: Vec<&str> = reopened["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        reopened_tables,
        new.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn duplicate_refuses_bad_requests() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    let before = core.workspace.clone();
    assert!(core
        .dispatch(json!({"op":"duplicateSheet","sheetId":"missing"}))
        .is_err());
    assert!(core
        .dispatch(
            json!({"op":"duplicateSheet","sheetId":before["sheets"][0]["id"],"baseRevision":-1})
        )
        .is_err());
    assert_eq!(core.workspace, before);
}
