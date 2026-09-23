//! Paragraph tab declarations retain their style inheritance separately from
//! direct formatting. Positions and the document default are stored in twips.
use crate::office::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn wc<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|n| n.has_tag_name((W, name)))
}
fn value<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<&'a str> {
    wc(n, name).and_then(|n| n.attribute((W, "val")))
}
pub(crate) fn part(pkg: &Package, kind: &str) -> Result<String, String> {
    if let Ok(xml) = pkg.text("word/_rels/document.xml.rels") {
        let d = parse(xml)?;
        if let Some(n) = d.root_element().children().find(|n| {
            attr(*n, "Type") == Some(&format!("{R}/{kind}"))
                && attr(*n, "TargetMode") != Some("External")
        }) {
            let target = attr(n, "Target").ok_or("Office relationship target missing")?;
            let full = if target.starts_with('/') {
                target.into()
            } else {
                format!("word/{target}")
            };
            let mut parts = vec![];
            for p in full.split('/') {
                match p {
                    "" | "." => {}
                    ".." => {
                        parts.pop();
                    }
                    _ => parts.push(p),
                }
            }
            return Ok(parts.join("/"));
        }
    }
    Ok(format!("word/{kind}.xml"))
}

#[derive(Clone, Default)]
struct Props {
    tabs: Vec<Value>,
    layout: serde_json::Map<String, Value>,
}
fn props(n: Option<roxmltree::Node>) -> Props {
    let tabs = n.and_then(|n| wc(n, "tabs"));
    Props {
        layout: crate::docx_paragraph::read(n),
        tabs: tabs
            .into_iter()
            .flat_map(|n| n.children())
            .filter(|n| n.has_tag_name((W, "tab")))
            .filter_map(|n| {
                let pos = n.attribute((W, "pos"))?.parse::<i32>().ok()?;
                Some(
                    json!({"position":pos, "alignment":n.attribute((W,"val")).unwrap_or("left"),
                "leader":n.attribute((W,"leader")).unwrap_or("none")}),
                )
            })
            .collect(),
    }
}
fn merge(stops: &mut BTreeMap<i64, Value>, additions: &[Value]) {
    for stop in additions {
        let Some(p) = stop["position"].as_i64() else {
            continue;
        };
        if stop["alignment"] == "clear" {
            stops.remove(&p);
        } else {
            stops.insert(p, stop.clone());
        }
    }
}

#[derive(Default)]
pub struct Catalog {
    pub table_rows: crate::docx_table_format::RowStyles,
    pub default: Option<u32>,
    named_styles: Value,
    defaults: Props,
    styles: HashMap<String, (Option<String>, Props)>,
    default_style: Option<String>,
    numbering: HashMap<(String, usize), Props>,
}
impl Catalog {
    pub fn read(pkg: &Package) -> Result<Self, String> {
        let mut this = Self {
            table_rows: crate::docx_table_format::RowStyles::read(pkg)?,
            named_styles: crate::docx_styles::read(pkg)?,
            ..Self::default()
        };
        if let Ok(xml) = pkg.text(&part(pkg, "settings")?) {
            let d = parse(xml)?;
            this.default = value(d.root_element(), "defaultTabStop")
                .and_then(|s| s.parse().ok())
                .filter(|n| *n > 0);
        }
        if let Ok(xml) = pkg.text(&part(pkg, "styles")?) {
            let d = parse(xml)?;
            this.defaults = props(
                wc(d.root_element(), "docDefaults")
                    .and_then(|n| wc(n, "pPrDefault"))
                    .and_then(|n| wc(n, "pPr")),
            );
            for n in d
                .root_element()
                .children()
                .filter(|n| n.has_tag_name((W, "style")))
            {
                if n.attribute((W, "type")) != Some("paragraph") {
                    continue;
                }
                let Some(id) = n.attribute((W, "styleId")) else {
                    continue;
                };
                this.styles.insert(
                    id.into(),
                    (value(n, "basedOn").map(String::from), props(wc(n, "pPr"))),
                );
                if matches!(n.attribute((W, "default")), Some("1" | "true" | "on")) {
                    this.default_style = Some(id.into());
                }
            }
        }
        if let Ok(xml) = pkg.text(&part(pkg, "numbering")?) {
            let d = parse(xml)?;
            let abstracts: HashMap<_, _> = d
                .root_element()
                .children()
                .filter(|n| n.has_tag_name((W, "abstractNum")))
                .filter_map(|n| n.attribute((W, "abstractNumId")).map(|id| (id, n)))
                .collect();
            for n in d
                .root_element()
                .children()
                .filter(|n| n.has_tag_name((W, "num")))
            {
                let Some(id) = n.attribute((W, "numId")) else {
                    continue;
                };
                let Some(base) = value(n, "abstractNumId").and_then(|id| abstracts.get(id)) else {
                    continue;
                };
                for l in base.children().filter(|n| n.has_tag_name((W, "lvl"))) {
                    if let Some(i) = l
                        .attribute((W, "ilvl"))
                        .and_then(|s| s.parse::<usize>().ok())
                    {
                        this.numbering.insert((id.into(), i), props(wc(l, "pPr")));
                    }
                }
                for o in n.children().filter(|n| n.has_tag_name((W, "lvlOverride"))) {
                    if let (Some(i), Some(l)) = (
                        o.attribute((W, "ilvl"))
                            .and_then(|s| s.parse::<usize>().ok()),
                        wc(o, "lvl"),
                    ) {
                        this.numbering.insert((id.into(), i), props(wc(l, "pPr")));
                    }
                }
            }
        }
        Ok(this)
    }
    pub fn refresh(&self, document: &mut Value) -> Result<(), String> {
        fn walk(c: &Catalog, node: &mut Value) -> Result<(), String> {
            if matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
                let style = node["attrs"]["paragraphFormat"]["styleId"]
                    .as_str()
                    .map(String::from)
                    .or_else(|| {
                        (node["type"] == "heading").then(|| {
                            format!("Heading{}", node["attrs"]["level"].as_u64().unwrap_or(1))
                        })
                    });
                let xml = format!(
                    "<w:p xmlns:w=\"{W}\"><w:pPr>{}</w:pPr></w:p>",
                    style
                        .as_ref()
                        .map(|id| format!("<w:pStyle w:val=\"{}\"/>", esc(id)))
                        .unwrap_or_default()
                );
                let dom = parse(&xml)?;
                let direct = node["attrs"]["paragraphFormat"]["direct"]
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                let mut format = c
                    .paragraph_format(dom.root_element(), &node["attrs"]["numbering"])
                    .unwrap();
                format["direct"] = json!(direct);
                let old_outline =
                    crate::docx_paragraph::outline_level(&node["attrs"]["paragraphFormat"]);
                let outline = crate::docx_paragraph::outline_level(&format);
                if outline.is_some() || old_outline.is_some() {
                    crate::docx_paragraph::set_outline(node, outline);
                }
                node["attrs"]["paragraphFormat"] = format;
                let old = node["attrs"]["tabs"]["direct"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                if let Some(mut tabs) = c.paragraph(dom.root_element(), &node["attrs"]["numbering"])
                {
                    tabs["direct"] = json!(old);
                    node["attrs"]["tabs"] = tabs;
                } else if !old.is_empty() || node["attrs"].get("tabs").is_some() {
                    node["attrs"]["tabs"] = json!({"inherited":[],"direct":old});
                }
            }
            for child in node
                .get_mut("content")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                walk(c, child)?;
            }
            Ok(())
        }
        walk(self, document)
    }
    pub fn paragraph_format(&self, p: roxmltree::Node, numbering: &Value) -> Option<Value> {
        let pr = wc(p, "pPr");
        let mut inherited = serde_json::Map::from_iter([
            (String::from("before"), json!(0)),
            (String::from("after"), json!(0)),
            (String::from("line"), json!(240)),
            (String::from("lineRule"), json!("auto")),
        ]);
        inherited.extend(self.defaults.layout.clone());
        if let (Some(id), Some(level)) = (numbering["numId"].as_str(), numbering["level"].as_u64())
        {
            if let Some(n) = self.numbering.get(&(id.into(), level as usize)) {
                inherited.extend(n.layout.clone());
            }
        }
        let style = pr
            .and_then(|n| value(n, "pStyle"))
            .map(String::from)
            .or_else(|| self.default_style.clone());
        let mut current = style.clone();
        let mut seen = HashSet::new();
        let mut chain = vec![];
        while let Some(id) = current.filter(|id| seen.insert(id.clone())) {
            let Some((base, properties)) = self.styles.get(&id) else {
                break;
            };
            chain.push(properties);
            current = base.clone();
        }
        for p in chain.into_iter().rev() {
            inherited.extend(p.layout.clone());
        }
        let direct = crate::docx_paragraph::read(pr);
        if inherited.is_empty() && direct.is_empty() {
            return None;
        }
        let mut result = json!({"inherited":inherited,"direct":direct});
        if let Some(style) = style {
            result["styleId"] = json!(style);
        }
        let resolved = crate::docx_styles::resolve(&self.named_styles, result["styleId"].as_str());
        result["inheritedRun"] = resolved["run"].clone();
        for key in ["alignment", "outlineLevel"] {
            if let Some(v) = resolved.get(key) {
                result[format!("inherited{}{}", key[..1].to_uppercase(), &key[1..])] = v.clone();
            }
        }
        Some(result)
    }
    pub fn paragraph(&self, p: roxmltree::Node, numbering: &Value) -> Option<Value> {
        let pr = wc(p, "pPr");
        let mut inherited = BTreeMap::new();
        merge(&mut inherited, &self.defaults.tabs);
        if let (Some(id), Some(level)) = (numbering["numId"].as_str(), numbering["level"].as_u64())
        {
            if let Some(n) = self.numbering.get(&(id.into(), level as usize)) {
                merge(&mut inherited, &n.tabs);
            }
        }
        let mut current = pr
            .and_then(|n| value(n, "pStyle"))
            .map(String::from)
            .or_else(|| self.default_style.clone());
        let mut seen = HashSet::new();
        let mut chain = vec![];
        while let Some(id) = current.filter(|id| seen.insert(id.clone())) {
            let Some((base, p)) = self.styles.get(&id) else {
                break;
            };
            chain.push(p);
            current = base.clone();
        }
        for p in chain.into_iter().rev() {
            merge(&mut inherited, &p.tabs);
        }
        let direct = props(pr).tabs;
        if inherited.is_empty() && direct.is_empty() {
            return None;
        }
        Some(json!({"inherited":inherited.into_values().collect::<Vec<_>>(),"direct":direct}))
    }
}

pub fn xml(tabs: &Value) -> String {
    let mut s = String::new();
    for t in tabs["direct"].as_array().into_iter().flatten() {
        if let Some(p) = t["position"].as_i64() {
            s.push_str(&format!(
                "<w:tab w:val=\"{}\" w:pos=\"{p}\" w:leader=\"{}\"/>",
                esc(t["alignment"].as_str().unwrap_or("left")),
                esc(t["leader"].as_str().unwrap_or("none"))
            ));
        }
    }
    if s.is_empty() {
        s
    } else {
        format!("<w:tabs>{s}</w:tabs>")
    }
}

pub fn write_default(document: &Value, pkg: &mut Package) -> Result<(), String> {
    let Some(default) = document["attrs"]["defaultTabStop"].as_u64() else {
        return Ok(());
    };
    let path = part(pkg, "settings")?;
    let xml = pkg
        .text(&path)
        .map(String::from)
        .unwrap_or(format!("<w:settings xmlns:w=\"{W}\"/>"));
    let d = parse(&xml)?;
    let root = d.root_element();
    if value(root, "defaultTabStop").and_then(|s| s.parse::<u64>().ok()) == Some(default) {
        return Ok(());
    }
    let property = format!("<w:defaultTabStop xmlns:w=\"{W}\" w:val=\"{default}\"/>");
    let result = if let Some(old) = wc(root, "defaultTabStop") {
        replace_ranges(&xml, vec![(old.range(), property)])
    } else {
        // Insert before the following CT_Settings schema elements. Unknown
        // extensions stay in their original order and all other bytes survive.
        let preceding = [
            "writeProtection",
            "view",
            "zoom",
            "removePersonalInformation",
            "removeDateAndTime",
            "doNotDisplayPageBoundaries",
            "displayBackgroundShape",
            "printPostScriptOverText",
            "printFractionalCharacterWidth",
            "printFormsData",
            "embedTrueTypeFonts",
            "embedSystemFonts",
            "saveSubsetFonts",
            "saveFormsData",
            "mirrorMargins",
            "alignBordersAndEdges",
            "bordersDoNotSurroundHeader",
            "bordersDoNotSurroundFooter",
            "gutterAtTop",
            "hideSpellingErrors",
            "hideGrammaticalErrors",
            "activeWritingStyle",
            "proofState",
            "formsDesign",
            "attachedTemplate",
            "linkStyles",
            "stylePaneFormatFilter",
            "stylePaneSortMethod",
            "documentType",
            "mailMerge",
            "revisionView",
            "trackRevisions",
            "doNotTrackMoves",
            "doNotTrackFormatting",
            "documentProtection",
            "autoFormatOverride",
            "styleLockTheme",
            "styleLockQFSet",
        ];
        let mut inner = String::new();
        let mut inserted = false;
        for n in root.children() {
            if !inserted && n.is_element() && !preceding.contains(&n.tag_name().name()) {
                inner.push_str(&property);
                inserted = true;
            }
            inner.push_str(raw(&xml, n));
        }
        if !inserted {
            inner.push_str(&property);
        }
        replace_inner(&xml, root, &inner)
    };
    pkg.set(&path, result);
    ensure_settings(pkg)?;
    Ok(())
}

/// Ensure a relationship-selected settings part without changing its contents.
pub(crate) fn ensure_settings(pkg: &mut Package) -> Result<String, String> {
    let path = part(pkg, "settings")?;
    if !pkg.entries.contains_key(&path) {
        pkg.set(&path, format!("<w:settings xmlns:w=\"{W}\"/>"));
    }
    let rels = pkg
        .text("word/_rels/document.xml.rels")
        .map(String::from)
        .unwrap_or(format!("<Relationships xmlns=\"{REL_NS}\"/>"));
    let d = parse(&rels)?;
    let root = d.root_element();
    if !root.children().any(|n| {
        attr(n, "Type") == Some(&format!("{R}/settings"))
            && attr(n, "TargetMode") != Some("External")
    }) {
        let mut i = 1;
        while root
            .children()
            .any(|n| attr(n, "Id") == Some(&format!("folioSettings{i}")))
        {
            i += 1;
        }
        let inner = root.children().map(|n| raw(&rels, n)).collect::<String>();
        pkg.set("word/_rels/document.xml.rels",replace_inner(&rels,root,&format!("{inner}<Relationship xmlns=\"{REL_NS}\" Id=\"folioSettings{i}\" Type=\"{R}/settings\" Target=\"/{}\"/>",esc(&path))));
    }
    let ct = pkg.text("[Content_Types].xml")?.to_string();
    let d = parse(&ct)?;
    let root = d.root_element();
    if !root
        .children()
        .any(|n| attr(n, "PartName") == Some(&format!("/{path}")))
    {
        let inner = root.children().map(|n| raw(&ct, n)).collect::<String>();
        pkg.set("[Content_Types].xml",replace_inner(&ct,root,&format!("{inner}<Override xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\" PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/>",esc(&path))));
    }
    Ok(path)
}

pub fn validate_node(node: &Value) -> Result<(), String> {
    if let Some(v) = node["attrs"].get("defaultTabStop") {
        if v.as_u64()
            .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            .is_none()
        {
            return Err("Invalid default tab spacing".into());
        }
    }
    if let Some(tabs) = node["attrs"].get("tabs") {
        if !tabs.is_object() {
            return Err("Invalid paragraph tabs".into());
        }
        for key in ["direct", "inherited"] {
            let ts = tabs[key]
                .as_array()
                .filter(|ts| ts.len() <= 1000)
                .ok_or("Invalid tab stops")?;
            for t in ts {
                if t["position"]
                    .as_i64()
                    .filter(|n| i32::try_from(*n).is_ok())
                    .is_none()
                    || !t["alignment"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 40)
                    || !t["leader"].as_str().is_some_and(|s| s.len() <= 40)
                {
                    return Err("Invalid tab stop".into());
                }
            }
        }
    }
    Ok(())
}

pub fn hydrate(document: &mut Value, source: &Value) {
    fn collect(n: &Value, out: &mut HashMap<String, Value>) {
        if let (Some(id), Some(t)) = (n["attrs"]["officeId"].as_str(), n["attrs"].get("tabs")) {
            out.insert(id.into(), t.clone());
        }
        for c in n["content"].as_array().into_iter().flatten() {
            collect(c, out);
        }
    }
    fn apply(n: &mut Value, source: &HashMap<String, Value>) {
        if n["attrs"].get("tabs").is_none() {
            if let Some(t) = n["attrs"]["officeId"]
                .as_str()
                .and_then(|id| source.get(id))
            {
                n["attrs"]["tabs"] = t.clone();
            }
        }
        if let Some(cs) = n.get_mut("content").and_then(Value::as_array_mut) {
            for c in cs {
                apply(c, source);
            }
        }
    }
    if document["attrs"].get("defaultTabStop").is_none() {
        if let Some(v) = source["attrs"].get("defaultTabStop") {
            document["attrs"]["defaultTabStop"] = v.clone();
        }
    }
    let mut lookup = HashMap::new();
    collect(source, &mut lookup);
    apply(document, &lookup);
}
