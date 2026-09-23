//! Logical merged-cell tables and source-preserving Word table serialization.
use crate::{docx, docx_numbering::Catalog, docx_table_format as fmt, office::*};
use serde_json::{json, Value};
use std::collections::HashMap;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub row: usize,
    pub index: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}
pub fn grid(table: &Value) -> Result<(usize, Vec<Cell>), String> {
    fmt::validate(table)?;
    let rows = table["content"]
        .as_array()
        .ok_or("Table rows are missing")?;
    if rows.is_empty() || rows.len() > 1000 {
        return Err("Tables need 1–1,000 rows".into());
    }
    let mut occupied = vec![vec![false; 1000]; rows.len()];
    let mut width = 0;
    let mut out = vec![];
    for (r, row) in rows.iter().enumerate() {
        if row["type"] != "tableRow" {
            return Err("Invalid table row".into());
        }
        fmt::validate(row)?;
        let mut c = 0;
        for (index, cell) in row["content"]
            .as_array()
            .ok_or("Table cells are missing")?
            .iter()
            .enumerate()
        {
            if cell["type"] != "tableCell" && cell["type"] != "tableHeader" {
                return Err("Invalid table cell".into());
            }
            fmt::validate(cell)?;
            if let Some(continuations) = cell["attrs"].get("mergeContinuations") {
                for continuation in continuations
                    .as_object()
                    .ok_or("Invalid merge continuations")?
                    .values()
                {
                    if continuation["type"] != "tableCell" && continuation["type"] != "tableHeader"
                    {
                        return Err("Invalid merge continuation".into());
                    }
                    fmt::validate(continuation)?;
                }
            }
            while c < 1000 && occupied[r][c] {
                c += 1;
            }
            let span = |name: &str| -> Result<usize, String> {
                match cell["attrs"].get(name) {
                    None => Ok(1),
                    Some(v) => v
                        .as_u64()
                        .filter(|v| *v > 0 && *v <= 1000)
                        .map(|v| v as usize)
                        .ok_or("Invalid table cell span".into()),
                }
            };
            let cs = span("colspan")?;
            let rs = span("rowspan")?;
            if c + cs > 1000 || r + rs > rows.len() {
                return Err("Table cell span exceeds the table".into());
            }
            for line in occupied.iter_mut().skip(r).take(rs) {
                for slot in line.iter_mut().skip(c).take(cs) {
                    if *slot {
                        return Err("Table cells overlap".into());
                    }
                    *slot = true;
                }
            }
            out.push(Cell {
                row: r,
                index,
                col: c,
                rows: rs,
                cols: cs,
            });
            c += cs;
            width = width.max(c);
        }
    }
    if width == 0 || occupied.iter().any(|row| row[..width].iter().any(|v| !*v)) {
        return Err("The table has uncovered grid positions".into());
    }
    if let Some(widths) = table["attrs"].get("columnWidths") {
        let widths = widths.as_array().ok_or("Invalid table column widths")?;
        if widths.len() != width
            || widths.iter().any(|v| {
                v.as_f64()
                    .is_none_or(|n| !n.is_finite() || n <= 0.0 || n > 31680.0)
            })
        {
            return Err("Invalid table column widths".into());
        }
    }
    Ok((width, out))
}

pub fn read(
    node: roxmltree::Node,
    rels: &HashMap<String, String>,
    numbering: &Catalog,
    tabs: &crate::docx_tabs::Catalog,
    id: &str,
) -> Option<Value> {
    // Row omissions and legacy horizontal merge groups need a distinct layout.
    if node.descendants().any(|n| {
        let allowed = match n.tag_name().name() {
            "tbl" => Some(&["tblPr", "tblGrid", "tr"][..]),
            "tr" => Some(&["trPr", "tc"][..]),
            "tc" => Some(&["tcPr", "p", "tbl"][..]),
            _ => None,
        };
        allowed.is_some_and(|names| {
            n.children().any(|c| {
                c.is_element()
                    && (!names.contains(&c.tag_name().name())
                        || c.tag_name().namespace() != Some(W))
            })
        })
    }) {
        return None;
    }
    if node.descendants().any(|n| {
        matches!(
            n.tag_name().name(),
            "gridBefore" | "gridAfter" | "hMerge" | "tblPrChange" | "trPrChange" | "tcPrChange"
        )
    }) {
        return None;
    }
    let mut rows: Vec<Value> = vec![];
    let mut active: HashMap<usize, (usize, usize, usize)> = HashMap::new();
    for (r, row) in node
        .children()
        .filter(|n| n.has_tag_name((W, "tr")))
        .enumerate()
    {
        let row_id = format!("{id}-{r}");
        let mut cells: Vec<Value> = vec![];
        let mut next = HashMap::new();
        let mut col = 0;
        for (c, cell) in row
            .children()
            .filter(|n| n.has_tag_name((W, "tc")))
            .enumerate()
        {
            let pr = child(cell, "tcPr");
            let span = pr
                .and_then(|n| child(n, "gridSpan"))
                .and_then(|n| attr(n, "val"))
                .map(str::parse::<usize>)
                .transpose()
                .ok()?
                .unwrap_or(1);
            if span == 0 || span > 1000 {
                return None;
            }
            let content: Vec<_> = cell
                .children()
                .filter(|n| n.has_tag_name((W, "p")) || n.has_tag_name((W, "tbl")))
                .enumerate()
                .map(|(p, n)| {
                    docx::read_block(n, rels, numbering, tabs, &format!("{id}-{r}-{c}-{p}"))
                })
                .collect();
            let mut value = json!({"type":"tableCell","attrs":{"officeId":format!("{id}-{r}-{c}"),"colspan":span,"rowspan":1},"content":content});
            fmt::read(cell, &mut value);
            let merge = pr
                .and_then(|n| child(n, "vMerge"))
                .map(|n| attr(n, "val").unwrap_or("continue"));
            if merge == Some("continue") {
                // Nonempty continuation content must remain visible/preserved.
                if cell
                    .children()
                    .filter(|n| n.has_tag_name((W, "p")) || n.has_tag_name((W, "tbl")))
                    .any(|p| {
                        !p.has_tag_name((W, "p"))
                            || p.children()
                                .any(|n| n.is_element() && !n.has_tag_name((W, "pPr")))
                    })
                {
                    return None;
                }
                let &(ar, ac, width) = active.get(&col)?;
                if width != span {
                    return None;
                }
                let anchor = &mut rows[ar]["content"][ac];
                let height = anchor["attrs"]["rowspan"].as_u64()? + 1;
                anchor["attrs"]["rowspan"] = json!(height);
                if !anchor["attrs"]["mergeContinuations"].is_object() {
                    anchor["attrs"]["mergeContinuations"] = json!({});
                }
                anchor["attrs"]["mergeContinuations"][&row_id] = value;
                next.insert(col, (ar, ac, width));
            } else {
                if merge.is_some() && merge != Some("restart") {
                    return None;
                }
                if merge == Some("restart") {
                    next.insert(col, (r, cells.len(), span));
                }
                cells.push(value);
            }
            col += span;
        }
        let mut value = json!({"type":"tableRow","attrs":{"officeId":row_id},"content":cells});
        fmt::read(row, &mut value);
        tabs.table_rows.apply(node, row, r, &mut value);
        rows.push(value);
        active = next;
    }
    let mut table = json!({"type":"table","attrs":{"officeId":id},"content":rows});
    let (cols, _) = grid(&table).ok()?;
    let widths: Vec<_> = child(node, "tblGrid")
        .map(|g| {
            g.children()
                .filter(|n| n.has_tag_name((W, "gridCol")))
                .filter_map(|n| attr(n, "w").and_then(|n| n.parse::<u64>().ok()))
                .collect()
        })
        .unwrap_or_default();
    if widths.len() == cols && widths.iter().all(|n| *n > 0 && *n <= 31680) {
        table["attrs"]["columnWidths"] = json!(widths);
    }
    fmt::read(node, &mut table);
    Some(table)
}

pub fn source_map<'a>(
    xml: &'a str,
    node: roxmltree::Node,
    id: String,
    out: &mut HashMap<String, &'a str>,
) {
    out.insert(id.clone(), raw(xml, node));
    let kind = node.tag_name().name();
    if kind == "tbl" {
        for (i, n) in node
            .children()
            .filter(|n| n.has_tag_name((W, "tr")))
            .enumerate()
        {
            source_map(xml, n, format!("{id}-{i}"), out);
        }
    }
    if kind == "tr" {
        for (i, n) in node
            .children()
            .filter(|n| n.has_tag_name((W, "tc")))
            .enumerate()
        {
            source_map(xml, n, format!("{id}-{i}"), out);
        }
    }
    if kind == "tc" {
        for (i, n) in node
            .children()
            .filter(|n| n.has_tag_name((W, "p")) || n.has_tag_name((W, "tbl")))
            .enumerate()
        {
            source_map(xml, n, format!("{id}-{i}"), out);
        }
    }
}

pub fn collect_old<'a>(
    node: &'a Value,
    sources: &HashMap<String, &'a str>,
    old: &mut HashMap<&'a str, (&'a Value, &'a str)>,
) {
    if let Some(id) = node["attrs"]["officeId"].as_str() {
        if let Some(xml) = sources.get(id) {
            old.insert(id, (node, *xml));
        }
    }
    match node {
        Value::Object(m) => {
            for value in m.values() {
                collect_old(value, sources, old)
            }
        }
        Value::Array(a) => {
            for value in a {
                collect_old(value, sources, old)
            }
        }
        _ => {}
    }
}

fn source<'a>(node: &Value, old: &HashMap<&str, (&Value, &'a str)>) -> Option<&'a str> {
    node["attrs"]["officeId"]
        .as_str()
        .and_then(|id| old.get(id))
        .map(|v| v.1)
}
fn property(xml: Option<&str>, namespaces: &str, name: &str) -> String {
    let Some(xml) = xml else { return String::new() };
    let wrapped = docx::wrap_fragment(xml, namespaces);
    let Ok(d) = parse(&wrapped) else {
        return String::new();
    };
    d.root_element()
        .children()
        .find(|n| n.is_element())
        .and_then(|n| n.children().find(|n| n.has_tag_name((W, name))))
        .map(|n| raw(&wrapped, n).to_string())
        .unwrap_or_default()
}
fn cell_props(
    xml: Option<&str>,
    namespaces: &str,
    span: usize,
    merge: Option<&str>,
    width: Option<u64>,
) -> String {
    let props = property(xml, namespaces, "tcPr");
    let wrapped = docx::wrap_fragment(&props, namespaces);
    let d = parse(&wrapped).unwrap();
    let order = [
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
    ];
    let mut values = vec![];
    let original = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, "tcPr")));
    let old_span = original
        .and_then(|pr| child(pr, "gridSpan"))
        .and_then(|n| attr(n, "val"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let old_merge = original
        .and_then(|pr| child(pr, "vMerge"))
        .map(|n| attr(n, "val").unwrap_or("continue"));
    // A row-only setting must not normalize cell merge properties or discard
    // extension attributes on an unchanged gridSpan/vMerge element.
    if width.is_none() && old_span == span && old_merge == merge {
        return props;
    }
    if let Some(pr) = original {
        for n in pr.children() {
            if !(old_span != span && n.has_tag_name((W, "gridSpan")))
                && !(old_merge != merge && n.has_tag_name((W, "vMerge")))
                && !(width.is_some() && n.has_tag_name((W, "tcW")))
            {
                values.push((
                    order
                        .iter()
                        .position(|v| *v == n.tag_name().name())
                        .unwrap_or(usize::MAX),
                    raw(&wrapped, n).to_string(),
                ));
            }
        }
    }
    if let Some(width) = width {
        values.push((1, format!("<w:tcW w:w=\"{width}\" w:type=\"dxa\"/>")));
    }
    if old_span != span && span > 1 {
        values.push((2, format!("<w:gridSpan w:val=\"{span}\"/>")));
    }
    if let Some(mode) = merge.filter(|_| old_merge != merge) {
        values.push((4, format!("<w:vMerge w:val=\"{mode}\"/>")));
    }
    values.sort_by_key(|v| v.0);
    fmt::wrap_properties("tcPr", original, values.into_iter().map(|v| v.1).collect())
}

pub fn write(
    node: &Value,
    old: &HashMap<&str, (&Value, &str)>,
    links: &HashMap<String, String>,
    namespaces: &str,
) -> String {
    let Ok((cols, cells)) = grid(node) else {
        return docx::paragraph(&crate::workspace::document_text(node), "");
    };
    let rows = node["content"].as_array().unwrap();
    let old_xml = source(node, old);
    let props = property(old_xml, namespaces, "tblPr");
    let props = if props.is_empty() {
        "<w:tblPr><w:tblBorders><w:top w:val=\"single\" w:sz=\"4\"/><w:left w:val=\"single\" w:sz=\"4\"/><w:bottom w:val=\"single\" w:sz=\"4\"/><w:right w:val=\"single\" w:sz=\"4\"/><w:insideH w:val=\"single\" w:sz=\"4\"/><w:insideV w:val=\"single\" w:sz=\"4\"/></w:tblBorders></w:tblPr>".to_string()
    } else {
        props
    };
    let before = node["attrs"]["officeId"]
        .as_str()
        .and_then(|id| old.get(id))
        .map(|v| v.0);
    let changed_width = node["attrs"]["layout"] == "fixed"
        && before.is_none_or(|b| {
            b["attrs"]["columnWidths"] != node["attrs"]["columnWidths"]
                || b["attrs"]["layout"] != node["attrs"]["layout"]
        });
    let preferred_width = changed_width.then(|| {
        (0..cols)
            .map(|c| {
                node["attrs"]["columnWidths"][c]
                    .as_f64()
                    .unwrap_or(2400.0)
                    .round() as u64
            })
            .sum()
    });
    let props = fmt::patch(props, namespaces, "tblPr", node, before, preferred_width);
    let original_grid = property(old_xml, namespaces, "tblGrid");
    let grid = if !original_grid.is_empty()
        && before.is_some_and(|b| {
            b["attrs"]["columnWidths"] == node["attrs"]["columnWidths"]
                && self::grid(b).is_ok_and(|g| g.0 == cols)
        }) {
        original_grid
    } else {
        format!(
            "<w:tblGrid>{}</w:tblGrid>",
            (0..cols)
                .map(|c| format!(
                    "<w:gridCol w:w=\"{}\"/>",
                    node["attrs"]["columnWidths"][c]
                        .as_f64()
                        .unwrap_or(2400.0)
                        .round() as u64
                ))
                .collect::<String>()
        )
    };
    let mut body = String::new();
    for (r, row) in rows.iter().enumerate() {
        let row_before = row["attrs"]["officeId"]
            .as_str()
            .and_then(|id| old.get(id))
            .map(|v| v.0);
        // A row permutation must retain the complete original row, including
        // wrapper attributes and unmodeled cell content. Width changes and
        // vertical continuations still require geometry-aware reconstruction.
        if row_before == Some(row)
            && before.is_some_and(|b| b["attrs"]["columnWidths"] == node["attrs"]["columnWidths"])
            && cells
                .iter()
                .filter(|c| c.row <= r && c.row + c.rows > r)
                .all(|c| c.rows == 1)
        {
            if let Some(xml) = source(row, old) {
                body.push_str(xml);
                continue;
            }
        }
        let pr = fmt::patch(
            property(source(row, old), namespaces, "trPr"),
            namespaces,
            "trPr",
            row,
            row_before,
            None,
        );
        body.push_str(&format!("<w:tr>{pr}"));
        let mut line: Vec<_> = cells
            .iter()
            .filter(|c| c.row <= r && c.row + c.rows > r)
            .collect();
        line.sort_by_key(|c| c.col);
        for position in line {
            let anchor = &rows[position.row]["content"][position.index];
            let continuation = position.row < r;
            let empty = json!({"type":"tableCell","content":[{"type":"paragraph"}]});
            let cell = if continuation {
                row["attrs"]["officeId"]
                    .as_str()
                    .and_then(|id| anchor["attrs"]["mergeContinuations"].get(id))
                    .unwrap_or(&empty)
            } else {
                anchor
            };
            let cell_before = cell["attrs"]["officeId"]
                .as_str()
                .and_then(|id| old.get(id))
                .map(|v| v.0);
            let width_changed = cell_before.is_none_or(|b| {
                b["attrs"]["colspan"].as_u64().unwrap_or(1) != position.cols as u64
            }) || before
                .is_none_or(|b| b["attrs"]["columnWidths"] != node["attrs"]["columnWidths"]);
            let width = width_changed.then(|| {
                (position.col..position.col + position.cols)
                    .map(|c| {
                        node["attrs"]["columnWidths"][c]
                            .as_f64()
                            .unwrap_or(2400.0)
                            .round() as u64
                    })
                    .sum()
            });
            let pr = cell_props(
                source(cell, old),
                namespaces,
                position.cols,
                if position.rows == 1 {
                    None
                } else if continuation {
                    Some("continue")
                } else {
                    Some("restart")
                },
                width,
            );
            // Formatting an anchor applies to its entire vertical merge. Leave
            // source continuation formatting alone unless the anchor changed it.
            let anchor_before = anchor["attrs"]["officeId"]
                .as_str()
                .and_then(|id| old.get(id))
                .map(|v| v.0);
            let mut format = cell.clone();
            if continuation {
                for key in fmt::CELL_KEYS {
                    if let Some(value) = anchor["attrs"].get(*key) {
                        if anchor_before.is_none_or(|b| b["attrs"][*key] != *value) {
                            if !format["attrs"].is_object() {
                                format["attrs"] = json!({});
                            }
                            format["attrs"][*key] = value.clone();
                        }
                    }
                }
            }
            let pr = fmt::patch(pr, namespaces, "tcPr", &format, cell_before, None);
            let mut text = cell["content"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|n| docx::write_block(n, old, links, namespaces))
                        .collect::<String>()
                })
                .unwrap_or_default();
            if text.is_empty()
                || cell["content"]
                    .as_array()
                    .and_then(|a| a.last())
                    .is_some_and(|n| n["type"] == "table")
            {
                text.push_str("<w:p/>");
            }
            body.push_str(&format!("<w:tc>{pr}{text}</w:tc>"));
        }
        body.push_str("</w:tr>");
    }
    format!("<w:tbl xmlns:w=\"{W}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"{namespaces}>{props}{grid}{body}</w:tbl>")
}

// Before structural table editing existed, saved tables lacked row/cell ids.
// Hydrate only identical grid geometry, preserving all authored text/content.
pub fn hydrate(document: &mut Value, source: &Value) {
    fn collect<'a>(v: &'a Value, out: &mut HashMap<&'a str, &'a Value>) {
        if v["type"] == "table" {
            if let Some(id) = v["attrs"]["officeId"].as_str() {
                out.insert(id, v);
            }
        }
        if let Some(a) = v["content"].as_array() {
            for n in a {
                collect(n, out);
            }
        }
    }
    fn visit(v: &mut Value, tables: &HashMap<&str, &Value>) {
        if v["type"] == "table" {
            if let Some(original) = v["attrs"]["officeId"]
                .as_str()
                .and_then(|id| tables.get(id))
            {
                if let (Ok((cols, cells)), Ok((source_cols, source_cells))) =
                    (grid(v), grid(original))
                {
                    if cols == source_cols
                        && cells.len() == source_cells.len()
                        && cells.iter().zip(source_cells.iter()).all(|(a, b)| {
                            (a.row, a.col, a.rows, a.cols) == (b.row, b.col, b.rows, b.cols)
                        })
                    {
                        fmt::hydrate(v, original, fmt::TABLE_KEYS);
                        if v["attrs"].get("columnWidths").is_none() {
                            if let Some(widths) = original["attrs"].get("columnWidths") {
                                v["attrs"]["columnWidths"] = widths.clone();
                            }
                        }
                        for (a, b) in cells.iter().zip(source_cells.iter()) {
                            let r = &mut v["content"][a.row];
                            if !r["attrs"].is_object() {
                                r["attrs"] = json!({});
                            }
                            if r["attrs"].get("officeId").is_none() {
                                r["attrs"]["officeId"] =
                                    original["content"][b.row]["attrs"]["officeId"].clone();
                            }
                            fmt::hydrate(r, &original["content"][b.row], fmt::ROW_KEYS);
                            let cell = &mut r["content"][a.index];
                            if !cell["attrs"].is_object() {
                                cell["attrs"] = json!({});
                            }
                            fmt::hydrate(
                                cell,
                                &original["content"][b.row]["content"][b.index],
                                fmt::CELL_KEYS,
                            );
                            if cell["attrs"].get("officeId").is_none() {
                                cell["attrs"]["officeId"] = original["content"][b.row]["content"]
                                    [b.index]["attrs"]["officeId"]
                                    .clone();
                            }
                        }
                    }
                }
            }
        }
        if let Some(a) = v.get_mut("content").and_then(Value::as_array_mut) {
            for n in a {
                visit(n, tables);
            }
        }
    }
    let mut tables = HashMap::new();
    collect(source, &mut tables);
    visit(document, &tables);
}
