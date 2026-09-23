//! Direct and inherited paragraph layout. Values use Word's native twips and
//! 240ths-of-a-line units; unmodeled attributes remain in the source XML.
use crate::office::*;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const INDENT: &[&str] = &[
    "left",
    "right",
    "start",
    "end",
    "firstLine",
    "hanging",
    "leftChars",
    "rightChars",
    "startChars",
    "endChars",
    "firstLineChars",
    "hangingChars",
];
pub const SPACING: &[&str] = &[
    "before",
    "after",
    "beforeLines",
    "afterLines",
    "beforeAutospacing",
    "afterAutospacing",
    "line",
    "lineRule",
];

pub fn read(pr: Option<roxmltree::Node>) -> Map<String, Value> {
    let mut result = Map::new();
    let Some(pr) = pr else {
        return result;
    };
    for (tag, keys) in [("ind", INDENT), ("spacing", SPACING)] {
        if let Some(node) = pr.children().find(|n| n.has_tag_name((W, tag))) {
            for key in keys {
                if let Some(v) = node.attribute((W, *key)) {
                    if *key == "lineRule" {
                        result.insert((*key).into(), json!(v));
                    } else if key.ends_with("Autospacing") {
                        result.insert((*key).into(), json!(matches!(v, "1" | "true" | "on")));
                    } else if let Ok(n) = v.parse::<i64>() {
                        result.insert((*key).into(), json!(n));
                    }
                }
            }
        }
    }
    for tag in [
        "contextualSpacing",
        "bidi",
        "mirrorIndents",
        "snapToGrid",
        "keepNext",
        "keepLines",
        "widowControl",
        "pageBreakBefore",
    ] {
        if let Some(n) = pr.children().find(|n| n.has_tag_name((W, tag))) {
            result.insert(
                tag.into(),
                json!(!matches!(
                    n.attribute((W, "val")),
                    Some("0" | "false" | "off")
                )),
            );
        }
    }
    if let Some(level) = pr
        .children()
        .find(|n| n.has_tag_name((W, "outlineLvl")))
        .and_then(|n| n.attribute((W, "val")))
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v <= 9)
    {
        result.insert("outlineLevel".into(), json!(level));
    }
    result
}

pub fn outline_level(format: &Value) -> Option<u64> {
    format["direct"]["outlineLevel"]
        .as_u64()
        .or_else(|| format["inheritedOutlineLevel"].as_u64())
        .or_else(|| format["inherited"]["outlineLevel"].as_u64())
        .filter(|v| *v <= 9)
}
pub fn set_outline(node: &mut Value, level: Option<u64>) {
    if let Some(level) = level.filter(|n| *n < 9) {
        node["type"] = json!("heading");
        node["attrs"]["level"] = json!(level + 1);
    } else {
        node["type"] = json!("paragraph");
        if let Some(attrs) = node["attrs"].as_object_mut() {
            attrs.remove("level");
        }
    }
}
pub fn normalize_outline(node: &mut Value, before: &Value) {
    if matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
        if let Some(level) = node["attrs"]["paragraphFormat"]["direct"]["outlineLevel"].as_u64() {
            set_outline(node, Some(level));
        } else if before["attrs"]["paragraphFormat"]["direct"]
            .get("outlineLevel")
            .is_some()
            && node["attrs"]["officeId"] == before["attrs"]["officeId"]
        {
            let level = outline_level(&node["attrs"]["paragraphFormat"]);
            set_outline(node, level);
        }
    }
    // Match source identities once per container, rather than scanning all
    // siblings for every paragraph during ordinary typing.
    let mut old_by_id = HashMap::new();
    for child in before["content"].as_array().into_iter().flatten() {
        if let Some(id) = child["attrs"]["officeId"].as_str() {
            old_by_id.entry(id).or_insert(child);
        }
    }
    for (index, child) in node
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let old = child["attrs"]["officeId"]
            .as_str()
            .and_then(|id| old_by_id.get(id).copied())
            .unwrap_or(&before["content"][index]);
        normalize_outline(child, old);
    }
}

pub fn validate(node: &Value) -> Result<(), String> {
    let Some(format) = node["attrs"].get("paragraphFormat") else {
        return Ok(());
    };
    for layer in ["inherited", "direct"] {
        let properties = format[layer]
            .as_object()
            .ok_or("Invalid paragraph formatting")?;
        for (key, value) in properties {
            let valid = if key == "outlineLevel" {
                value.as_u64().is_some_and(|v| v <= 9)
            } else if key == "lineRule" {
                value
                    .as_str()
                    .is_some_and(|v| ["auto", "exact", "atLeast"].contains(&v))
            } else if ["beforeAutospacing", "afterAutospacing"].contains(&key.as_str())
                || [
                    "contextualSpacing",
                    "bidi",
                    "mirrorIndents",
                    "snapToGrid",
                    "keepNext",
                    "keepLines",
                    "widowControl",
                    "pageBreakBefore",
                ]
                .contains(&key.as_str())
            {
                value.is_boolean()
            } else if INDENT.contains(&key.as_str()) || SPACING.contains(&key.as_str()) {
                value.as_i64().is_some_and(|n| {
                    (-2_147_483_647..=2_147_483_647).contains(&n)
                        && (INDENT.contains(&key.as_str()) || n >= 0)
                })
            } else {
                false
            };
            if !valid {
                return Err(format!("Invalid paragraph setting: {key}"));
            }
        }
    }
    if format.get("styleId").is_some_and(|v| !v.is_string()) {
        return Err("Invalid paragraph style identity".into());
    }
    Ok(())
}

pub fn patch(mut props: String, namespaces: &str, node: &Value, before: Option<&Value>) -> String {
    let after = &node["attrs"]["paragraphFormat"]["direct"];
    let old = before
        .map(|b| &b["attrs"]["paragraphFormat"]["direct"])
        .unwrap_or(&Value::Null);
    for (tag, keys) in [("ind", INDENT), ("spacing", SPACING)] {
        let changed = keys
            .iter()
            .filter(|k| after[**k] != old[**k])
            .copied()
            .collect::<Vec<_>>();
        if changed.is_empty() {
            continue;
        }
        let wrapped = format!("<root xmlns:w=\"{W}\"{namespaces}>{props}</root>");
        let Ok(d) = parse(&wrapped) else {
            continue;
        };
        let original = d.descendants().find(|n| n.has_tag_name((W, tag)));
        let mut attrs = String::new();
        let mut inner = String::new();
        if let Some(n) = original {
            for ns in n.namespaces() {
                if ns.name() == Some("w") {
                    continue;
                }
                let name = ns.name().map(|s| format!(":{s}")).unwrap_or_default();
                attrs.push_str(&format!(" xmlns{name}=\"{}\"", esc(ns.uri())));
            }
            for a in n.attributes() {
                if a.namespace() == Some(W) && changed.contains(&a.name()) {
                    continue;
                }
                let prefix = a
                    .namespace()
                    .and_then(|uri| n.lookup_prefix(uri))
                    .map(|p| format!("{p}:"))
                    .unwrap_or_default();
                attrs.push_str(&format!(" {prefix}{}=\"{}\"", a.name(), esc(a.value())));
            }
            inner = n.children().map(|n| raw(&wrapped, n)).collect();
        }
        for key in changed {
            if let Some(value) = after.get(key) {
                let text = if let Some(s) = value.as_str() {
                    s.into()
                } else if let Some(b) = value.as_bool() {
                    if b { "1" } else { "0" }.into()
                } else {
                    value.to_string()
                };
                attrs.push_str(&format!(" w:{key}=\"{}\"", esc(&text)));
            }
        }
        let xml = format!("<w:{tag} xmlns:w=\"{W}\"{attrs}>{inner}</w:{tag}>");
        props = crate::docx::set_prop(&props, namespaces, tag, xml);
    }
    for tag in [
        "contextualSpacing",
        "bidi",
        "mirrorIndents",
        "snapToGrid",
        "keepNext",
        "keepLines",
        "widowControl",
        "pageBreakBefore",
    ] {
        if after[tag] != old[tag] {
            let xml = after[tag]
                .as_bool()
                .map(|b| format!("<w:{tag} w:val=\"{}\"/>", if b { 1 } else { 0 }))
                .unwrap_or_default();
            props = crate::docx::set_prop(&props, namespaces, tag, xml);
        }
    }
    if after["outlineLevel"] != old["outlineLevel"] {
        let xml = after["outlineLevel"]
            .as_u64()
            .map(|level| {
                let wrapped = format!("<root xmlns:w=\"{W}\"{namespaces}>{props}</root>");
                let mut attrs = String::new();
                if let Ok(d) = parse(&wrapped) {
                    if let Some(n) = d.descendants().find(|n| n.has_tag_name((W, "outlineLvl"))) {
                        for ns in n.namespaces().filter(|ns| ns.name() != Some("w")) {
                            let name = ns.name().map(|p| format!(":{p}")).unwrap_or_default();
                            attrs.push_str(&format!(" xmlns{name}=\"{}\"", esc(ns.uri())));
                        }
                        for a in n
                            .attributes()
                            .filter(|a| !(a.namespace() == Some(W) && a.name() == "val"))
                        {
                            let prefix = a
                                .namespace()
                                .and_then(|uri| n.lookup_prefix(uri))
                                .map(|p| format!("{p}:"))
                                .unwrap_or_default();
                            attrs.push_str(&format!(
                                " {prefix}{}=\"{}\"",
                                a.name(),
                                esc(a.value())
                            ));
                        }
                    }
                }
                format!("<w:outlineLvl xmlns:w=\"{W}\"{attrs} w:val=\"{level}\"/>")
            })
            .unwrap_or_default();
        props = crate::docx::set_prop(&props, namespaces, "outlineLvl", xml);
    }
    props
}

pub fn hydrate(document: &mut Value, source: &Value, baseline: &Value) {
    fn collect(n: &Value, values: &mut HashMap<String, Value>) {
        if let (Some(id), Some(p)) = (
            n["attrs"]["officeId"].as_str(),
            n["attrs"].get("paragraphFormat"),
        ) {
            values.insert(id.into(), p.clone());
        }
        for c in n["content"].as_array().into_iter().flatten() {
            collect(c, values);
        }
    }
    fn apply(n: &mut Value, values: &HashMap<String, Value>, baseline: &HashMap<String, Value>) {
        if n["attrs"].get("paragraphFormat").is_none() {
            if let Some(p) = n["attrs"]["officeId"]
                .as_str()
                .and_then(|id| values.get(id))
            {
                n["attrs"]["paragraphFormat"] = p.clone();
            }
        }
        // Older workspaces did not project direct outline settings. Restore
        // only this previously uneditable source property, never other edits.
        if let Some(level) = n["attrs"]["officeId"]
            .as_str()
            .filter(|id| {
                baseline
                    .get(*id)
                    .is_some_and(|p| p["direct"].get("outlineLevel").is_none())
            })
            .and_then(|id| values.get(id))
            .and_then(|p| p["direct"]["outlineLevel"].as_u64())
        {
            if n["attrs"]["paragraphFormat"]["direct"]
                .get("outlineLevel")
                .is_none()
            {
                n["attrs"]["paragraphFormat"]["direct"]["outlineLevel"] = json!(level);
                set_outline(n, Some(level));
            }
        }
        for c in n
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            apply(c, values, baseline);
        }
    }
    let mut values = HashMap::new();
    collect(source, &mut values);
    let mut old = HashMap::new();
    collect(baseline, &mut old);
    apply(document, &values, &old);
}
