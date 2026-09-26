//! Materialize the application's heading defaults only when used. Existing
//! document declarations win; merely opening Styles never changes a document.
use crate::{docx_styles as styles, office::Package};
use serde_json::{json, Value};
use std::collections::HashSet;

fn matching<'a>(catalog: &'a Value, id: &str, name: &str) -> Option<&'a Value> {
    let all = catalog["styles"].as_array()?;
    all.iter()
        .find(|s| {
            s["type"] == "paragraph" && s["id"].as_str().is_some_and(|v| v.eq_ignore_ascii_case(id))
        })
        .or_else(|| {
            all.iter().find(|s| {
                s["type"] == "paragraph"
                    && s["name"]
                        .as_str()
                        .is_some_and(|v| v.eq_ignore_ascii_case(name))
            })
        })
}
pub fn heading(catalog: &Value, level: u64) -> Option<&Value> {
    matching(
        catalog,
        &format!("Heading{level}"),
        &format!("Heading {level}"),
    )
}
fn unique(used: &mut HashSet<String>, preferred: &str) -> String {
    let mut id = preferred.to_string();
    let mut suffix = 0;
    while !used.insert(id.to_ascii_lowercase()) {
        suffix += 1;
        id = format!("FolioBuiltin{preferred}{suffix}");
    }
    id
}
pub fn available(catalog: &Value) -> Vec<Value> {
    let mut used = catalog["styles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["id"].as_str().map(str::to_ascii_lowercase))
        .collect::<HashSet<_>>();
    let mut added = vec![];
    let body = catalog["defaultStyle"]
        .as_str()
        .and_then(|id| styles::style(catalog, id))
        .filter(|s| s["type"] == "paragraph")
        .or_else(|| matching(catalog, "Normal", "Normal"));
    let body_id = if let Some(body) = body {
        body["id"].as_str().unwrap().to_string()
    } else {
        let id = unique(&mut used, "Normal");
        // Preserve a document with no default declaration. Explicitly applying
        // this Normal uses its existing document defaults without changing them.
        added.push(
            json!({"id":id,"name":"Normal","type":"paragraph","customStyle":false,
            "properties":{"paragraph":{},"tabs":[],"run":{}}}),
        );
        id
    };
    for (index, (size, before, after, bold, italic)) in [
        (48, 0, 270, false, false),
        (36, 270, 180, false, false),
        (29, 210, 180, false, false),
        (30, 120, 0, true, false),
        (27, 120, 0, true, false),
        (24, 120, 0, true, false),
        (24, 120, 0, true, false),
        (24, 120, 0, false, true),
        (24, 120, 0, false, true),
    ]
    .into_iter()
    .enumerate()
    {
        let level = index as u64 + 1;
        if heading(catalog, level).is_some() {
            continue;
        }
        let mut run = json!({"sz":size});
        if bold {
            run["bold"] = json!(true);
        }
        if italic {
            run["italic"] = json!(true);
        }
        added.push(json!({"id":unique(&mut used,&format!("Heading{level}")),"name":format!("Heading {level}"),
            "type":"paragraph","customStyle":false,"basedOn":body_id,"next":body_id,
            "properties":{"outlineLevel":index,"paragraph":{"before":before,"after":after},"tabs":[],"run":run}}));
    }
    // Captions and table-of-contents levels (W10.03, W10.05), in Word's
    // names so Word and Folio treat them as the built-in styles.
    if matching(catalog, "Caption", "caption").is_none() {
        added.push(
            json!({"id":unique(&mut used,"Caption"),"name":"caption","type":"paragraph",
            "customStyle":false,"basedOn":body_id,"next":body_id,
            "properties":{"paragraph":{"after":200},"tabs":[],
                "run":{"italic":true,"sz":18,"color":{"val":"0E2841"}}}}),
        );
    }
    for level in 1..=3u64 {
        if matching(catalog, &format!("TOC{level}"), &format!("toc {level}")).is_some() {
            continue;
        }
        added.push(
            json!({"id":unique(&mut used,&format!("TOC{level}")),"name":format!("toc {level}"),
            "type":"paragraph","customStyle":false,"basedOn":body_id,"next":body_id,
            "properties":{"paragraph":{"after":100,"left":(level - 1) * 220},
                "tabs":[{"position":9350,"alignment":"right","leader":"dot"}],"run":{}}}),
        );
    }
    added
}
pub fn expanded(catalog: &Value) -> Value {
    let mut result = catalog.clone();
    result["styles"]
        .as_array_mut()
        .unwrap()
        .extend(available(catalog));
    result
}
/// Add the chosen definition and any missing base/following definitions, not the
/// entire gallery. Returns false when the identity already exists.
pub fn materialize(catalog: &mut Value, id: &str) -> bool {
    let candidates = available(catalog);
    let mut wanted = vec![id.to_string()];
    let mut added = false;
    while let Some(id) = wanted.pop() {
        if styles::style(catalog, &id).is_some() {
            continue;
        }
        let Some(s) = candidates.iter().find(|s| s["id"] == id) else {
            continue;
        };
        for key in ["basedOn", "next"] {
            if let Some(id) = s[key].as_str() {
                wanted.push(id.to_string());
            }
        }
        catalog["styles"].as_array_mut().unwrap().push(s.clone());
        added = true;
    }
    added
}
/// Most edits need no catalog work. Avoid cloning a preserved Office package
/// on the normal typing path; inspect only references to missing definitions.
pub fn needed(document: &Value, catalog: &Value) -> bool {
    let candidates = available(catalog);
    fn walk(node: &Value, candidates: &[Value]) -> bool {
        if node["type"] == "tableOfContents"
            && node["attrs"]["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|e| {
                    let id = format!("TOC{}", e["level"].as_u64().unwrap_or(1));
                    candidates.iter().any(|s| s["id"] == id.as_str())
                })
        {
            return true;
        }
        if matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
            if let Some(id) = node["attrs"]["paragraphFormat"]["styleId"].as_str() {
                if candidates.iter().any(|s| s["id"] == id) {
                    return true;
                }
            } else if node["type"] == "heading" {
                if let Some(level) = node["attrs"]["level"].as_u64() {
                    if candidates.iter().any(|s| {
                        s["properties"]["outlineLevel"]
                            .as_u64()
                            .is_some_and(|n| n + 1 == level)
                    }) {
                        return true;
                    }
                }
            }
        }
        node["content"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| walk(c, candidates))
    }
    walk(document, &candidates)
}
/// Ordinary typing keeps the protected catalog unchanged. A new canonical
/// heading reference can request a built-in definition owned by the engine.
pub fn materialize_document(document: &mut Value, pkg: &mut Package) -> Result<(), String> {
    let catalog = document["attrs"]
        .get(styles::KEY)
        .cloned()
        .unwrap_or(styles::read(pkg)?);
    let expanded = expanded(&catalog);
    fn collect(node: &mut Value, catalog: &Value, wanted: &mut HashSet<String>) {
        // A table of contents uses a TOC style for each level it shows.
        if node["type"] == "tableOfContents" {
            for e in node["attrs"]["entries"].as_array().into_iter().flatten() {
                wanted.insert(format!("TOC{}", e["level"].as_u64().unwrap_or(1)));
            }
        }
        if matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
            if let Some(id) = node["attrs"]["paragraphFormat"]["styleId"].as_str() {
                wanted.insert(id.into());
            } else if node["type"] == "heading" {
                if let Some(s) = node["attrs"]["level"]
                    .as_u64()
                    .and_then(|n| heading(catalog, n))
                {
                    wanted.insert(s["id"].as_str().unwrap().into());
                }
            }
        }
        for c in node
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            collect(c, catalog, wanted);
        }
    }
    let mut wanted = HashSet::new();
    collect(document, &expanded, &mut wanted);
    let mut next = catalog.clone();
    let mut changed = false;
    // Deterministic declaration ordering across platforms and repeated saves.
    let mut ids = wanted.into_iter().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        changed |= materialize(&mut next, &id);
    }
    if !changed {
        return Ok(());
    }
    fn bind(node: &mut Value, catalog: &Value) {
        if node["type"] == "heading" && node["attrs"]["paragraphFormat"]["styleId"].is_null() {
            if let Some(s) = node["attrs"]["level"]
                .as_u64()
                .and_then(|n| heading(catalog, n))
            {
                node["attrs"]["paragraphFormat"]["styleId"] = s["id"].clone();
            }
        }
        for c in node
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            bind(c, catalog);
        }
    }
    bind(document, &next);
    document["attrs"][styles::KEY] = next;
    styles::write(document, pkg)?;
    crate::docx_tabs::Catalog::read(pkg)?.refresh(document)?;
    Ok(())
}
