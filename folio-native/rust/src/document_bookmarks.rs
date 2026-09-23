//! Durable body-story ranges. Positions are UTF-16 offsets in logical paragraphs,
//! including table cells and list items. Each paragraph has one trailing newline;
//! unmodeled blocks occupy U+FFFC + newline. They are not Quill embed offsets.
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Debug)]
pub struct Paragraph {
    pub path: Vec<usize>,
    pub start: usize,
    pub end: usize,
}
#[derive(Default, Debug)]
pub struct Index {
    pub text: String,
    pub paragraphs: Vec<Paragraph>,
    length: usize,
    boundaries: Vec<usize>,
}
pub fn at<'a>(mut node: &'a Value, path: &[usize]) -> &'a Value {
    for &i in path {
        node = &node["content"][i];
    }
    node
}
pub fn at_mut<'a>(mut node: &'a mut Value, path: &[usize]) -> &'a mut Value {
    for &i in path {
        node = &mut node["content"][i];
    }
    node
}
impl Index {
    pub fn new(document: &Value) -> Self {
        fn visit(n: &Value, path: &mut Vec<usize>, out: &mut Index) {
            match n["type"].as_str().unwrap_or("") {
                "paragraph" | "heading" => {
                    let text = crate::workspace::document_text(n);
                    let mut boundary = out.length;
                    out.boundaries.push(boundary);
                    for c in text.chars() {
                        boundary += c.len_utf16();
                        out.boundaries.push(boundary);
                    }
                    let end = out.length + text.encode_utf16().count();
                    out.paragraphs.push(Paragraph {
                        path: path.clone(),
                        start: out.length,
                        end,
                    });
                    out.text.push_str(&text);
                    out.text.push('\n');
                    out.length = end + 1;
                }
                "doc" | "table" | "tableRow" | "tableCell" | "tableHeader" | "bulletList"
                | "orderedList" | "listItem" | "blockquote" => {
                    if let Some(children) = n["content"].as_array() {
                        for (i, child) in children.iter().enumerate() {
                            path.push(i);
                            visit(child, path, out);
                            path.pop();
                        }
                    }
                }
                _ => {
                    out.text.push_str("\u{fffc}\n");
                    out.length += 2;
                }
            }
        }
        let mut out = Self::default();
        visit(document, &mut vec![], &mut out);
        out
    }
    pub fn paragraph(&self, offset: usize) -> Option<&Paragraph> {
        let i = self.paragraphs.partition_point(|p| p.start <= offset);
        i.checked_sub(1)
            .and_then(|i| self.paragraphs.get(i))
            .filter(|p| offset <= p.end)
    }
    pub(crate) fn boundary(&self, offset: usize) -> bool {
        self.boundaries.binary_search(&offset).is_ok()
    }
}
pub fn entries(doc: &Value) -> &[Value] {
    doc["attrs"]["bookmarks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
pub fn valid_name(name: &str, hidden: bool) -> bool {
    !name.is_empty()
        && name.chars().count() <= 40
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || hidden && c == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}
pub fn validate(doc: &Value) -> Result<(), String> {
    let Some(value) = doc["attrs"].get("bookmarks") else {
        return Ok(());
    };
    let bookmarks = value.as_array().ok_or("Invalid document bookmarks")?;
    if bookmarks.len() > 10000 {
        return Err("Too many document bookmarks".into());
    }
    let index = Index::new(doc);
    let mut names = HashSet::new();
    let mut source_ids = HashSet::new();
    for b in bookmarks {
        let name = b["name"].as_str().ok_or("Bookmark name is missing")?;
        if !valid_name(name, true) || !names.insert(name.to_lowercase()) {
            return Err("Bookmark names must be unique and contain letters, numbers or underscores (40 characters maximum).".into());
        }
        let start = b["start"].as_u64().ok_or("Invalid bookmark start")? as usize;
        let end = b["end"].as_u64().ok_or("Invalid bookmark end")? as usize;
        if start > end || !index.boundary(start) || !index.boundary(end) {
            return Err("Bookmark range does not identify document text".into());
        }
        if let Some(id) = b.get("sourceId") {
            if id.as_str().is_none_or(|s| s.parse::<u64>().is_err()) {
                return Err("Invalid Word bookmark identifier".into());
            }
            if !source_ids.insert(id.as_str().unwrap()) {
                return Err("Duplicate Word bookmark identifier".into());
            }
        }
    }
    Ok(())
}

// Text edits use old-document coordinates and must reconstruct the new text.
// Exact edits disambiguate repeated text (e.g. typing another 'a' in 'aaaa').
#[derive(Debug)]
struct Edit {
    start: usize,
    end: usize,
    inserted: usize,
}
fn edits(before: &str, after: &str, exact: Option<&Value>) -> Result<Vec<Edit>, String> {
    if let Some(exact) = exact {
        let old: Vec<u16> = before.encode_utf16().collect();
        let mut reconstructed = vec![];
        let mut cursor = 0;
        let mut result = vec![];
        for e in exact.as_array().ok_or("Invalid bookmark text edits")? {
            let start = e["start"].as_u64().ok_or("Invalid edit start")? as usize;
            let end = e["end"].as_u64().ok_or("Invalid edit end")? as usize;
            let text = e["text"].as_str().ok_or("Invalid inserted text")?;
            if start < cursor
                || end < start
                || end > old.len()
                || String::from_utf16(&old[cursor..start]).is_err()
                || String::from_utf16(&old[start..end]).is_err()
            {
                return Err("Bookmark text edits overlap or split a character".into());
            }
            reconstructed.extend_from_slice(&old[cursor..start]);
            reconstructed.extend(text.encode_utf16());
            result.push(Edit {
                start,
                end,
                inserted: text.encode_utf16().count(),
            });
            cursor = end;
        }
        reconstructed.extend_from_slice(&old[cursor..]);
        if reconstructed != after.encode_utf16().collect::<Vec<_>>() {
            return Err("Bookmark text edits no longer match the document".into());
        }
        return Ok(result);
    }
    let diff = similar::TextDiff::configure()
        .timeout(std::time::Duration::from_millis(100))
        .diff_chars(before, after);
    let mut old = 0;
    let mut result = vec![];
    let mut pending: Option<Edit> = None;
    for change in diff.iter_all_changes() {
        let len = change.value().encode_utf16().count();
        match change.tag() {
            similar::ChangeTag::Equal => {
                if let Some(edit) = pending.take() {
                    result.push(edit);
                }
                old += len;
            }
            similar::ChangeTag::Delete => {
                pending
                    .get_or_insert(Edit {
                        start: old,
                        end: old,
                        inserted: 0,
                    })
                    .end += len;
                old += len;
            }
            similar::ChangeTag::Insert => {
                pending
                    .get_or_insert(Edit {
                        start: old,
                        end: old,
                        inserted: 0,
                    })
                    .inserted += len;
            }
        }
    }
    if let Some(edit) = pending {
        result.push(edit);
    }
    Ok(result)
}
/// Validate and invert exact edits even when the document has no bookmarks.
/// Both directions use UTF-16 positions in their own source document.
pub fn inverse_edits(before: &Value, after: &Value, exact: &Value) -> Result<Value, String> {
    let values = exact.as_array().ok_or("Invalid document history edits")?;
    if values.len() > 100000 {
        return Err("Too many document history edits".into());
    }
    let old = Index::new(before);
    let new = Index::new(after);
    let checked = edits(&old.text, &new.text, Some(exact))?;
    let source = old.text.encode_utf16().collect::<Vec<_>>();
    let mut shift = 0_i64;
    let mut inverse = Vec::with_capacity(checked.len());
    for edit in checked {
        let start = (edit.start as i64 + shift) as usize;
        let removed = String::from_utf16(&source[edit.start..edit.end])
            .map_err(|_| "Document history splits a character")?;
        inverse.push(json!({"start":start,"end":start+edit.inserted,"text":removed}));
        shift += edit.inserted as i64 - (edit.end - edit.start) as i64;
    }
    Ok(json!(inverse))
}

/// A verified row permutation carries each row's local ranges with it. Text
/// edits inside an identified row are rebased locally, rather than diffing
/// repeated values against a different row after the move.
struct RowPart {
    old_start: usize,
    old_end: usize,
    new_start: usize,
    new_end: usize,
    edits: Vec<Edit>,
}

fn row_permutation(previous: &Value, next: &Value) -> Option<Vec<RowPart>> {
    if previous["type"] != "doc" || next["type"] != "doc" {
        return None;
    }
    let old = previous["content"].as_array()?;
    let new = next["content"].as_array()?;
    if old.len() != new.len() {
        return None;
    }
    let mut pairs = vec![];
    let mut moved = false;
    let mut old_offset = 0;
    let mut new_offset = 0;
    for (a, b) in old.iter().zip(new) {
        if a["type"] != b["type"] || a["attrs"]["officeId"] != b["attrs"]["officeId"] {
            return None;
        }
        if a["type"] == "table" {
            let before = a["content"].as_array()?;
            let after = b["content"].as_array()?;
            if before.len() != after.len() {
                return None;
            }
            let mut used = HashSet::new();
            let mut starts = vec![];
            let mut cursor = old_offset;
            for row in before {
                starts.push(cursor);
                cursor += Index::new(row).length;
            }
            for (i, row) in after.iter().enumerate() {
                let id = row["attrs"]["officeId"].as_str();
                let candidates: Vec<_> = before
                    .iter()
                    .enumerate()
                    .filter(|(j, old)| {
                        !used.contains(j)
                            && (id.is_some() && old["attrs"]["officeId"].as_str() == id
                                || *old == row)
                    })
                    .collect();
                if candidates.is_empty() || id.is_some() && candidates.len() != 1 {
                    return None;
                }
                let (j, original) = candidates[0];
                used.insert(j);
                moved |= i != j;
                let old_text = Index::new(original);
                let new_text = Index::new(row);
                pairs.push(RowPart {
                    old_start: starts[j],
                    old_end: starts[j] + old_text.length,
                    new_start: new_offset,
                    new_end: new_offset + new_text.length,
                    edits: edits(&old_text.text, &new_text.text, None).ok()?,
                });
                new_offset += new_text.length;
            }
            old_offset = cursor;
        } else {
            let old_text = Index::new(a);
            let new_text = Index::new(b);
            pairs.push(RowPart {
                old_start: old_offset,
                old_end: old_offset + old_text.length,
                new_start: new_offset,
                new_end: new_offset + new_text.length,
                edits: edits(&old_text.text, &new_text.text, None).ok()?,
            });
            old_offset += old_text.length;
            new_offset += new_text.length;
        }
    }
    moved.then_some(pairs)
}

fn moved_ranges(
    previous: &Value,
    next: &Value,
    ranges: &[Value],
    key: &str,
) -> Result<Option<Value>, String> {
    let Some(parts) = row_permutation(previous, next) else {
        return Ok(None);
    };
    let new_index = Index::new(next);
    let mut result = vec![];
    for original in ranges {
        let start = original["start"].as_u64().ok_or("Invalid range start")? as usize;
        let end = original["end"].as_u64().ok_or("Invalid range end")? as usize;
        let first = parts
            .iter()
            .find(|p| p.old_start <= start && start < p.old_end)
            .ok_or("The sort could not locate a document range")?;
        let last = if start == end {
            first
        } else {
            parts
                .iter()
                .find(|p| p.old_start < end && end <= p.old_end)
                .ok_or("The sort could not locate a document range")?
        };
        if std::ptr::eq(first, last)
            && first.edits.iter().any(|e| {
                let a = first.old_start + e.start;
                let z = first.old_start + e.end;
                a < z && a <= start && z >= end && (start < end || a < start && start < z)
            })
        {
            continue;
        }
        let map = |part: &RowPart, pos: usize| -> usize {
            let mut local = (pos - part.old_start) as i64;
            for edit in part.edits.iter().rev() {
                let a = edit.start as i64;
                let z = edit.end as i64;
                if local > z || local == z && a < z {
                    local += edit.inserted as i64 - (z - a);
                } else if local >= a {
                    local = a;
                }
            }
            (part.new_start as i64 + local).clamp(part.new_start as i64, part.new_end as i64)
                as usize
        };
        let new_start = map(first, start);
        let new_end = map(last, end);
        if new_start > new_end {
            return Err(format!("This sort would reverse a {key} range spanning multiple rows. Adjust that range before sorting."));
        }
        if !new_index.boundary(new_start) || !new_index.boundary(new_end) {
            return Err("This sort would move a document range outside editable text".into());
        }
        let mut range = original.clone();
        range["start"] = json!(new_start);
        range["end"] = json!(new_end);
        result.push(range);
    }
    Ok(Some(json!(result)))
}

pub fn rebase_ranges(
    previous: &Value,
    next: &mut Value,
    key: &str,
    exact: Option<&Value>,
) -> Result<(), String> {
    let ranges = previous["attrs"][key]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if ranges.is_empty() {
        // Content replacement cannot inject metadata behind the bookmark command.
        if let Some(attrs) = next.get_mut("attrs").and_then(Value::as_object_mut) {
            attrs.remove(key);
        }
        if previous["attrs"].get(key).is_some() {
            next["attrs"][key] = json!([]);
        }
        return Ok(());
    }
    if exact.is_none() && previous["content"] == next["content"] {
        next["attrs"][key] = previous["attrs"][key].clone();
        return Ok(());
    }
    if exact.is_none() {
        if let Some(moved) = moved_ranges(previous, next, ranges, key)? {
            next["attrs"][key] = moved;
            return Ok(());
        }
    }
    let old = Index::new(previous);
    let new = Index::new(next);
    let edits = edits(&old.text, &new.text, exact)?;
    let mut output = vec![];
    for original in ranges {
        let mut b = original.clone();
        let mut start = b["start"].as_u64().ok_or("Invalid bookmark start")? as i64;
        let mut end = b["end"].as_u64().ok_or("Invalid bookmark end")? as i64;
        let mut removed = false;
        // Descending edits keep the remaining edit coordinates valid.
        for e in edits.iter().rev() {
            let a = e.start as i64;
            let z = e.end as i64;
            let count = e.inserted as i64;
            if a < z && a <= start && z >= end && (start < end || a < start && start < z) {
                removed = true;
                break;
            }
            let map = |pos: i64, right: bool| {
                if pos < a {
                    pos
                } else if pos > z {
                    pos + count - (z - a)
                } else if pos == z && a < z {
                    a + count
                } else {
                    a + if right { count } else { 0 }
                }
            };
            // A range includes insertion at its opening, excludes at its end.
            // A point remains immediately before text inserted at that point.
            start = map(start, false);
            end = map(end, false);
        }
        if !removed && start <= end && new.boundary(start as usize) && new.boundary(end as usize) {
            b["start"] = json!(start);
            b["end"] = json!(end);
            output.push(b);
        }
    }
    next["attrs"][key] = json!(output);
    Ok(())
}
pub fn rebase(previous: &Value, next: &mut Value, exact: Option<&Value>) -> Result<(), String> {
    rebase_ranges(previous, next, "bookmarks", exact)?;
    validate(next)
}
pub fn command(document: &Value, request: &Value) -> Result<Value, String> {
    let mut next = document.clone();
    let name = request["name"].as_str().ok_or("Enter a bookmark name")?;
    let mut list = entries(document).to_vec();
    let found = list.iter().position(|b| {
        b["name"]
            .as_str()
            .is_some_and(|n| n.to_lowercase() == name.to_lowercase())
    });
    match request["action"].as_str() {
        Some("delete") => {
            list.remove(found.ok_or("Bookmark no longer exists")?);
        }
        Some("set") => {
            if !valid_name(name, false) {
                return Err("Start the bookmark name with a letter; use letters, numbers or underscores (40 characters maximum).".into());
            }
            let mut b = json!({"name":name,"start":request["start"],"end":request["end"]});
            if let Some(i) = found {
                if let Some(id) = list[i].get("sourceId") {
                    b["sourceId"] = id.clone();
                }
                list[i] = b;
            } else {
                list.push(b);
            }
        }
        _ => return Err("Unknown bookmark action".into()),
    }
    next["attrs"]["bookmarks"] = json!(list);
    validate(&next)?;
    Ok(next)
}

pub fn hydrate(document: &mut Value, source: &Value) -> Result<(), String> {
    // Absence marks a legacy projection; an explicit empty list is a deletion.
    if document["attrs"].get("bookmarks").is_none() && source["attrs"].get("bookmarks").is_some() {
        rebase(source, document, None)?;
    }
    Ok(())
}
