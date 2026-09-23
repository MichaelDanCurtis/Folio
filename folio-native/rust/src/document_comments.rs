//! Local review threads anchored in the shared UTF-16 body-story coordinates.
use crate::document_bookmarks as ranges;
use serde_json::{json, Value};
use std::collections::HashSet;
pub const KEY: &str = "comments";
pub fn entries(document: &Value) -> &[Value] {
    document["attrs"][KEY]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn string<'a>(value: &'a Value, key: &str, max: usize, empty: bool) -> Result<&'a str, String> {
    let s = value[key]
        .as_str()
        .ok_or(format!("Missing comment {key}"))?;
    if (!empty && s.trim().is_empty())
        || s.chars().count() > max
        || s.chars()
            .any(|c| c < ' ' && c != '\n' && c != '\t' && c != '\r')
    {
        return Err(format!("Invalid comment {key}"));
    }
    Ok(s)
}
pub fn validate(document: &Value) -> Result<(), String> {
    let Some(value) = document["attrs"].get(KEY) else {
        return Ok(());
    };
    let threads = value.as_array().ok_or("Invalid document comments")?;
    if threads.len() > 10000 {
        return Err("Too many comment threads".into());
    }
    let index = ranges::Index::new(document);
    let mut ids = HashSet::new();
    let mut sources = HashSet::new();
    for thread in threads {
        if !ids.insert(string(thread, "id", 160, false)?.to_owned()) {
            return Err("Duplicate comment identifier".into());
        }
        let start = thread["start"].as_u64().ok_or("Invalid comment start")? as usize;
        let end = thread["end"].as_u64().ok_or("Invalid comment end")? as usize;
        if start > end || !index.boundary(start) || !index.boundary(end) {
            return Err("Comment range no longer identifies document text".into());
        }
        if !thread["resolved"].is_boolean() {
            return Err("Invalid comment status".into());
        }
        let messages = thread["messages"]
            .as_array()
            .filter(|m| !m.is_empty() && m.len() <= 1000)
            .ok_or("Invalid comment messages")?;
        for message in messages {
            if !ids.insert(string(message, "id", 160, false)?.to_owned()) {
                return Err("Duplicate comment message identifier".into());
            }
            string(message, "author", 256, true)?;
            string(message, "text", 100000, true)?;
            string(message, "date", 128, true)?;
            if let Some(id) = message.get("sourceId") {
                let id = id
                    .as_str()
                    .filter(|s| s.parse::<u32>().is_ok())
                    .ok_or("Invalid Word comment identifier")?;
                if !sources.insert(id) {
                    return Err("Duplicate Word comment identifier".into());
                }
            }
        }
    }
    Ok(())
}
fn message(r: &Value) -> Result<Value, String> {
    let author = string(r, "author", 256, false)?.trim();
    let text = string(r, "text", 100000, false)?.trim();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Ok(
        json!({"id":format!("message-{stamp}"),"author":author,"text":text,"date":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true)}),
    )
}
pub fn command(document: &Value, request: &Value) -> Result<Value, String> {
    let mut list = entries(document).to_vec();
    let action = request["action"]
        .as_str()
        .ok_or("Choose a comment action")?;
    if action == "add" {
        let m = message(request)?;
        list.push(json!({"id":format!("thread-{}",m["id"].as_str().unwrap()),"start":request["start"],"end":request["end"],"resolved":false,"messages":[m]}));
    } else {
        let i = list
            .iter()
            .position(|t| t["id"] == request["id"])
            .ok_or("This comment no longer exists")?;
        match action {
            "delete" => {
                list.remove(i);
            }
            "resolve" | "reopen" => list[i]["resolved"] = json!(action == "resolve"),
            "reply" => {
                if list[i]["resolved"] == true {
                    return Err("Reopen this thread before replying".into());
                }
                list[i]["messages"]
                    .as_array_mut()
                    .unwrap()
                    .push(message(request)?);
            }
            "edit" | "deleteReply" => {
                if list[i]["resolved"] == true {
                    return Err("Reopen this thread before editing".into());
                }
                let messages = list[i]["messages"].as_array_mut().unwrap();
                let j = messages
                    .iter()
                    .position(|m| m["id"] == request["messageId"])
                    .ok_or("This message no longer exists")?;
                if action == "deleteReply" {
                    if j == 0 {
                        return Err("Use Delete thread to remove the opening comment".into());
                    }
                    messages.remove(j);
                } else {
                    if messages[j]["readOnly"] == true {
                        return Err("This comment contains rich content. Edit its text in Word to retain that content.".into());
                    }
                    messages[j]["text"] = json!(string(request, "text", 100000, false)?.trim());
                }
            }
            _ => return Err("Unknown comment action".into()),
        }
    }
    let mut next = document.clone();
    next["attrs"][KEY] = json!(list);
    validate(&next)?;
    Ok(next)
}
pub fn rebase(previous: &Value, next: &mut Value, exact: Option<&Value>) -> Result<(), String> {
    ranges::rebase_ranges(previous, next, KEY, exact)?;
    validate(next)
}
pub fn hydrate(document: &mut Value, source: &Value) -> Result<(), String> {
    if document["attrs"].get(KEY).is_none() && source["attrs"].get(KEY).is_some() {
        rebase(source, document, None)?;
    }
    Ok(())
}
