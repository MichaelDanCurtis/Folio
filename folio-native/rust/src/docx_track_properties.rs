//! Apply editor property deltas to authoritative Word properties. The editor's
//! projection is deliberately incomplete; absence there is not a deletion.
use crate::office::*;
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn signature(n: Node) -> String {
    let mut attrs = n
        .attributes()
        .map(|a| (a.namespace(), a.name(), a.value()))
        .collect::<Vec<_>>();
    attrs.sort();
    format!(
        "{:?}:{}:{attrs:?}:{}",
        n.tag_name().namespace(),
        n.tag_name().name(),
        n.children()
            .filter(|n| n.is_element())
            .map(signature)
            .collect::<String>()
    )
}
fn children(xml: &str, root: &str) -> Result<BTreeMap<String, String>, String> {
    if xml.is_empty() {
        return Ok(BTreeMap::new());
    }
    let d = parse(xml)?;
    if !d.root_element().has_tag_name((W, root)) {
        return Err("Mismatched Word properties".into());
    }
    let mut out = BTreeMap::new();
    for n in d.root_element().children().filter(|n| n.is_element()) {
        if n.tag_name().namespace() != Some(W) {
            return Err("The editor cannot change extension properties".into());
        }
        if out
            .insert(n.tag_name().name().into(), scoped_record(xml, n))
            .is_some()
        {
            return Err("Ambiguous duplicate Word property".into());
        }
    }
    Ok(out)
}
fn equal(a: Option<&String>, b: Option<&String>) -> Result<bool, String> {
    Ok(match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            signature(parse(a)?.root_element()) == signature(parse(b)?.root_element())
        }
        _ => false,
    })
}
fn attributes(xml: Option<&String>) -> Result<BTreeMap<String, String>, String> {
    let Some(xml) = xml else {
        return Ok(BTreeMap::new());
    };
    let d = parse(xml)?;
    if d.root_element().children().any(|n| n.is_element()) {
        return Err("Unexpected children in a compound Word setting".into());
    }
    d.root_element()
        .attributes()
        .map(|a| {
            if a.namespace() != Some(W) {
                return Err("Unsupported editor property attribute".into());
            }
            Ok((a.name().into(), a.value().into()))
        })
        .collect()
}
fn merge_attributes(
    source: &str,
    before: Option<&String>,
    after: Option<&String>,
) -> Result<String, String> {
    let before = attributes(before)?;
    let after = attributes(after)?;
    let d = parse(source)?;
    let n = d.root_element();
    let mut edits = vec![];
    let mut added = String::new();
    let mut prefix = n.lookup_prefix(W).unwrap_or("folioW").to_string();
    while n
        .lookup_namespace_uri(Some(&prefix))
        .is_some_and(|ns| ns != W)
    {
        prefix.push('_');
    }
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        if before.get(key) == after.get(key) {
            continue;
        }
        if n.has_tag_name((W, "rFonts")) {
            let theme = format!("{key}Theme");
            if let Some(a) = n
                .attributes()
                .find(|a| a.namespace() == Some(W) && a.name() == theme)
            {
                edits.push((a.range(), String::new()));
            }
        }
        if let Some(a) = n
            .attributes()
            .find(|a| a.namespace() == Some(W) && a.name() == key)
        {
            let name = source[a.range()].split('=').next().unwrap().trim();
            edits.push((
                a.range(),
                after
                    .get(key)
                    .map(|v| format!("{name}=\"{}\"", esc(v)))
                    .unwrap_or_default(),
            ));
        } else if let Some(value) = after.get(key) {
            added.push_str(&format!(" {prefix}:{key}=\"{}\"", esc(value)));
        }
    }
    let mut at = opening_tag_end(source).ok_or("Missing Word property tag")?;
    if source.as_bytes()[at - 1] == b'/' {
        at -= 1;
    }
    if !added.is_empty() {
        if n.lookup_namespace_uri(Some(&prefix)) != Some(W) {
            added.push_str(&format!(" xmlns:{prefix}=\"{W}\""));
        }
        edits.push((at..at, added));
    }
    let merged = replace_ranges(source, edits);
    let d = parse(&merged)?;
    if after.is_empty() && d.root_element().attributes().len() == 0 {
        Ok(String::new())
    } else {
        Ok(merged)
    }
}

/// Preserve properties the projection never represented; change only fields
/// actually changed by the user. `source` must contain current properties only.
pub fn overlay(source: &str, before: &str, after: &str, root: &str) -> Result<String, String> {
    let before = children(before, root)?;
    let after = children(after, root)?;
    let mut result = if source.is_empty() {
        format!("<w:{root} xmlns:w=\"{W}\"/>")
    } else {
        source.into()
    };
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        if equal(before.get(key), after.get(key))? {
            continue;
        }
        if key.ends_with("Change") || matches!(key.as_str(), "ins" | "del" | "moveFrom" | "moveTo")
        {
            return Err("Revision metadata cannot be edited as formatting".into());
        }
        let d = parse(&result)?;
        let old = d
            .root_element()
            .children()
            .find(|n| n.has_tag_name((W, key.as_str())));
        let replacement =
            if matches!(key.as_str(), "rFonts" | "lang" | "spacing" | "ind") && old.is_some() {
                merge_attributes(
                    &scoped_record(&result, old.unwrap()),
                    before.get(key),
                    after.get(key),
                )?
            } else {
                after.get(key).cloned().unwrap_or_default()
            };
        result = set(&result, root, key, &replacement)?;
    }
    Ok(result)
}

pub fn set(source: &str, root: &str, tag: &str, replacement: &str) -> Result<String, String> {
    if root == "pPr" {
        return Ok(crate::docx::set_prop(source, "", tag, replacement.into()));
    }
    let order = [
        "rStyle",
        "rFonts",
        "b",
        "bCs",
        "i",
        "iCs",
        "caps",
        "smallCaps",
        "strike",
        "dstrike",
        "outline",
        "shadow",
        "emboss",
        "imprint",
        "noProof",
        "snapToGrid",
        "vanish",
        "webHidden",
        "color",
        "spacing",
        "w",
        "kern",
        "position",
        "sz",
        "szCs",
        "highlight",
        "u",
        "effect",
        "bdr",
        "shd",
        "fitText",
        "vertAlign",
        "rtl",
        "cs",
        "em",
        "lang",
        "eastAsianLayout",
        "specVanish",
        "oMath",
        "ins",
        "del",
        "moveFrom",
        "moveTo",
        "rPrChange",
    ];
    let rank = |tag: &str| order.iter().position(|x| *x == tag).unwrap_or(usize::MAX);
    let d = parse(source)?;
    let n = d.root_element();
    let mut contents = String::new();
    let mut placed = false;
    for c in n.children() {
        if c.has_tag_name((W, tag)) {
            continue;
        }
        if !placed && c.is_element() && rank(c.tag_name().name()) > rank(tag) {
            contents.push_str(replacement);
            placed = true;
        }
        contents.push_str(raw(source, c));
    }
    if !placed {
        contents.push_str(replacement);
    }
    Ok(replace_inner(source, n, &contents))
}
