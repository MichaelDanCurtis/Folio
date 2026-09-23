use crate::office::*;
use serde_json::{json, Value};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const FONT_KEYS: &[&str] = &[
    "fontName",
    "fontSize",
    "bold",
    "italic",
    "strike",
    "underline",
    "verticalAlign",
    "color",
];

// Extension metadata may use the same local names as SpreadsheetML. Only
// unqualified attributes and SpreadsheetML elements control the cell format.
fn attr<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.namespace().is_none() && a.name() == name)
        .map(|a| a.value())
}
fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children().find(|n| n.has_tag_name((S, name)))
}

#[derive(Default)]
pub struct SheetStyleBases {
    cells: std::collections::HashMap<String, String>,
    rows: std::collections::HashMap<usize, String>,
    columns: Vec<(usize, usize, String)>,
}
impl SheetStyleBases {
    pub fn projection(&self, catalog: &[Value]) -> Value {
        let style = |id: &str| {
            id.parse::<usize>()
                .ok()
                .and_then(|i| catalog.get(i))
                .cloned()
                .unwrap_or(json!({}))
        };
        json!({
            "default":style("0"),
            "rows":self.rows.iter().map(|(r,s)|(r.to_string(),style(s))).collect::<serde_json::Map<_,_>>(),
            "columns":self.columns.iter().map(|(min,max,s)|json!({"min":min,"max":max,"style":style(s)})).collect::<Vec<_>>()
        })
    }
    pub fn read(pkg: &Package, path: Option<&str>) -> Result<Self, String> {
        let Some(path) = path.filter(|p| pkg.entries.contains_key(*p)) else {
            return Ok(Self::default());
        };
        let d = parse(pkg.text(path)?)?;
        let mut result = Self::default();
        for n in d
            .descendants()
            .filter(|n| n.tag_name().namespace() == Some(S))
        {
            match n.tag_name().name() {
                "c" => {
                    if let (Some(a), Some(s)) = (attr(n, "r"), attr(n, "s")) {
                        result.cells.insert(a.into(), s.into());
                    }
                }
                "row" if matches!(attr(n, "customFormat"), Some("1" | "true")) => {
                    if let (Some(r), Some(s)) = (
                        attr(n, "r").and_then(|r| r.parse::<usize>().ok()),
                        attr(n, "s"),
                    ) {
                        result.rows.insert(r, s.into());
                    }
                }
                "col" => {
                    if let (Some(min), Some(max), Some(s)) = (
                        attr(n, "min").and_then(|v| v.parse().ok()),
                        attr(n, "max").and_then(|v| v.parse().ok()),
                        attr(n, "style"),
                    ) {
                        result.columns.push((min, max, s.into()));
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }
    pub fn at(&self, address: &str) -> Result<&str, String> {
        let (r, c) = crate::formula::parse_address(address)?;
        Ok(self
            .cells
            .get(address)
            .or_else(|| self.rows.get(&(r + 1)))
            .map(String::as_str)
            .or_else(|| {
                self.columns
                    .iter()
                    .rev()
                    .find(|(min, max, _)| c + 1 >= *min && c + 1 <= *max)
                    .map(|(_, _, s)| s.as_str())
            })
            .unwrap_or("0"))
    }
}
const ORDER: &[&str] = &[
    "numFmts",
    "fonts",
    "fills",
    "borders",
    "cellStyleXfs",
    "cellXfs",
    "cellStyles",
    "dxfs",
    "tableStyles",
    "colors",
    "extLst",
];
const BUILTINS: &[(u32, &str)] = &[
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "mm-dd-yy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yy h:mm"),
    (37, "#,##0 ;(#,##0)"),
    (38, "#,##0 ;[Red](#,##0)"),
    (39, "#,##0.00;(#,##0.00)"),
    (40, "#,##0.00;[Red](#,##0.00)"),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mmss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
];
fn record<'a, 'i>(
    root: roxmltree::Node<'a, 'i>,
    section: &str,
    id: usize,
) -> Option<roxmltree::Node<'a, 'i>> {
    child(root, section)?
        .children()
        .filter(|n| n.is_element())
        .nth(id)
}
fn component<'a>(
    root: roxmltree::Node<'a, '_>,
    xf: roxmltree::Node<'a, '_>,
    field: &str,
    flag: &str,
) -> &'a str {
    let parent = attr(xf, "xfId")
        .and_then(|v| v.parse().ok())
        .and_then(|id| record(root, "cellStyleXfs", id));
    if matches!(attr(xf, flag), Some("0" | "false")) {
        parent
            .and_then(|p| attr(p, field))
            .or_else(|| attr(xf, field))
            .unwrap_or("0")
    } else {
        attr(xf, field)
            .or_else(|| parent.and_then(|p| attr(p, field)))
            .unwrap_or("0")
    }
}
fn enabled(node: Option<roxmltree::Node>) -> bool {
    node.is_some_and(|n| !matches!(attr(n, "val"), Some("0" | "false")))
}

/// Display text excludes phonetic annotations and extension payloads. Rich
/// properties are partial overrides of the cell font, not a replacement XF.
pub fn string_content(node: roxmltree::Node) -> (String, Option<Value>) {
    let mut text = String::new();
    let mut runs = vec![];
    let mut rich = false;
    for n in node.children() {
        let (part, style) = if n.has_tag_name((S, "r")) {
            rich = true;
            let mut style = json!({});
            if let Some(properties) = child(n, "rPr") {
                for (key, tag) in [("bold", "b"), ("italic", "i"), ("strike", "strike")] {
                    if let Some(value) = child(properties, tag) {
                        style[key] = json!(enabled(Some(value)));
                    }
                }
                for (key, tag, default) in [
                    ("fontName", "rFont", ""),
                    ("underline", "u", "single"),
                    ("verticalAlign", "vertAlign", "baseline"),
                ] {
                    if let Some(value) = child(properties, tag) {
                        style[key] = json!(attr(value, "val").unwrap_or(default));
                    }
                }
                if let Some(size) = child(properties, "sz")
                    .and_then(|n| attr(n, "val"))
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| v.is_finite() && *v > 0.)
                {
                    style["fontSize"] = json!(size);
                }
                if let Some(color) = child(properties, "color") {
                    if matches!(attr(color, "auto"), Some("1" | "true")) {
                        style["color"] = json!("automatic");
                    } else if let Some(rgb) = attr(color, "rgb").filter(|v| {
                        matches!(v.len(), 6 | 8) && v.bytes().all(|c| c.is_ascii_hexdigit())
                    }) {
                        style["color"] =
                            json!(format!("#{}", if rgb.len() == 8 { &rgb[2..] } else { rgb }));
                    }
                }
            }
            (child(n, "t").and_then(|n| n.text()).unwrap_or(""), style)
        } else if n.has_tag_name((S, "t")) {
            (n.text().unwrap_or(""), json!({}))
        } else {
            continue;
        };
        let part = crate::xlsx_strings::decode(part);
        text.push_str(&part);
        if !part.is_empty() {
            runs.push(json!({"text":part,"style":style}));
        }
    }
    (text, rich.then(|| json!(runs)))
}
fn alignment<'a, 'i>(
    root: roxmltree::Node<'a, 'i>,
    xf: roxmltree::Node<'a, 'i>,
) -> Option<roxmltree::Node<'a, 'i>> {
    let inherited = attr(xf, "xfId")
        .and_then(|v| v.parse().ok())
        .and_then(|id| record(root, "cellStyleXfs", id))
        .and_then(|n| child(n, "alignment"));
    let own = child(xf, "alignment");
    if matches!(attr(xf, "applyAlignment"), Some("0" | "false")) {
        inherited
    } else {
        own.or(inherited)
    }
}
fn project(root: roxmltree::Node, xf: roxmltree::Node) -> Value {
    let font = component(root, xf, "fontId", "applyFont")
        .parse()
        .ok()
        .and_then(|id| record(root, "fonts", id));
    let fill = component(root, xf, "fillId", "applyFill")
        .parse()
        .ok()
        .and_then(|id| record(root, "fills", id));
    let num = component(root, xf, "numFmtId", "applyNumberFormat");
    let custom = child(root, "numFmts")
        .and_then(|n| n.children().find(|n| attr(*n, "numFmtId") == Some(num)))
        .and_then(|n| attr(n, "formatCode"));
    let builtin = num
        .parse::<u32>()
        .ok()
        .and_then(|id| BUILTINS.iter().find(|(n, _)| *n == id).map(|(_, s)| *s));
    let mut spec = json!({"bold":enabled(font.and_then(|n|child(n,"b"))),"italic":enabled(font.and_then(|n|child(n,"i"))),"strike":enabled(font.and_then(|n|child(n,"strike")))});
    // Unknown/localized built-ins are retained as their original XF id. They
    // must not be projected as General and later overwrite a date/currency.
    if let Some(code) = custom.or(builtin) {
        spec["numFmt"] = json!(code);
    }
    if let Some(name) = font
        .and_then(|n| child(n, "name"))
        .and_then(|n| attr(n, "val"))
    {
        spec["fontName"] = json!(name);
    }
    if let Some(size) = font
        .and_then(|n| child(n, "sz"))
        .and_then(|n| attr(n, "val"))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.)
    {
        spec["fontSize"] = json!(size);
    }
    spec["underline"] = json!(font
        .and_then(|n| child(n, "u"))
        .map(|n| attr(n, "val").unwrap_or("single"))
        .unwrap_or("none"));
    spec["verticalAlign"] = json!(font
        .and_then(|n| child(n, "vertAlign"))
        .and_then(|n| attr(n, "val"))
        .unwrap_or("baseline"));
    for (key, node) in [
        ("color", font.and_then(|n| child(n, "color"))),
        (
            "fill",
            fill.and_then(|n| child(n, "patternFill"))
                .filter(|n| attr(*n, "patternType") == Some("solid"))
                .and_then(|n| child(n, "fgColor")),
        ),
    ] {
        if key == "color" && node.is_some_and(|n| matches!(attr(n, "auto"), Some("1" | "true"))) {
            spec[key] = json!("automatic");
            continue;
        }
        if let Some(rgb) = node
            .and_then(|n| attr(n, "rgb"))
            .filter(|v| matches!(v.len(), 6 | 8) && v.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            spec[key] = json!(format!("#{}", if rgb.len() == 8 { &rgb[2..] } else { rgb }));
        }
    }
    crate::xlsx_alignment::project(alignment(root, xf), &mut spec);
    spec
}
pub fn read(pkg: &Package) -> Vec<Value> {
    let Ok(xml) = pkg.text("xl/styles.xml") else {
        return vec![];
    };
    let Ok(d) = parse(xml) else { return vec![] };
    let root = d.root_element();
    child(root, "cellXfs")
        .map(|n| {
            n.children()
                .filter(|n| n.is_element())
                .map(|xf| project(root, xf))
                .collect()
        })
        .unwrap_or_default()
}

/// Column widths use the built-in Normal style, which need not be cell XF 0
/// or have the English display name "Normal".
pub fn normal_font(pkg: &Package) -> Value {
    let Ok(xml) = pkg.text("xl/styles.xml") else {
        return json!({});
    };
    let Ok(doc) = parse(xml) else {
        return json!({});
    };
    let root = doc.root_element();
    let normal = child(root, "cellStyles").and_then(|styles| {
        styles
            .children()
            .find(|n| n.has_tag_name((S, "cellStyle")) && attr(*n, "builtinId") == Some("0"))
            .or_else(|| {
                styles.children().find(|n| {
                    n.has_tag_name((S, "cellStyle")) && attr(*n, "name") == Some("Normal")
                })
            })
    });
    let index = normal
        .and_then(|n| attr(n, "xfId"))
        .and_then(|id| id.parse().ok())
        .unwrap_or(0);
    let Some(xf) = record(root, "cellStyleXfs", index).or_else(|| record(root, "cellXfs", 0))
    else {
        return json!({});
    };
    let projected = project(root, xf);
    json!(projected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| FONT_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>())
}

fn legacy_string_values(
    pkg: &Package,
    path: &str,
    phonetics: bool,
) -> Result<serde_json::Map<String, Value>, String> {
    let value = |node: roxmltree::Node| {
        if phonetics {
            xml_text(node)
        } else {
            node.children()
                .filter_map(|n| {
                    if n.has_tag_name((S, "t")) {
                        n.text()
                    } else if n.has_tag_name((S, "r")) {
                        child(n, "t").and_then(|t| t.text())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        }
    };
    let strings = if let Ok(xml) = pkg.text("xl/sharedStrings.xml") {
        parse(xml)?
            .root_element()
            .children()
            .filter(|n| n.has_tag_name((S, "si")))
            .map(value)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let doc = parse(pkg.text(path)?)?;
    let mut result = serde_json::Map::new();
    for cell in doc
        .descendants()
        .filter(|n| n.has_tag_name((S, "c")) && child(*n, "f").is_none())
    {
        let value = match attr(cell, "t") {
            Some("inlineStr") => child(cell, "is").map(value),
            Some("s") => child(cell, "v")
                .and_then(|v| v.text())
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|i| strings.get(i).cloned()),
            Some("str") => Some(child(cell, "v").and_then(|v| v.text()).unwrap_or("").into()),
            _ => None,
        };
        if let (Some(a), Some(mut value)) = (attr(cell, "r"), value) {
            if value.parse::<f64>().is_ok()
                || value.starts_with(['=', '\''])
                || matches!(value.to_uppercase().as_str(), "TRUE" | "FALSE")
            {
                value.insert(0, '\'');
            }
            result.insert(a.into(), json!(value));
        }
    }
    Ok(result)
}

/// Upgrade only unchanged projected fields in older workspaces. Source XML is
/// authoritative for omitted metadata; actual user edits remain overrides.
pub fn hydrate(workspace: &mut Value, pkg: &Package) -> Result<(), String> {
    if let Some(warnings) = workspace["officeWorkbook"]["report"]["warnings"].as_array_mut() {
        for warning in warnings {
            if warning.as_str() == Some(crate::xlsx::DATE1904_LEGACY_WARNING) {
                *warning = json!(crate::xlsx::DATE1904_NOTICE);
            }
        }
    }

    let projected = crate::xlsx::import(pkg)?.0;
    let before = workspace["officeWorkbook"]["baseline"].clone();
    let Some(old_sheets) = before.as_array() else {
        return Ok(());
    };
    if !workspace["sheets"].is_array() {
        return Err("Invalid worksheets".into());
    }
    for (i, old) in old_sheets.iter().enumerate() {
        let Some(source) = projected
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["officePath"] == old["officePath"])
        else {
            continue;
        };
        let Some(actual) = source["cellStyles"].as_object() else {
            continue;
        };
        let current_index = workspace["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .position(|s| s["id"] == old["id"]);
        // Older importers concatenated pronunciation annotations into cell
        // values. Repair only an exact match to that old projection. Actual
        // edits are preserved and the corrected baseline matches source XML.
        let changed: Vec<_> = source["cells"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(a, v)| old["cells"][*a] != **v)
            .collect();
        if !changed.is_empty() {
            let path = source["officePath"].as_str().unwrap();
            let legacy = legacy_string_values(pkg, path, true)?;
            let escaped = legacy_string_values(pkg, path, false)?;
            for (a, value) in changed {
                if legacy.get(a).is_some_and(|v| old["cells"][a] == *v)
                    || escaped.get(a).is_some_and(|v| old["cells"][a] == *v)
                {
                    if let Some(index) = current_index {
                        if workspace["sheets"][index]["cells"][a] == old["cells"][a] {
                            migrate_text_formats(&mut workspace["sheets"][index], old, source, a)?;
                            workspace["sheets"][index]["cells"][a] = value.clone();
                        }
                    }
                    workspace["officeWorkbook"]["baseline"][i]["cells"][a] = value.clone();
                }
            }
        }
        // Correct only the old interpretation of stored string caches. A
        // package may have newer numeric caches after fill/sort/filter; those
        // are not an importer migration and must not rewrite saved snapshots.
        let cache_doc = parse(pkg.text(source["officePath"].as_str().unwrap())?)?;
        for cell in cache_doc
            .descendants()
            .filter(|n| n.has_tag_name((S, "c")) && attr(*n, "t") == Some("str"))
        {
            if let Some(a) = attr(cell, "r") {
                let raw = child(cell, "v").and_then(|n| n.text()).unwrap_or("");
                let legacy = raw.parse::<f64>().map(|v| json!(v)).unwrap_or(json!(raw));
                let value = &source["cached"][a];
                if old["cached"][a] == *value || old["cached"][a] != legacy {
                    continue;
                }
                if let Some(index) = current_index {
                    if workspace["sheets"][index]["cached"][a] == old["cached"][a]
                        && workspace["sheets"][index]["cells"][a] == old["cells"][a]
                    {
                        workspace["sheets"][index]["cached"][a] = value.clone();
                    }
                }
                workspace["officeWorkbook"]["baseline"][i]["cached"][a] = value.clone();
            }
        }
        if let Some(index) = current_index {
            if workspace["sheets"][index].get("styleDefaults").is_none() {
                workspace["sheets"][index]["styleDefaults"] = source["styleDefaults"].clone();
            }
            crate::xlsx_alignment::hydrate_defaults(
                &mut workspace["sheets"][index]["styleDefaults"],
                &source["styleDefaults"],
            );
            if workspace["sheets"][index]["styleDefaults"]
                .get("normalFont")
                .is_none()
            {
                workspace["sheets"][index]["styleDefaults"]["normalFont"] =
                    source["styleDefaults"]["normalFont"].clone();
            }
        }
        if workspace["officeWorkbook"]["baseline"][i]
            .get("styleDefaults")
            .is_none()
        {
            workspace["officeWorkbook"]["baseline"][i]["styleDefaults"] =
                source["styleDefaults"].clone();
        }
        crate::xlsx_alignment::hydrate_defaults(
            &mut workspace["officeWorkbook"]["baseline"][i]["styleDefaults"],
            &source["styleDefaults"],
        );
        if workspace["officeWorkbook"]["baseline"][i]["styleDefaults"]
            .get("normalFont")
            .is_none()
        {
            workspace["officeWorkbook"]["baseline"][i]["styleDefaults"]["normalFont"] =
                source["styleDefaults"]["normalFont"].clone();
        }
        for (a, spec) in actual {
            for key in [
                "bold",
                "italic",
                "strike",
                "underline",
                "verticalAlign",
                "fontName",
                "fontSize",
                "numFmt",
                "color",
                "fill",
                "_richText",
            ]
            .into_iter()
            .chain(crate::xlsx_alignment::KEYS.iter().map(|(key, _)| *key))
            {
                if let Some(index) = current_index {
                    let current = &mut workspace["sheets"][index];
                    if current["cellStyles"][a].get(key) == old["cellStyles"][a].get(key) {
                        put_style_property(current, a, key, spec.get(key));
                    }
                }
                put_style_property(
                    &mut workspace["officeWorkbook"]["baseline"][i],
                    a,
                    key,
                    spec.get(key),
                );
            }
        }
    }
    Ok(())
}

/// Character formatting in pre-codec workspaces used the visible escape-token
/// positions. Translate complete-token boundaries, retaining the exact font
/// overrides. A boundary inside one escape has no equivalent character; refuse
/// that upgrade rather than silently reassigning an authored font selection.
fn migrate_text_formats(
    current: &mut Value,
    old: &Value,
    source: &Value,
    a: &str,
) -> Result<(), String> {
    let style = current["cellStyles"][a].clone();
    let old_text = crate::sheet_text::literal(old["cells"][a].as_str().unwrap_or(""))?;
    let new_text = crate::sheet_text::literal(source["cells"][a].as_str().unwrap_or(""))?;
    let parts = old["cellStyles"][a]["_richText"]
        .as_array()
        .filter(|runs| {
            runs.iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<String>()
                == old_text
        })
        .map(|runs| {
            runs.iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![old_text]);
    let mut boundaries = std::collections::BTreeMap::new();
    let (mut old_offset, mut new_offset) = (0, 0);
    let mut decoded = String::new();
    for part in parts {
        for (before, after) in crate::xlsx_strings::boundaries(part) {
            boundaries.insert(old_offset + before, new_offset + after);
        }
        let text = crate::xlsx_strings::decode(part);
        old_offset += part.encode_utf16().count();
        new_offset += text.encode_utf16().count();
        decoded.push_str(&text);
    }
    if decoded != new_text {
        // The earlier phonetic-annotation repair has no character offsets to
        // translate in supported legacy files.
        if style.get("_richTextEdits").is_some() {
            return Err(format!("Cannot safely upgrade character formatting at {a} from this older workspace. The file is unchanged; reopen it with the previous Folio version."));
        }
        return Ok(());
    }
    if style["_richTextSource"] != old_text || style.get("_richTextEdits").is_none() {
        // Whole-cell font changes may already have modified run properties.
        // Upgrade their text separately, without replacing those properties.
        if let Some(runs) = current
            .get_mut("cellStyles")
            .and_then(|s| s.get_mut(a))
            .and_then(|s| s.get_mut("_richText"))
            .and_then(Value::as_array_mut)
        {
            if runs
                .iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<String>()
                == old_text
            {
                for run in runs {
                    run["text"] = json!(crate::xlsx_strings::decode(
                        run["text"].as_str().unwrap_or("")
                    ));
                }
            }
        }
        return Ok(());
    }
    let mut translated = vec![];
    for (lo, hi, font) in crate::sheet_text::read_edits(old_text, &style["_richTextEdits"])? {
        let position = |offset| {
            boundaries.get(&offset).copied().ok_or_else(||
            format!("Cannot safely upgrade character formatting at {a}: a selection ends inside an Excel escape sequence. The file is unchanged; reopen it with the previous Folio version."))
        };
        translated.push(json!({"start":position(lo)?,"end":position(hi)?,"style":font}));
    }
    let mapped = json!(translated);
    let ranges = crate::sheet_text::read_edits(new_text, &mapped)?;
    let base = source["cellStyles"][a]["_richText"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![json!({"text":new_text,"style":{}})]);
    let mut projected = vec![];
    let mut offset = 0;
    let mut first = 0;
    for run in base {
        let text = run["text"].as_str().unwrap_or("");
        let stop = offset + text.encode_utf16().count();
        while first < ranges.len() && ranges[first].1 <= offset {
            first += 1;
        }
        let relevant = ranges[first..]
            .iter()
            .take_while(|r| r.0 < stop)
            .collect::<Vec<_>>();
        let mut cuts = std::collections::BTreeSet::from([offset, stop]);
        for (lo, hi, _) in &relevant {
            cuts.insert((*lo).max(offset));
            cuts.insert((*hi).min(stop));
        }
        let cuts = cuts.into_iter().collect::<Vec<_>>();
        let mut at = 0;
        for span in cuts.windows(2) {
            let (lo, hi) = (span[0], span[1]);
            let mut font = run["style"].as_object().cloned().unwrap_or_default();
            while at < relevant.len() && relevant[at].1 <= lo {
                at += 1;
            }
            if let Some((start, end, edits)) = relevant.get(at) {
                if *start <= lo && hi <= *end {
                    font.extend(edits.as_object().unwrap().clone());
                }
            }
            let part = &text[crate::sheet_text::byte_at(text, lo - offset)?
                ..crate::sheet_text::byte_at(text, hi - offset)?];
            projected.push(json!({"text":part,"style":font}));
        }
        offset = stop;
    }
    current["cellStyles"][a]["_richText"] = json!(projected);
    current["cellStyles"][a]["_richTextSource"] = json!(new_text);
    current["cellStyles"][a]["_richTextEdits"] = mapped;
    Ok(())
}
fn put_style_property(sheet: &mut Value, address: &str, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        if !sheet["cellStyles"].is_object() {
            sheet["cellStyles"] = json!({});
        }
        if !sheet["cellStyles"][address].is_object() {
            sheet["cellStyles"][address] = json!({});
        }
        sheet["cellStyles"][address][key] = value.clone();
    } else if let Some(style) = sheet["cellStyles"][address].as_object_mut() {
        style.remove(key);
    }
}
fn signature(n: roxmltree::Node) -> Value {
    if n.is_element() {
        let mut attrs = n
            .attributes()
            .map(|a| (a.namespace().unwrap_or(""), a.name(), a.value()))
            .collect::<Vec<_>>();
        attrs.sort();
        json!([
            n.tag_name().namespace(),
            n.tag_name().name(),
            attrs,
            n.children()
                .filter(
                    |c| c.is_element() || c.is_text() && !c.text().unwrap_or("").trim().is_empty()
                )
                .map(signature)
                .collect::<Vec<_>>()
        ])
    } else {
        json!(n.text().unwrap_or(""))
    }
}
pub(crate) fn section(xml: &str, name: &str, entry: &str) -> Result<(String, usize), String> {
    let d = parse(xml)?;
    let root = d.root_element();
    let candidate = parse(entry)?;
    let fingerprint = signature(candidate.root_element());
    if let Some(node) = child(root, name) {
        let records = node
            .children()
            .filter(|n| n.is_element())
            .collect::<Vec<_>>();
        if let Some(id) = records.iter().position(|n| signature(*n) == fingerprint) {
            return Ok((xml.into(), id));
        }
        let count = records.len();
        let inner = node.children().map(|n| raw(xml, n)).collect::<String>() + entry;
        let base =
            crate::xlsx_tables::attributes(xml, node, &[("count", Some((count + 1).to_string()))]);
        let replacement = crate::xlsx::fragment_inner(&base, &inner)?;
        Ok((
            replace_ranges(xml, vec![(node.range(), replacement)]),
            count,
        ))
    } else {
        let rank = ORDER
            .iter()
            .position(|n| *n == name)
            .ok_or("Unknown style section")?;
        let at = root
            .children()
            .find(|n| {
                n.is_element()
                    && ORDER
                        .iter()
                        .position(|t| *t == n.tag_name().name())
                        .is_some_and(|r| r > rank)
            })
            .map(|n| n.range().start)
            .unwrap_or_else(|| xml.rfind("</").unwrap());
        Ok((
            replace_ranges(
                xml,
                vec![(
                    at..at,
                    format!("<{name} xmlns=\"{S}\" count=\"1\">{entry}</{name}>"),
                )],
            ),
            0,
        ))
    }
}
// Change a direct font child while preserving all other fields and extensions.
fn font_property(xml: &str, name: &str, value: Option<String>) -> Result<String, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    let replacement = value.unwrap_or_default();
    if let Some(node) = root.children().find(|n| n.has_tag_name((S, name))) {
        Ok(replace_ranges(xml, vec![(node.range(), replacement)]))
    } else if replacement.is_empty() {
        Ok(xml.into())
    } else {
        // CT_Font and CT_RPrElt put style toggles before size/color/name.
        // Appending a new bold/underline child after the font name produces an
        // invalid font record even though tolerant readers still display it.
        let order = [
            "b",
            "i",
            "strike",
            "condense",
            "extend",
            "outline",
            "shadow",
            "u",
            "vertAlign",
            "sz",
            "color",
            "name",
            "rFont",
            "family",
            "charset",
            "scheme",
        ];
        let rank = order
            .iter()
            .position(|n| *n == name)
            .ok_or("Unknown font property")?;
        let at = root
            .children()
            .find(|n| {
                n.tag_name().namespace() == Some(S)
                    && order
                        .iter()
                        .position(|s| *s == n.tag_name().name())
                        .is_some_and(|r| r > rank)
            })
            .map(|n| n.range().start)
            .or_else(|| xml.rfind("</"))
            .ok_or("Invalid font record")?;
        Ok(replace_ranges(xml, vec![(at..at, replacement)]))
    }
}

/// Whole-cell font commands affect rich runs too. Shared strings are copied
/// into this cell, so another cell using the same string is never changed.
pub fn format_rich_cell(pkg: &Package, xml: &str, spec: &Value) -> Result<String, String> {
    let d = parse(xml)?;
    let cell = d.root_element();
    let content = if attr(cell, "t") == Some("s") {
        let index = child(cell, "v")
            .and_then(|n| n.text())
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or("Invalid shared string")?;
        let strings = pkg.text("xl/sharedStrings.xml")?;
        let sd = parse(strings)?;
        let si = sd
            .root_element()
            .children()
            .filter(|n| n.has_tag_name((S, "si")))
            .nth(index)
            .ok_or("Shared string missing")?;
        format!(
            "<is xmlns=\"{S}\">{}</is>",
            si.children()
                .filter(|n| n.is_element())
                .map(|n| scoped_record(strings, n))
                .collect::<String>()
        )
    } else if let Some(is) = child(cell, "is") {
        scoped_record(xml, is)
    } else {
        return Ok(xml.into());
    };
    let data = parse(&content)?;
    let mut edits = vec![];
    for run in data
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, "r")))
    {
        let old = child(run, "rPr");
        let mut properties = old
            .map(|n| scoped_record(&content, n))
            .unwrap_or_else(|| format!("<rPr xmlns=\"{S}\"></rPr>"));
        properties = edit_run_properties(properties, spec)?;
        let copied = scoped_record(&content, run);
        let rd = parse(&copied)?;
        let r = rd.root_element();
        let replacement = if let Some(old) = child(r, "rPr") {
            replace_ranges(&copied, vec![(old.range(), properties)])
        } else {
            let at = copied.find('>').unwrap() + 1;
            replace_ranges(&copied, vec![(at..at, properties)])
        };
        edits.push((run.range(), replacement));
    }
    if edits.is_empty() {
        return Ok(xml.into());
    }
    let content = replace_ranges(&content, edits);
    let base = crate::xlsx_tables::attributes(xml, cell, &[("t", Some("inlineStr".into()))]);
    let extra = cell
        .children()
        .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "is" | "v"))
        .map(|n| raw(xml, n))
        .collect::<String>();
    crate::xlsx::fragment_inner(&base, &(content + &extra))
}
pub fn ensure(pkg: &mut Package) -> Result<(), String> {
    if pkg.entries.contains_key("xl/styles.xml") {
        return Ok(());
    }
    pkg.set("xl/styles.xml",format!("{XML_HEAD}<styleSheet xmlns=\"{S}\"><fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Aptos\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border/></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/></cellXfs></styleSheet>"));
    for(path,entry)in[("xl/_rels/workbook.xml.rels",format!("<Relationship Id=\"folioStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>")),("[Content_Types].xml","<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>".into())]{let xml=pkg.text(path)?.to_string();let d=parse(&xml)?;let root=d.root_element();let inner=root.children().map(|n|raw(&xml,n)).collect::<String>();pkg.set(path,replace_inner(&xml,root,&(inner+&entry)));}
    Ok(())
}

/// Split only the selected character spans, cloning each original run and
/// property record so unmodeled effects, namespace attributes and annotations
/// survive. Shared strings are materialized only in the edited cell.
fn character_cell(
    pkg: &Package,
    xml: &str,
    expected: &str,
    edits: &Value,
) -> Result<String, String> {
    let ranges = crate::sheet_text::read_edits(expected, edits)?;
    if ranges.is_empty() {
        return Ok(xml.into());
    }
    let doc = parse(xml)?;
    let cell = doc.root_element();
    let content = if attr(cell, "t") == Some("s") {
        let index = child(cell, "v")
            .and_then(|n| n.text())
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or("Invalid shared string")?;
        let strings = pkg.text("xl/sharedStrings.xml")?;
        let sd = parse(strings)?;
        let si = sd
            .root_element()
            .children()
            .filter(|n| n.has_tag_name((S, "si")))
            .nth(index)
            .ok_or("Shared string missing")?;
        format!(
            "<is xmlns=\"{S}\">{}</is>",
            si.children()
                .filter(|n| n.is_element())
                .map(|n| scoped_record(strings, n))
                .collect::<String>()
        )
    } else if let Some(is) = child(cell, "is") {
        scoped_record(xml, is)
    } else if attr(cell, "t") == Some("str") && child(cell, "f").is_none() {
        let value = child(cell, "v").and_then(|n| n.text()).unwrap_or("");
        format!(
            "<is xmlns=\"{S}\"><t xml:space=\"preserve\">{}</t></is>",
            esc(value)
        )
    } else {
        return Err("Character formatting requires a text cell.".into());
    };
    let data = parse(&content)?;
    if string_content(data.root_element()).0 != expected {
        return Err("The source text changed before character formatting.".into());
    }
    let mut replacements = vec![];
    let mut offset = 0;
    let mut first_range = 0;
    for source in data
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, "r")) || n.has_tag_name((S, "t")))
    {
        let node = if source.has_tag_name((S, "t")) {
            Some(source)
        } else {
            child(source, "t")
        };
        let text = crate::xlsx_strings::decode(node.and_then(|n| n.text()).unwrap_or(""));
        let stop = offset + text.encode_utf16().count();
        while first_range < ranges.len() && ranges[first_range].1 <= offset {
            first_range += 1;
        }
        let relevant: Vec<_> = ranges[first_range..]
            .iter()
            .take_while(|r| r.0 < stop)
            .collect();
        if relevant.is_empty() || text.is_empty() {
            offset = stop;
            continue;
        }
        let mut cuts = std::collections::BTreeSet::from([offset, stop]);
        for (a, b, _) in &relevant {
            cuts.insert((*a).max(offset));
            cuts.insert((*b).min(stop));
        }
        let cuts: Vec<_> = cuts.into_iter().collect();
        let prototype = if source.has_tag_name((S, "t")) {
            format!("<r xmlns=\"{S}\">{}</r>", scoped_record(&content, source))
        } else {
            scoped_record(&content, source)
        };
        let mut output = String::new();
        let mut range_index = 0;
        for span in cuts.windows(2) {
            let (a, b) = (span[0], span[1]);
            let slice = &text[crate::sheet_text::byte_at(&text, a - offset)?
                ..crate::sheet_text::byte_at(&text, b - offset)?];
            let mut run = prototype.clone();
            let rd = parse(&run)?;
            let t = child(rd.root_element(), "t").ok_or("Rich text missing")?;
            let copied = scoped_record(&run, t);
            let td = parse(&copied)?;
            let tag = if let Some(space) = td.root_element().attributes().find(|a| {
                a.namespace() == Some("http://www.w3.org/XML/1998/namespace") && a.name() == "space"
            }) {
                replace_ranges(&copied, vec![(space.range_value(), "preserve".into())])
            } else {
                crate::xlsx_tables::attributes(
                    &copied,
                    td.root_element(),
                    &[("xml:space", Some("preserve".into()))],
                )
            };
            let tag = crate::xlsx::fragment_inner(&tag, &crate::xlsx_strings::xml(slice, false))?;
            run = replace_ranges(&run, vec![(t.range(), tag)]);
            while range_index < relevant.len() && relevant[range_index].1 <= a {
                range_index += 1;
            }
            if let Some((lo, hi, style)) = relevant.get(range_index) {
                if *lo <= a && b <= *hi {
                    let rd = parse(&run)?;
                    let root = rd.root_element();
                    let old = child(root, "rPr");
                    let properties = edit_run_properties(
                        old.map(|n| scoped_record(&run, n))
                            .unwrap_or_else(|| format!("<rPr xmlns=\"{S}\"></rPr>")),
                        style,
                    )?;
                    let range = old.map(|n| n.range()).unwrap_or_else(|| {
                        let at = run.find('>').unwrap() + 1;
                        at..at
                    });
                    run = replace_ranges(&run, vec![(range, properties)]);
                }
            }
            output.push_str(&run);
        }
        replacements.push((source.range(), output));
        offset = stop;
    }
    let content = replace_ranges(&content, replacements);
    let base = crate::xlsx_tables::attributes(xml, cell, &[("t", Some("inlineStr".into()))]);
    let extra = cell
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((S, "is")) && !n.has_tag_name((S, "v")))
        .map(|n| scoped_record(xml, n))
        .collect::<String>();
    crate::xlsx::fragment_inner(&base, &(content + &extra))
}

pub fn patch_character_formats(pkg: &Package, xml: &str, sheet: &Value) -> Result<String, String> {
    let Some(styles) = sheet["cellStyles"].as_object() else {
        return Ok(xml.into());
    };
    if !styles.values().any(|s| s.get("_richTextEdits").is_some()) {
        return Ok(xml.into());
    }
    let d = parse(xml)?;
    let mut edits = vec![];
    for cell in d.descendants().filter(|n| n.has_tag_name((S, "c"))) {
        let Some(a) = attr(cell, "r") else {
            continue;
        };
        let style = &sheet["cellStyles"][a];
        let (Some(source), Some(ranges)) = (
            style["_richTextSource"].as_str(),
            style.get("_richTextEdits"),
        ) else {
            continue;
        };
        if crate::sheet_text::literal(sheet["cells"][a].as_str().unwrap_or("")).ok() != Some(source)
        {
            continue;
        }
        let mut value = character_cell(pkg, &scoped_record(xml, cell), source, ranges)?;
        if let Some(font) = style.get("_fontOverrides") {
            value = format_rich_cell(pkg, &value, font)?;
        }
        edits.push((cell.range(), value));
    }
    Ok(replace_ranges(xml, edits))
}
pub fn add(pkg: &mut Package, spec: &Value, base: Option<&str>) -> Result<String, String> {
    ensure(pkg)?;
    let source = pkg.text("xl/styles.xml")?.to_string();
    let d = parse(&source)?;
    let root = d.root_element();
    let base_id = base.and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    let xf = record(root, "cellXfs", base_id)
        .ok_or("Cell format is missing from the source workbook")?;
    let projected = project(root, xf);
    let changed = |key: &str| spec.get(key).filter(|v| projected.get(key) != Some(*v));
    let mut xf_xml = scoped_record(&source, xf);
    let mut xml = source.clone();
    let mut updates = vec![];
    let font_id = component(root, xf, "fontId", "applyFont")
        .parse::<usize>()
        .map_err(|_| "Invalid font id")?;
    let original_font =
        record(root, "fonts", font_id).ok_or("Cell font is missing from the source workbook")?;
    let mut font = scoped_record(&source, original_font);
    for (key, tag) in [("bold", "b"), ("italic", "i"), ("strike", "strike")] {
        if let Some(value) = changed(key) {
            font = font_property(
                &font,
                tag,
                (value == true).then(|| format!("<{tag} xmlns=\"{S}\"/>")),
            )?;
        }
    }
    for (key, tag) in [
        ("fontName", "name"),
        ("fontSize", "sz"),
        ("underline", "u"),
        ("verticalAlign", "vertAlign"),
    ] {
        if let Some(value) = changed(key) {
            let text = value
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| value.to_string());
            font = font_property(
                &font,
                tag,
                ((key != "underline" || text != "none")
                    && (key != "verticalAlign" || text != "baseline"))
                    .then(|| format!("<{tag} xmlns=\"{S}\" val=\"{}\"/>", esc(&text))),
            )?;
            if key == "fontName" {
                for related in ["scheme", "family", "charset"] {
                    font = font_property(&font, related, None)?;
                }
            }
        }
    }
    if let Some(color) = changed("color").and_then(Value::as_str) {
        font = font_property(
            &font,
            "color",
            Some(if color == "automatic" {
                format!("<color xmlns=\"{S}\" auto=\"1\"/>")
            } else {
                format!(
                    "<color xmlns=\"{S}\" rgb=\"FF{}\"/>",
                    esc(color.trim_start_matches('#'))
                )
            }),
        )?;
    }
    if signature(parse(&font)?.root_element()) != signature(original_font) {
        let (next, id) = section(&xml, "fonts", &font)?;
        xml = next;
        updates.push(("fontId", Some(id.to_string())));
        updates.push(("applyFont", Some("1".into())));
    }
    if let Some(color) = changed("fill").and_then(Value::as_str) {
        let fill=format!("<fill xmlns=\"{S}\"><patternFill patternType=\"solid\"><fgColor rgb=\"FF{}\"/><bgColor indexed=\"64\"/></patternFill></fill>",esc(color.trim_start_matches('#')));
        let (next, id) = section(&xml, "fills", &fill)?;
        xml = next;
        updates.push(("fillId", Some(id.to_string())));
        updates.push(("applyFill", Some("1".into())));
    }
    if let Some(code) = changed("numFmt").and_then(Value::as_str) {
        let current = parse(&xml)?;
        let custom = child(current.root_element(), "numFmts")
            .and_then(|n| n.children().find(|n| attr(*n, "formatCode") == Some(code)))
            .and_then(|n| attr(n, "numFmtId"))
            .map(String::from);
        let id = if let Some(id) = custom {
            id
        } else if let Some((id, _)) = BUILTINS.iter().find(|(_, f)| *f == code) {
            id.to_string()
        } else {
            let max = current
                .descendants()
                .filter_map(|n| attr(n, "numFmtId").and_then(|s| s.parse::<u32>().ok()))
                .max()
                .unwrap_or(163)
                .max(163)
                .checked_add(1)
                .ok_or("Number format limit reached")?;
            xml = section(
                &xml,
                "numFmts",
                &format!(
                    "<numFmt xmlns=\"{S}\" numFmtId=\"{max}\" formatCode=\"{}\"/>",
                    esc(code)
                ),
            )?
            .0;
            max.to_string()
        };
        updates.push(("numFmtId", Some(id)));
        updates.push(("applyNumberFormat", Some("1".into())));
    }
    let mut alignment_source = alignment(root, xf).map(|n| scoped_record(&source, n));
    if crate::xlsx_alignment::KEYS
        .iter()
        .any(|(key, _)| changed(key).is_some())
        && matches!(attr(xf, "applyAlignment"), Some("0" | "false"))
    {
        if let Some(own) = child(xf, "alignment") {
            // Keep metadata attached to an ignored direct record while
            // materializing the effective parent settings as explicit values.
            alignment_source = crate::xlsx_alignment::patch(
                Some(scoped_record(&source, own)),
                projected.as_object().unwrap(),
                &json!({}),
            )?;
        }
    }
    if let Some(value) = crate::xlsx_alignment::patch(
        alignment_source,
        spec.as_object().ok_or("Invalid cell format")?,
        &projected,
    )? {
        let xd = parse(&xf_xml)?;
        let own = child(xd.root_element(), "alignment");
        let range = own.map(|n| n.range()).unwrap_or_else(|| {
            let at = xd
                .root_element()
                .children()
                .find(|n| n.has_tag_name((S, "protection")) || n.has_tag_name((S, "extLst")))
                .map(|n| n.range().start)
                .unwrap_or_else(|| xf_xml.rfind("</").unwrap_or(xf_xml.len()));
            at..at
        });
        if xf_xml.trim_end().ends_with("/>") {
            xf_xml = crate::xlsx::fragment_inner(&xf_xml, &value)?;
        } else {
            xf_xml = replace_ranges(&xf_xml, vec![(range, value)]);
        }
        updates.push(("applyAlignment", Some("1".into())));
    }
    if updates.is_empty() {
        return Ok(base_id.to_string());
    }
    let xd = parse(&xf_xml)?;
    xf_xml = crate::xlsx_tables::attributes(&xf_xml, xd.root_element(), &updates);
    let (xml, id) = section(&xml, "cellXfs", &xf_xml)?;
    pkg.set("xl/styles.xml", xml);
    Ok(id.to_string())
}

pub fn validate_format(style: &serde_json::Map<String, Value>) -> Result<(), String> {
    for (key, v) in style {
        match key.as_str() {
            "bold" | "italic" | "strike" => {
                if !v.is_boolean() {
                    return Err("Invalid text style".into());
                }
            }
            "fontName" => {
                if v.as_str().is_none_or(|s| {
                    s.is_empty()
                        || s.trim() != s
                        || s.chars().count() > 255
                        || s.chars().any(char::is_control)
                }) {
                    return Err("Enter a font name of 1–255 characters.".into());
                }
            }
            "fontSize" => {
                if v.as_f64().is_none_or(|n| {
                    !n.is_finite() || !(1.0..=409.0).contains(&n) || (n * 2.0).fract() != 0.0
                }) {
                    return Err(
                        "Enter a font size from 1 to 409 points, in half-point steps.".into(),
                    );
                }
            }
            "underline" => {
                if !matches!(
                    v.as_str(),
                    Some("none" | "single" | "double" | "singleAccounting" | "doubleAccounting")
                ) {
                    return Err("Choose an underline style.".into());
                }
            }
            "verticalAlign" => {
                if !matches!(v.as_str(), Some("baseline" | "superscript" | "subscript")) {
                    return Err("Choose normal, superscript or subscript text.".into());
                }
            }
            "numFmt" => {
                crate::number_format::validate(v.as_str().ok_or("Invalid number format")?)?;
            }
            "color" | "fill" => {
                if key == "color" && v == "automatic" {
                    continue;
                }
                if v.as_str().is_none_or(|s| {
                    s.len() != 7
                        || !s.starts_with('#')
                        || !s[1..].bytes().all(|b| b.is_ascii_hexdigit())
                }) {
                    return Err("Invalid color".into());
                }
            }
            key if crate::xlsx_alignment::KEYS.iter().any(|(k, _)| *k == key) => {
                crate::xlsx_alignment::validate(key, v)?
            }
            _ => return Err("Unsupported format property".into()),
        }
    }
    Ok(())
}

fn edit_run_properties(mut properties: String, spec: &Value) -> Result<String, String> {
    for key in FONT_KEYS {
        let Some(value) = spec.get(*key) else {
            continue;
        };
        let tag = match *key {
            "fontName" => "rFont",
            "fontSize" => "sz",
            "bold" => "b",
            "italic" => "i",
            "verticalAlign" => "vertAlign",
            "underline" => "u",
            other => other,
        };
        let replacement = if *key == "color" {
            let color = value.as_str().ok_or("Invalid font color")?;
            if color == "automatic" {
                format!("<color xmlns=\"{S}\" auto=\"1\"/>")
            } else {
                format!(
                    "<color xmlns=\"{S}\" rgb=\"FF{}\"/>",
                    esc(color.trim_start_matches('#'))
                )
            }
        } else {
            let val = if let Some(b) = value.as_bool() {
                if b {
                    "1".into()
                } else {
                    "0".into()
                }
            } else if let Some(s) = value.as_str() {
                s.into()
            } else {
                value.to_string()
            };
            format!("<{tag} xmlns=\"{S}\" val=\"{}\"/>", esc(&val))
        };
        properties = font_property(&properties, tag, Some(replacement))?;
        if *key == "fontName" {
            for related in ["scheme", "family", "charset"] {
                properties = font_property(&properties, related, None)?;
            }
        }
    }
    Ok(properties)
}

/// Import complete XF dependencies, rather than interpreting the UI's subset
/// of font/alignment fields. Clipboard snapshots may predate later style edits.
/// Copy only the effective number format, retaining the destination XF and all
/// of its other components. Built-in IDs stay IDs (including locale formats
/// absent from our display catalog); custom records receive collision-free IDs.
pub fn import_number_format_record(target: &mut Package, entry: &str) -> Result<u32, String> {
    let source = parse(entry)?;
    let item = source.root_element();
    if !item.has_tag_name((S, "numFmt")) {
        return Err("Invalid copied number format record".into());
    }
    let code = attr(item, "formatCode").ok_or("Copied number format code is missing")?;
    ensure(target)?;
    let tx = target.text("xl/styles.xml")?;
    let td = parse(tx)?;
    if let Some(id) = child(td.root_element(), "numFmts")
        .and_then(|n| n.children().find(|n| attr(*n, "formatCode") == Some(code)))
        .and_then(|n| attr(n, "numFmtId"))
        .and_then(|s| s.parse::<u32>().ok())
    {
        return Ok(id);
    }
    let next = td
        .descendants()
        .filter_map(|n| attr(n, "numFmtId").and_then(|s| s.parse::<u32>().ok()))
        .max()
        .unwrap_or(163)
        .max(163)
        .checked_add(1)
        .ok_or("Number format limit reached")?;
    let scoped = scoped_record(entry, item);
    let d = parse(&scoped)?;
    let rewritten = crate::xlsx_tables::attributes(
        &scoped,
        d.root_element(),
        &[("numFmtId", Some(next.to_string()))],
    );
    let xml = section(tx, "numFmts", &rewritten)?.0;
    target.set("xl/styles.xml", xml);
    Ok(next)
}

pub fn transfer_number_format(
    source: &Package,
    target: &mut Package,
    source_id: &str,
    target_id: &str,
) -> Result<String, String> {
    if !source.entries.contains_key("xl/styles.xml") {
        let mut defaults = source.clone();
        ensure(&mut defaults)?;
        return transfer_number_format(&defaults, target, source_id, target_id);
    }
    ensure(target)?;
    let sx = source.text("xl/styles.xml")?;
    let sd = parse(sx)?;
    let root = sd.root_element();
    let source_xf = record(
        root,
        "cellXfs",
        source_id.parse().map_err(|_| "Invalid copied format")?,
    )
    .ok_or("Copied cell format is missing")?;
    let id = component(root, source_xf, "numFmtId", "applyNumberFormat");
    let numeric_id: u32 = id.parse().map_err(|_| "Invalid copied number format")?;
    let mut tx = target.text("xl/styles.xml")?.to_string();
    let mapped = if numeric_id < 164 {
        id.to_string()
    } else {
        let item = child(root, "numFmts")
            .and_then(|n| n.children().find(|n| attr(*n, "numFmtId") == Some(id)))
            .ok_or("Copied number format is missing")?;
        let code = attr(item, "formatCode").ok_or("Copied number format code is missing")?;
        let td = parse(&tx)?;
        let formats = child(td.root_element(), "numFmts");
        let matching = formats
            .and_then(|n| n.children().find(|n| attr(*n, "formatCode") == Some(code)))
            .and_then(|n| attr(n, "numFmtId"))
            .map(str::to_string);
        if let Some(id) = matching {
            id
        } else {
            let next = td
                .descendants()
                .filter_map(|n| attr(n, "numFmtId").and_then(|s| s.parse::<u32>().ok()))
                .max()
                .unwrap_or(163)
                .max(163)
                .checked_add(1)
                .ok_or("Number format limit reached")?
                .to_string();
            let entry = scoped_record(sx, item);
            let ed = parse(&entry)?;
            let entry = crate::xlsx_tables::attributes(
                &entry,
                ed.root_element(),
                &[("numFmtId", Some(next.clone()))],
            );
            tx = section(&tx, "numFmts", &entry)?.0;
            next
        }
    };
    let td = parse(&tx)?;
    let base = record(
        td.root_element(),
        "cellXfs",
        target_id
            .parse()
            .map_err(|_| "Invalid destination format")?,
    )
    .ok_or("Destination cell format is missing")?;
    let xf = scoped_record(&tx, base);
    let xd = parse(&xf)?;
    let xf = crate::xlsx_tables::attributes(
        &xf,
        xd.root_element(),
        &[
            ("numFmtId", Some(mapped)),
            ("applyNumberFormat", Some("1".into())),
        ],
    );
    let (xml, id) = section(&tx, "cellXfs", &xf)?;
    target.set("xl/styles.xml", xml);
    Ok(id.to_string())
}

pub fn transfer(source: &Package, target: &mut Package, id: &str) -> Result<String, String> {
    // A new unformatted workbook can omit its optional styles part. Materialize
    // its default style before copying, rather than adopting the target's XF 0.
    if !source.entries.contains_key("xl/styles.xml") && id == "0" {
        let mut defaults = source.clone();
        ensure(&mut defaults)?;
        return transfer(&defaults, target, id);
    }
    ensure(target)?;
    let source_xml = source.text("xl/styles.xml")?;
    let d = parse(source_xml)?;
    fn xf(
        source: &str,
        root: roxmltree::Node,
        target: &mut String,
        kind: &str,
        id: usize,
    ) -> Result<usize, String> {
        let n = record(root, kind, id).ok_or("Copied cell format is missing")?;
        let mut entry = scoped_record(source, n);
        let mut updates = vec![];
        for (attribute, collection) in [
            ("fontId", "fonts"),
            ("fillId", "fills"),
            ("borderId", "borders"),
        ] {
            if let Some(id) = attr(n, attribute) {
                let item = record(
                    root,
                    collection,
                    id.parse().map_err(|_| "Invalid copied format")?,
                )
                .ok_or("Copied format dependency missing")?;
                let (xml, mapped) = section(target, collection, &scoped_record(source, item))?;
                *target = xml;
                updates.push((attribute, Some(mapped.to_string())));
            }
        }
        if let Some(id) = attr(n, "numFmtId")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 164)
        {
            let item = child(root, "numFmts")
                .and_then(|n| {
                    n.children().find(|n| {
                        attr(*n, "numFmtId").and_then(|v| v.parse::<usize>().ok()) == Some(id)
                    })
                })
                .ok_or("Copied number format missing")?;
            let td = parse(target)?;
            let existing = child(td.root_element(), "numFmts");
            let matching = existing
                .and_then(|n| {
                    n.children()
                        .find(|n| attr(*n, "formatCode") == attr(item, "formatCode"))
                })
                .and_then(|n| attr(n, "numFmtId"))
                .map(str::to_string);
            let mapped = if let Some(id) = matching {
                id
            } else {
                let next = existing
                    .into_iter()
                    .flat_map(|n| n.children())
                    .filter_map(|n| attr(n, "numFmtId").and_then(|v| v.parse::<usize>().ok()))
                    .max()
                    .unwrap_or(163)
                    .max(163)
                    + 1;
                let entry = scoped_record(source, item);
                let ed = parse(&entry)?;
                let entry = crate::xlsx_tables::attributes(
                    &entry,
                    ed.root_element(),
                    &[("numFmtId", Some(next.to_string()))],
                );
                *target = section(target, "numFmts", &entry)?.0;
                next.to_string()
            };
            updates.push(("numFmtId", Some(mapped)));
        }
        if kind == "cellXfs" {
            if let Some(base) = attr(n, "xfId") {
                let mapped = xf(
                    source,
                    root,
                    target,
                    "cellStyleXfs",
                    base.parse().map_err(|_| "Invalid copied style")?,
                )?;
                updates.push(("xfId", Some(mapped.to_string())));
            }
        }
        let ed = parse(&entry)?;
        entry = crate::xlsx_tables::attributes(&entry, ed.root_element(), &updates);
        let (xml, mapped) = section(target, kind, &entry)?;
        *target = xml;
        Ok(mapped)
    }
    let mut xml = target.text("xl/styles.xml")?.to_string();
    let mapped = xf(
        source_xml,
        d.root_element(),
        &mut xml,
        "cellXfs",
        id.parse().map_err(|_| "Invalid copied format")?,
    )?;
    target.set("xl/styles.xml", xml);
    Ok(mapped.to_string())
}
