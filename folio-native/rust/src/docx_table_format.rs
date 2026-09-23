//! Direct table formatting. Unedited or unsupported source properties stay raw.
use crate::{docx, office::*};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const TABLE_KEYS: &[&str] = &["layout", "alignment"];
pub const ROW_KEYS: &[&str] = &["height", "header", "cantSplit", "inheritedCantSplit"];
pub const CELL_KEYS: &[&str] = &["fill", "verticalAlign"];

/// Keep style defaults separate from direct row edits. A false direct value
/// must override an inherited true value without rewriting the table style.
#[derive(Default)]
pub struct RowStyles {
    styles: HashMap<String, String>,
    default: Option<String>,
}
impl RowStyles {
    pub fn read(pkg: &Package) -> Result<Self, String> {
        let mut out = Self::default();
        if let Ok(xml) = pkg.text(&crate::docx_tabs::part(pkg, "styles")?) {
            let d = parse(xml)?;
            for n in d.root_element().children().filter(|n| {
                n.has_tag_name((W, "style")) && n.attribute((W, "type")) == Some("table")
            }) {
                if let Some(id) = n.attribute((W, "styleId")) {
                    out.styles.insert(id.into(), scoped_record(xml, n));
                    if matches!(n.attribute((W, "default")), Some("1" | "on" | "true")) {
                        out.default = Some(id.into());
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn apply(
        &self,
        table: roxmltree::Node,
        row: roxmltree::Node,
        index: usize,
        value: &mut Value,
    ) {
        let pr = child(table, "tblPr");
        let mut id = pr
            .and_then(|n| child(n, "tblStyle"))
            .and_then(|n| attr(n, "val"))
            .or(self.default.as_deref())
            .map(String::from);
        let mut seen = HashSet::new();
        let mut chain = vec![];
        while let Some(key) = id {
            if !seen.insert(key.clone()) {
                break;
            }
            let Some(xml) = self.styles.get(&key) else {
                break;
            };
            let Ok(d) = parse(xml) else {
                break;
            };
            id = child(d.root_element(), "basedOn")
                .and_then(|n| attr(n, "val"))
                .map(String::from);
            chain.push(d);
        }
        let mut base = None;
        let mut conditions = HashMap::new();
        let mut band_size = 1;
        let mut look_bits = 0u16;
        let mut look_values = HashMap::new();
        let mut read_look = |p: roxmltree::Node| {
            if let Some(n) = child(p, "tblLook") {
                if let Some(v) = attr(n, "val").and_then(|s| u16::from_str_radix(s, 16).ok()) {
                    look_bits = v;
                }
                for key in ["firstRow", "lastRow", "noHBand"] {
                    if let Some(v) = attr(n, key) {
                        look_values.insert(key, !matches!(v, "0" | "off" | "false"));
                    }
                }
            }
            if let Some(n) = child(p, "tblStyleRowBandSize")
                .and_then(|n| attr(n, "val"))
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|n| *n > 0)
            {
                band_size = n;
            }
        };
        let flag = |p: roxmltree::Node| {
            child(p, "trPr")
                .and_then(|n| child(n, "cantSplit"))
                .map(|n| !matches!(attr(n, "val"), Some("0" | "off" | "false")))
        };
        for d in chain.iter().rev() {
            let n = d.root_element();
            base = flag(n).or(base);
            if let Some(p) = child(n, "tblPr") {
                read_look(p);
            }
            for c in n.children().filter(|n| n.has_tag_name((W, "tblStylePr"))) {
                if let (Some(kind), Some(v)) = (attr(c, "type"), flag(c)) {
                    conditions.insert(kind, v);
                }
            }
        }
        if let Some(p) = pr {
            read_look(p);
        }
        let enabled = |name, bit| {
            look_values
                .get(name)
                .copied()
                .unwrap_or(look_bits & bit != 0)
        };
        let first = enabled("firstRow", 0x20);
        let last = enabled("lastRow", 0x40);
        let no_band = enabled("noHBand", 0x200);
        let count = table
            .children()
            .filter(|n| n.has_tag_name((W, "tr")))
            .count();
        let cnf = child(row, "trPr").and_then(|n| child(n, "cnfStyle"));
        let active = |name, bit: usize, fallback| {
            cnf.and_then(|n| {
                attr(n, name)
                    .map(|v| !matches!(v, "0" | "off" | "false"))
                    .or_else(|| {
                        attr(n, "val")
                            .filter(|s| s.len() == 12)
                            .map(|s| s.as_bytes()[bit] == b'1')
                    })
            })
            .unwrap_or(fallback)
        };
        let band_row = index.saturating_sub(usize::from(first));
        let odd = (band_row / band_size) % 2 == 0;
        for (kind, on) in [
            ("wholeTable", true),
            ("band1Horz", active("oddHBand", 6, !no_band && odd)),
            ("band2Horz", active("evenHBand", 7, !no_band && !odd)),
            ("firstRow", active("firstRow", 0, first && index == 0)),
            ("lastRow", active("lastRow", 1, last && index + 1 == count)),
        ] {
            if on {
                base = conditions.get(kind).copied().or(base);
            }
        }
        if let Some(v) = base {
            value["attrs"]["inheritedCantSplit"] = json!(v);
        }
        // Cell/corner conditional row properties require a cell-aware style
        // cascade. Preserve them raw and report the missing preview behavior.
        if conditions.keys().any(|k| {
            ![
                "wholeTable",
                "band1Horz",
                "band2Horz",
                "firstRow",
                "lastRow",
            ]
            .contains(k)
        }) {
            value["attrs"]["rowBreakStyleUnsupported"] = json!(true);
        }
    }
}

fn choice(v: &Value, key: &str, options: &[&str]) -> Result<(), String> {
    if let Some(value) = v["attrs"].get(key) {
        if !value.as_str().is_some_and(|s| options.contains(&s)) {
            return Err(format!("Invalid table {key}"));
        }
    }
    Ok(())
}
fn color(s: &str) -> bool {
    s == "auto" || s.len() == 6 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn validate(v: &Value) -> Result<(), String> {
    match v["type"].as_str().unwrap_or("") {
        "table" => {
            choice(v, "layout", &["fixed", "autofit"])?;
            choice(v, "alignment", &["left", "center", "right"])?;
        }
        "tableRow" => {
            for key in ["cantSplit", "inheritedCantSplit"] {
                if v["attrs"].get(key).is_some_and(|h| !h.is_boolean()) {
                    return Err("Row page splitting must be true or false".into());
                }
            }
            if v["attrs"].get("header").is_some_and(|h| !h.is_boolean()) {
                return Err("Repeat header row must be true or false".into());
            }
            if let Some(h) = v["attrs"].get("height") {
                if !h["rule"]
                    .as_str()
                    .is_some_and(|s| ["auto", "atLeast", "exact"].contains(&s))
                    || !h["value"]
                        .as_u64()
                        .is_some_and(|n| n <= 31680 && (h["rule"] == "auto" || n > 0))
                {
                    return Err("Invalid table row height".into());
                }
            }
        }
        "tableCell" | "tableHeader" => {
            choice(v, "verticalAlign", &["top", "center", "bottom"])?;
            if let Some(fill) = v["attrs"].get("fill") {
                if !fill.as_str().is_some_and(color) {
                    return Err("Invalid table cell fill".into());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn read(node: roxmltree::Node, value: &mut Value) {
    let (kind, property) = match node.tag_name().name() {
        "tbl" => (0, "tblPr"),
        "tr" => (1, "trPr"),
        "tc" => (2, "tcPr"),
        _ => return,
    };
    let Some(pr) = child(node, property) else {
        return;
    };
    let a = &mut value["attrs"];
    if kind == 0 {
        for (key, tag, field, choices) in [
            ("layout", "tblLayout", "type", &["fixed", "autofit"][..]),
            ("alignment", "jc", "val", &["left", "center", "right"][..]),
        ] {
            if let Some(s) = child(pr, tag)
                .and_then(|n| attr(n, field))
                .filter(|s| choices.contains(s))
            {
                a[key] = json!(s);
            }
        }
    } else if kind == 1 {
        if let Some(h) = child(pr, "cantSplit") {
            a["cantSplit"] = json!(!matches!(attr(h, "val"), Some("false" | "0" | "off")));
        }
        if let Some(h) = child(pr, "tblHeader") {
            a["header"] = json!(!matches!(attr(h, "val"), Some("false" | "0" | "off")));
        }
        if let Some(h) = child(pr, "trHeight") {
            let rule = attr(h, "hRule").unwrap_or("atLeast");
            let n = attr(h, "val")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if ["auto", "atLeast", "exact"].contains(&rule)
                && n <= 31680
                && (rule == "auto" || n > 0)
            {
                a["height"] = json!({"rule":rule,"value":n});
            }
        }
    } else {
        if let Some(s) = child(pr, "vAlign")
            .and_then(|n| attr(n, "val"))
            .filter(|s| ["top", "center", "bottom"].contains(s))
        {
            a["verticalAlign"] = json!(s);
        }
        if let Some(shd) = child(pr, "shd") {
            if attr(shd, "themeFill").is_none() && matches!(attr(shd, "val"), None | Some("clear"))
            {
                if let Some(s) = attr(shd, "fill").filter(|s| color(s)) {
                    a["fill"] = json!(s);
                }
            }
        }
    }
}

pub fn hydrate(target: &mut Value, source: &Value, keys: &[&str]) {
    for key in keys {
        if target["attrs"].get(*key).is_none() {
            if let Some(v) = source["attrs"].get(*key) {
                if !target["attrs"].is_object() {
                    target["attrs"] = json!({});
                }
                target["attrs"][*key] = v.clone();
            }
        }
    }
}

pub fn wrap_properties(kind: &str, original: Option<roxmltree::Node>, body: String) -> String {
    let mut attributes = String::new();
    if let Some(pr) = original {
        for ns in pr.namespaces().filter(|n| n.name() != Some("w")) {
            let name = ns.name().map(|n| format!(":{n}")).unwrap_or_default();
            attributes.push_str(&format!(" xmlns{name}=\"{}\"", esc(ns.uri())));
        }
        for a in pr.attributes() {
            let prefix = a
                .namespace()
                .and_then(|uri| pr.lookup_prefix(uri))
                .map(|p| format!("{p}:"))
                .unwrap_or_default();
            attributes.push_str(&format!(" {prefix}{}=\"{}\"", a.name(), esc(a.value())));
        }
    }
    format!("<w:{kind} xmlns:w=\"{W}\"{attributes}>{body}</w:{kind}>")
}

pub fn patch(
    props: String,
    namespaces: &str,
    kind: &str,
    current: &Value,
    before: Option<&Value>,
    width: Option<u64>,
) -> String {
    let changed = |key: &str| {
        current["attrs"].get(key).is_some()
            && before.is_none_or(|b| b["attrs"][key] != current["attrs"][key])
    };
    let a = &current["attrs"];
    let mut replacements = vec![];
    let order: &[&str] = match kind {
        "tblPr" => &[
            "tblStyle",
            "tblpPr",
            "tblOverlap",
            "bidiVisual",
            "tblStyleRowBandSize",
            "tblStyleColBandSize",
            "tblW",
            "jc",
            "tblCellSpacing",
            "tblInd",
            "tblBorders",
            "shd",
            "tblLayout",
            "tblCellMar",
            "tblLook",
            "tblCaption",
            "tblDescription",
            "tblPrChange",
        ],
        "trPr" => &[
            "cnfStyle",
            "divId",
            "gridBefore",
            "gridAfter",
            "wBefore",
            "wAfter",
            "cantSplit",
            "trHeight",
            "tblHeader",
            "tblCellSpacing",
            "jc",
            "hidden",
            "ins",
            "del",
            "trPrChange",
        ],
        _ => &[
            "cnfStyle",
            "tcW",
            "gridSpan",
            "hMerge",
            "vMerge",
            "tcBorders",
            "shd",
            "noWrap",
            "tcMar",
            "textDirection",
            "tcFitText",
            "vAlign",
            "hideMark",
            "headers",
            "cellIns",
            "cellDel",
            "cellMerge",
            "tcPrChange",
        ],
    };
    if kind == "tblPr" {
        if changed("layout") {
            replacements.push((
                "tblLayout",
                format!(
                    "<w:tblLayout w:type=\"{}\"/>",
                    esc(a["layout"].as_str().unwrap())
                ),
            ));
        }
        if changed("alignment") {
            replacements.push((
                "jc",
                format!(
                    "<w:jc w:val=\"{}\"/>",
                    esc(a["alignment"].as_str().unwrap())
                ),
            ));
        }
        if let Some(width) = width {
            replacements.push(("tblW", format!("<w:tblW w:w=\"{width}\" w:type=\"dxa\"/>")));
        }
    } else if kind == "trPr" {
        if changed("height") {
            let h = &a["height"];
            replacements.push((
                "trHeight",
                format!(
                    "<w:trHeight w:val=\"{}\" w:hRule=\"{}\"/>",
                    h["value"].as_u64().unwrap(),
                    esc(h["rule"].as_str().unwrap())
                ),
            ));
        }
        for (key, tag) in [("header", "tblHeader"), ("cantSplit", "cantSplit")] {
            if changed(key) {
                let wrapped = docx::wrap_fragment(&props, namespaces);
                let parsed = parse(&wrapped).unwrap();
                let old = child(parsed.root_element(), "trPr")
                    .and_then(|p| child(p, tag))
                    .map(|n| scoped_record(&wrapped, n));
                let updated = crate::docx_page_setup::patch_leaf(
                    old.as_deref(),
                    tag,
                    &[("val", if a[key] == true { "on" } else { "off" }.into())],
                    &[],
                )
                .unwrap();
                replacements.push((tag, updated));
            }
        }
    } else if kind == "tcPr" {
        if changed("fill") {
            replacements.push((
                "shd",
                format!(
                    "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{}\"/>",
                    esc(a["fill"].as_str().unwrap())
                ),
            ));
        }
        if changed("verticalAlign") {
            replacements.push((
                "vAlign",
                format!(
                    "<w:vAlign w:val=\"{}\"/>",
                    esc(a["verticalAlign"].as_str().unwrap())
                ),
            ));
        }
    }
    if replacements.is_empty() {
        return props;
    }
    let wrapped = docx::wrap_fragment(&props, namespaces);
    let d = parse(&wrapped).unwrap();
    let rank = |name: &str| order.iter().position(|s| *s == name).unwrap_or(usize::MAX);
    let mut values = vec![];
    let original = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, kind)));
    if let Some(pr) = original {
        for n in pr.children() {
            if !replacements
                .iter()
                .any(|(tag, _)| n.has_tag_name((W, *tag)))
            {
                values.push((rank(n.tag_name().name()), raw(&wrapped, n).to_string()));
            }
        }
    }
    values.extend(replacements.into_iter().map(|(tag, xml)| (rank(tag), xml)));
    values.sort_by_key(|v| v.0);
    wrap_properties(kind, original, values.into_iter().map(|v| v.1).collect())
}
