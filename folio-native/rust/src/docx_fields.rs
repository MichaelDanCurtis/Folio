//! Atomic page fields. Imported source instructions/results survive edits around
//! the field; new fields carry an unresolved cache until a pagination engine runs.
use crate::office::*;
use roxmltree::Node;
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub fn kind(code: &str) -> Option<&str> {
    let mut tokens = code.split_whitespace();
    let raw = tokens.next()?;
    // FOLIO: captions and cross-references (W10.03, W10.04) name what they
    // number or point at: `SEQ Figure`, `REF _Ref12 \h`, `PAGEREF _Toc3`.
    if let Some(first) = ["SEQ", "REF", "PAGEREF"]
        .into_iter()
        .find(|k| k.eq_ignore_ascii_case(raw))
    {
        let target = tokens.next()?;
        if target.starts_with('\\') || !valid_target(target) {
            return None;
        }
        while let Some(t) = tokens.next() {
            match t {
                "\\*" => {
                    let value = tokens.next()?;
                    if !FORMATS.contains(&value) {
                        return None;
                    }
                }
                // SEQ: repeat, hide, next, reset (\r n), reset at heading (\s n).
                // REF: link, paragraph number, relative, no context, above/below.
                "\\c" | "\\h" | "\\n" | "\\w" | "\\p" | "\\f" | "\\d" => {}
                "\\r" | "\\s" if first == "SEQ" => {
                    tokens.next()?.parse::<u32>().ok()?;
                }
                "\\r" => {}
                _ => return None,
            }
        }
        return Some(first);
    }
    let first = ["PAGE", "NUMPAGES", "SECTIONPAGES", "SECTION"]
        .into_iter()
        .find(|k| k.eq_ignore_ascii_case(raw))?;
    // These switches format a value; they never fetch data or execute an action.
    while let Some(t) = tokens.next() {
        if t != "\\*" {
            return None;
        }
        let value = tokens.next()?;
        if !FORMATS.contains(&value) {
            return None;
        }
    }
    Some(first)
}

const FORMATS: [&str; 12] = [
    "MERGEFORMAT",
    "CHARFORMAT",
    "ARABIC",
    "Arabic",
    "ROMAN",
    "roman",
    "ALPHABETIC",
    "alphabetic",
    "Ordinal",
    "CardText",
    "OrdText",
    "Hex",
];

/// A SEQ identifier or bookmark name: letters, digits and underscores.
fn valid_target(t: &str) -> bool {
    !t.is_empty() && t.len() <= 40 && t.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The name a field's instruction points at: the SEQ identifier or the
/// bookmark of a REF or PAGEREF.
pub fn target(code: &str) -> Option<&str> {
    let mut tokens = code.split_whitespace();
    let raw = tokens.next()?;
    ["SEQ", "REF", "PAGEREF"]
        .iter()
        .any(|k| k.eq_ignore_ascii_case(raw))
        .then(|| tokens.next())
        .flatten()
}

fn has_fields(n: Node) -> bool {
    n.descendants().any(|c| {
        c.has_tag_name((W, "fldSimple"))
            || c.has_tag_name((W, "fldChar"))
            || c.has_tag_name((W, "instrText"))
    })
}
fn field(fragment: &str) -> Result<Value, String> {
    let x = format!("<w:p xmlns:w=\"{W}\">{fragment}</w:p>");
    let d = parse(&x)?;
    let root = d.root_element();
    let children = root
        .children()
        .filter(|n| n.is_element())
        .collect::<Vec<_>>();
    let simple = children.len() == 1 && children[0].has_tag_name((W, "fldSimple"));
    let mut code = String::new();
    let mut result = String::new();
    let mut locked = false;
    let mut dirty = false;
    if simple {
        let n = children[0];
        code = n
            .attribute((W, "instr"))
            .ok_or("Missing field instruction")?
            .into();
        locked = matches!(n.attribute((W, "fldLock")), Some("1" | "true" | "on"));
        dirty = matches!(n.attribute((W, "dirty")), Some("1" | "true" | "on"));
        if n.descendants().skip(1).any(has_fields) {
            return Err("Nested field".into());
        }
        result = n
            .descendants()
            .filter(|n| n.has_tag_name((W, "t")))
            .filter_map(|n| n.text())
            .collect();
    } else {
        let mut phase = 0;
        for n in children
            .iter()
            .flat_map(|n| n.descendants())
            .filter(|n| n.is_element())
        {
            if n.has_tag_name((W, "fldChar")) {
                match (phase, n.attribute((W, "fldCharType"))) {
                    (0, Some("begin")) => {
                        phase = 1;
                        locked = matches!(n.attribute((W, "fldLock")), Some("1" | "true" | "on"));
                        dirty = matches!(n.attribute((W, "dirty")), Some("1" | "true" | "on"));
                    }
                    (1, Some("separate")) => phase = 2,
                    (2, Some("end")) => phase = 3,
                    _ => return Err("Nested or incomplete page field".into()),
                }
            } else if n.has_tag_name((W, "instrText")) {
                if phase != 1 {
                    return Err("Unexpected instruction".into());
                }
                code.push_str(n.text().unwrap_or(""));
            } else if n.has_tag_name((W, "t")) {
                if phase != 2 {
                    return Err("Text outside field result".into());
                }
                result.push_str(n.text().unwrap_or(""));
            }
        }
        if phase != 3 {
            return Err("Incomplete page field".into());
        }
    }
    // Keep rich run properties, but leave fields with drawing/object/control or
    // annotation boundaries opaque. No source XML from an unrelated field is used.
    for n in root.descendants().skip(1).filter(|n| n.is_element()) {
        if n.ancestors().any(|a| a.has_tag_name((W, "rPr"))) {
            continue;
        }
        if n.tag_name().namespace() != Some(W)
            || !["fldSimple", "r", "t", "fldChar", "instrText"].contains(&n.tag_name().name())
        {
            return Err("Complex page field result".into());
        }
    }
    let k = kind(&code).ok_or("Unsupported field instruction")?;
    Ok(
        json!({"type":"pageField","text":result,"attrs":{"kind":k,"instruction":code,"locked":locked,"dirty":dirty,"sourceXml":fragment}}),
    )
}
/// None means no fields. Err leaves the complete paragraph in its preserved form.
pub fn project(n: Node) -> Result<Option<(String, Vec<(String, Value)>)>, String> {
    if !has_fields(n) {
        return Ok(None);
    }
    let source = scoped_record(n.document().input_text(), n);
    let d = parse(&source)?;
    let root = d.root_element();
    let children = root
        .children()
        .filter(|n| n.is_element())
        .collect::<Vec<_>>();
    let mut i = 0;
    let mut edits = vec![];
    let mut fields = vec![];
    while i < children.len() {
        let c = children[i];
        if !has_fields(c) {
            i += 1;
            continue;
        }
        let start = i;
        if c.has_tag_name((W, "fldSimple")) {
            i += 1;
        } else {
            if !c.has_tag_name((W, "r"))
                || !c.descendants().any(|n| {
                    n.has_tag_name((W, "fldChar"))
                        && n.attribute((W, "fldCharType")) == Some("begin")
                })
            {
                return Err("Field spans unsupported containers".into());
            }
            loop {
                if i >= children.len() {
                    return Err("Incomplete field".into());
                }
                let end = children[i].descendants().any(|n| {
                    n.has_tag_name((W, "fldChar")) && n.attribute((W, "fldCharType")) == Some("end")
                });
                i += 1;
                if end {
                    break;
                }
            }
        }
        let fragment = children[start..i]
            .iter()
            .map(|n| scoped_record(&source, *n))
            .collect::<String>();
        let mut value = field(&fragment)?;
        let wrapped = format!("<w:p xmlns:w=\"{W}\">{fragment}</w:p>");
        let fd = parse(&wrapped)?;
        let props = fd
            .descendants()
            .find(|n| n.has_tag_name((W, "r")) && n.descendants().any(|n| n.has_tag_name((W, "t"))))
            .and_then(|n| child(n, "rPr"))
            .map(|p| scoped_record(fd.input_text(), p))
            .unwrap_or_default();
        let token = format!(
            "FolioPageField{}x{}",
            fingerprint(source.as_bytes()),
            fields.len()
        );
        if source.contains(&token) {
            return Err("Field token collision".into());
        }
        value["attrs"]["sourceMarks"] = json!([]);
        edits.push((
            children[start].range().start..children[i - 1].range().end,
            format!("<w:r xmlns:w=\"{W}\">{props}<w:t>{token}</w:t></w:r>"),
        ));
        fields.push((token, value));
    }
    Ok(Some((replace_ranges(&source, edits), fields)))
}
pub fn validate(n: &Value) -> Result<(), String> {
    if n["type"] != "pageField" {
        return Ok(());
    }
    let a = &n["attrs"];
    let code = a["instruction"]
        .as_str()
        .ok_or("Missing field instruction")?;
    if kind(code) != a["kind"].as_str() || kind(code).is_none() {
        return Err("Invalid page field instruction".into());
    }
    let text = n["text"].as_str().ok_or("Invalid field result")?;
    if text.len() > 4096 || text.contains(['\n', '\r', '\t']) {
        return Err("Invalid page field result".into());
    }
    if let Some(raw) = a.get("sourceXml") {
        let raw = raw.as_str().ok_or("Invalid page field source")?;
        if raw.len() > 262144 {
            return Err("Page field source is too large".into());
        }
        let original = field(raw)?;
        if original["text"] != n["text"]
            || original["attrs"]["instruction"] != a["instruction"]
            || original["attrs"]["locked"] != a["locked"]
            || original["attrs"]["dirty"] != a["dirty"]
        {
            return Err("Page field source does not match its result".into());
        }
    }
    Ok(())
}
pub fn write(n: &Value, marks: &str) -> String {
    let a = &n["attrs"];
    let empty = json!([]);
    let style = n.get("marks").unwrap_or(&empty);
    if let Some(source) = a["sourceXml"]
        .as_str()
        .filter(|_| a["sourceMarks"] == *style)
    {
        return source.into();
    }
    format!("<w:fldSimple w:instr=\"{}\"{}{}><w:r><w:rPr>{marks}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:fldSimple>",esc(a["instruction"].as_str().unwrap_or(" PAGE ")),if a["locked"]==true{" w:fldLock=\"1\""}else{""},if a["dirty"]==true{" w:dirty=\"1\""}else{""},esc(n["text"].as_str().unwrap_or("?")))
}

/// Direct-child page field spans for placing annotation markers outside atomic
/// fields. Length is the cached visible result, not the instruction text.
pub fn atomic_ranges(n: Node) -> Vec<(std::ops::Range<usize>, usize)> {
    let children = n.children().filter(|c| c.is_element()).collect::<Vec<_>>();
    let mut out = vec![];
    let mut i = 0;
    while i < children.len() {
        let start = i;
        if children[i].has_tag_name((W, "fldSimple")) {
            i += 1;
        } else if children[i].has_tag_name((W, "r"))
            && children[i].descendants().any(|n| {
                n.has_tag_name((W, "fldChar")) && n.attribute((W, "fldCharType")) == Some("begin")
            })
        {
            loop {
                let end = children[i].descendants().any(|n| {
                    n.has_tag_name((W, "fldChar")) && n.attribute((W, "fldCharType")) == Some("end")
                });
                i += 1;
                if end || i == children.len() {
                    break;
                }
            }
        } else {
            i += 1;
            continue;
        }
        let fragment = children[start..i]
            .iter()
            .map(|c| scoped_record(n.document().input_text(), *c))
            .collect::<String>();
        if let Ok(value) = field(&fragment) {
            out.push((
                children[start].range().start..children[i - 1].range().end,
                value["text"].as_str().unwrap().encode_utf16().count(),
            ));
        }
    }
    out
}
