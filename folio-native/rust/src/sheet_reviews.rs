//! Cell annotations: notes and threaded conversations are distinct review kinds.
use crate::{document_comments, formula::parse_address};
use serde_json::{json, Value};
pub const KEY: &str = "cellReviews";
pub fn entries(sheet: &Value) -> impl Iterator<Item = (&String, &Value)> {
    sheet[KEY].as_object().into_iter().flat_map(|m| m.iter())
}
fn as_document(sheet: &Value) -> Value {
    json!({"type":"doc","content":[{"type":"paragraph"}],"attrs":{"comments":entries(sheet).map(|(_,v)|{let mut v=v.clone();v["start"]=json!(0);v["end"]=json!(0);v}).collect::<Vec<_>>()}})
}
pub fn validate(sheet: &Value) -> Result<(), String> {
    if sheet
        .get(KEY)
        .is_some_and(|v| !v.is_null() && !v.is_object())
    {
        return Err("Invalid worksheet comments and notes".into());
    }
    for (a, review) in entries(sheet) {
        parse_address(a)?;
        if !matches!(review["kind"].as_str(), Some("note" | "comment")) {
            return Err("Unknown cell annotation kind".into());
        }
        if review["kind"] == "note"
            && (review["messages"].as_array().is_none_or(|m| m.len() != 1)
                || review["resolved"] != false)
        {
            return Err("Notes cannot contain a conversation or resolved state".into());
        }
    }
    document_comments::validate(&as_document(sheet))
}
pub fn command(sheet: &Value, r: &Value) -> Result<Value, String> {
    let a = r["address"].as_str().ok_or("Select a cell")?.to_uppercase();
    parse_address(&a)?;
    let kind = r["kind"]
        .as_str()
        .filter(|k| matches!(*k, "note" | "comment"))
        .ok_or("Choose Comment or Note")?;
    let action = r["action"].as_str().ok_or("Choose a review action")?;
    let mut document = as_document(sheet);
    let old = sheet[KEY].get(&a);
    if action == "add" && old.is_some() {
        return Err("This cell already has a comment or note. Edit it or remove it first.".into());
    }
    if action != "add" && old.is_none_or(|v| v["kind"] != kind) {
        return Err("This cell annotation no longer exists".into());
    }
    if kind == "note" && !matches!(action, "add" | "edit" | "delete") {
        return Err("Notes have no replies or resolved state".into());
    }
    let mut request = r.clone();
    request["start"] = json!(0);
    request["end"] = json!(0);
    if let Some(old) = old {
        request["id"] = old["id"].clone();
        if kind == "note" {
            request["messageId"] = old["messages"][0]["id"].clone();
        }
    }
    document = document_comments::command(&document, &request)?;
    let mut next = sheet.clone();
    if !next[KEY].is_object() {
        next[KEY] = json!({});
    }
    if action == "delete" {
        next[KEY].as_object_mut().unwrap().remove(&a);
    } else {
        let list = document_comments::entries(&document);
        let mut review = if action == "add" {
            list.last().unwrap()
        } else {
            list.iter().find(|v| v["id"] == request["id"]).unwrap()
        }
        .clone();
        review["kind"] = json!(kind);
        review.as_object_mut().unwrap().remove("start");
        review.as_object_mut().unwrap().remove("end");
        next[KEY][&a] = review;
    }
    validate(&next)?;
    Ok(next)
}
/// Copied threads get fresh identities; authors remain explicit source records.
/// Opaque VML image relationships cannot be transplanted without their package.
pub fn copied(review: &Value) -> Result<Value, String> {
    if review["vmlRelationships"] == true {
        return Err("Copying a note with embedded drawing relationships is not supported yet. Nothing changed.".into());
    }
    if review["kind"] == "comment"
        && review["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["readOnly"] == true)
    {
        return Err("Copying comments with mentions needs additional author-reference support. Nothing changed.".into());
    }
    let mut next = review.clone();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    next["id"] = json!(format!("cell-review-{stamp}"));
    next.as_object_mut().unwrap().remove("guid");
    for (i, m) in next["messages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        m["id"] = json!(format!("cell-message-{stamp}-{i}"));
        m.as_object_mut().unwrap().remove("parentId");
    }
    Ok(next)
}
pub fn copy_cells(
    source: &Value,
    target: &mut Value,
    copies: &[(String, String)],
) -> Result<(), String> {
    for (from, to) in copies {
        if let Some(review) = source[KEY].get(from) {
            if !target[KEY].is_object() {
                target[KEY] = json!({});
            }
            target[KEY][to] = copied(review)?;
        } else if let Some(map) = target.get_mut(KEY).and_then(Value::as_object_mut) {
            map.remove(to);
        }
    }
    Ok(())
}
