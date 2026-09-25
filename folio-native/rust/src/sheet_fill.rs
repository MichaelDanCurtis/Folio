//! Directional fill uses immutable edge cells, so overlapping destinations never
//! become the source for a later formula or formatting copy.
use crate::{
    formula::{address, parse_address, COLS, ROWS},
    office::{attr, child, esc, parse, raw, replace_ranges, Package},
    structure::copy_formula,
    tables::Area,
    xlsx::fragment_inner,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub struct Fill {
    pub sheet: usize,
    pub area: Area,
    pub copies: Vec<(String, String)>,
}

impl Fill {
    pub fn from_request(w: &Value, r: &Value) -> Result<Self, String> {
        let sheet = w["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .position(|s| s["id"] == r["sheetId"])
            .ok_or("Worksheet no longer exists")?;
        let mut area = Area::parse(r["range"].as_str().ok_or("Select cells to fill")?.trim())?;
        let direction = r["direction"].as_str().ok_or("Choose a fill direction")?;
        // A single row/column fills from its adjacent source edge, just as the
        // keyboard command does for a single cell. Keep the user's selection.
        match direction {
            "down" if area.top == area.bottom => {
                area.top = area
                    .top
                    .checked_sub(1)
                    .ok_or("There is no row above this selection")?
            }
            "up" if area.top == area.bottom => area.bottom += 1,
            "right" if area.left == area.right => {
                area.left = area
                    .left
                    .checked_sub(1)
                    .ok_or("There is no column left of this selection")?
            }
            "left" if area.left == area.right => area.right += 1,
            "down" | "up" | "right" | "left" => {}
            _ => return Err("Choose Down, Right, Up or Left".into()),
        }
        if area.bottom >= ROWS || area.right >= COLS {
            return Err("There is no source cell beyond the worksheet edge".into());
        }
        if (area.bottom - area.top + 1) * (area.right - area.left + 1) > 100_000 {
            return Err("Fill up to 100,000 cells at once".into());
        }
        let mut copies = vec![];
        // The fill handle repeats a whole source block across the range.
        if let Some(source) = r.get("source").filter(|v| !v.is_null()) {
            let source = Area::parse(source.as_str().ok_or("Invalid fill source")?.trim())?;
            let vertical = matches!(direction, "down" | "up");
            let inside = source.top >= area.top
                && source.bottom <= area.bottom
                && source.left >= area.left
                && source.right <= area.right;
            let aligned = if vertical {
                source.left == area.left && source.right == area.right
            } else {
                source.top == area.top && source.bottom == area.bottom
            };
            let edge = match direction {
                "down" => source.top == area.top,
                "up" => source.bottom == area.bottom,
                "right" => source.left == area.left,
                _ => source.right == area.right,
            };
            if !inside || !aligned || !edge {
                return Err(
                    "The fill range must extend the source cells in the fill direction".into(),
                );
            }
            let height = (source.bottom - source.top + 1) as isize;
            let width = (source.right - source.left + 1) as isize;
            for row in area.top..=area.bottom {
                for col in area.left..=area.right {
                    let from = (
                        source.top
                            + (row as isize - source.top as isize).rem_euclid(height) as usize,
                        source.left
                            + (col as isize - source.left as isize).rem_euclid(width) as usize,
                    );
                    if from != (row, col) {
                        copies.push((address(from.0, from.1), address(row, col)));
                    }
                }
            }
            return Ok(Self {
                sheet,
                area,
                copies,
            });
        }
        for row in area.top..=area.bottom {
            for col in area.left..=area.right {
                let source = match direction {
                    "down" => (area.top, col),
                    "up" => (area.bottom, col),
                    "right" => (row, area.left),
                    _ => (row, area.right),
                };
                if source != (row, col) {
                    copies.push((address(source.0, source.1), address(row, col)));
                }
            }
        }
        Ok(Self {
            sheet,
            area,
            copies,
        })
    }

    pub fn apply(&self, before: &Value) -> Result<Value, String> {
        let mut next = before.clone();
        let old = &before["sheets"][self.sheet];
        let names = before["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let sheet = &mut next["sheets"][self.sheet];
        let mut edits = vec![];
        let mut copied_bytes = 0;
        for (from, to) in &self.copies {
            let value = old["cells"][from].as_str().unwrap_or("");
            let value = if let Some(formula) = value.strip_prefix('=') {
                if parse_address(from)?.1 != parse_address(to)?.1
                    && !crate::table_refs::references(formula)?.is_empty()
                {
                    return Err("Horizontal fill of table-column references needs additional support. Nothing changed.".into());
                }
                format!("={}", copy_formula(formula, &names, self.sheet, from, to)?)
            } else {
                value.into()
            };
            copied_bytes += value.len() + old["cellStyles"][from].to_string().len() + 64;
            if copied_bytes > 32_000_000 {
                return Err(
                    "The filled content exceeds the workspace size limit. Nothing changed.".into(),
                );
            }
            sheet["cells"][to] = json!(value);
            for field in ["cellStyles", "officeStyles"] {
                if let Some(v) = old[field].get(from) {
                    if !sheet[field].is_object() {
                        sheet[field] = json!({});
                    }
                    sheet[field][to] = v.clone();
                } else if let Some(map) = sheet[field].as_object_mut() {
                    map.remove(to);
                }
            }
            if let Some(cache) = sheet["cached"].as_object_mut() {
                cache.remove(to);
            }
            edits.push(json!({"sheetId":old["id"],"address":to,"value":value}));
        }
        crate::sheet_reviews::copy_cells(
            &before["sheets"][self.sheet],
            &mut next["sheets"][self.sheet],
            &self.copies,
        )?;
        crate::tables::after_cells(before, &mut next, &edits)?;
        crate::data_validation::copy_rules(
            &before["sheets"],
            self.sheet,
            &mut next["sheets"][self.sheet],
            &self.copies,
            None,
            None,
        )?;
        crate::conditional_copy::copy_rules(
            &before["sheets"],
            self.sheet,
            &mut next["sheets"][self.sheet],
            &names,
            self.sheet,
            &self.copies,
            false,
        )?;
        Ok(next)
    }

    /// Copy full source cell records after pending edits are baked into a
    /// disposable package. Source formats keep their original XF, including
    /// properties the UI does not yet expose. Worksheet geometry stays put.
    pub fn copy_package(&self, pkg: &mut Package, sheets: &Value) -> Result<(), String> {
        let sheet = &sheets[self.sheet];
        let path = sheet["officePath"]
            .as_str()
            .ok_or("Worksheet source missing")?;
        let xml = pkg.text(path)?.to_string();
        let d = parse(&xml)?;
        let root = d.root_element();
        if child(root, "sheetProtection").is_some() {
            return Err(
                "Filling a protected worksheet needs protection support. Nothing changed.".into(),
            );
        }
        for n in root.descendants().filter(|n| n.is_element()) {
            if n.tag_name().name() == "mergeCell"
                || n.tag_name().name() == "f" && matches!(attr(n, "t"), Some("array" | "dataTable"))
            {
                if attr(n, "ref")
                    .is_some_and(|r| Area::parse(r).is_ok_and(|a| a.overlaps(self.area)))
                {
                    return Err("Filling merged cells or array/data-table formulas needs additional support. Nothing changed.".into());
                }
            }
        }
        let data = child(root, "sheetData").ok_or("Worksheet data missing")?;
        let names = sheets
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let mut contents: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
        let mut row_nodes = BTreeMap::new();
        let mut source = HashMap::new();
        let mut row_styles = HashMap::new();
        let mut columns = vec![];
        if let Some(cols) = child(root, "cols") {
            for col in cols.children().filter(|n| n.is_element()) {
                if let (Some(min), Some(max), Some(style)) =
                    (attr(col, "min"), attr(col, "max"), attr(col, "style"))
                {
                    columns.push((
                        min.parse::<usize>().map_err(|_| "Invalid column")?,
                        max.parse::<usize>().map_err(|_| "Invalid column")?,
                        style,
                    ));
                }
            }
        }
        for row in data
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "row")
        {
            let r = attr(row, "r")
                .and_then(|r| r.parse::<usize>().ok())
                .filter(|r| *r > 0)
                .ok_or("Invalid worksheet row")?
                - 1;
            row_nodes.insert(r, row);
            if matches!(attr(row, "customFormat"), Some("1" | "true")) {
                if let Some(style) = attr(row, "s") {
                    row_styles.insert(r, style);
                }
            }
            for cell in row
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "c")
            {
                let a = attr(cell, "r").ok_or("Cell address missing")?;
                let (_, c) = parse_address(a)?;
                let normalized =
                    if let Some(f) = child(cell, "f").filter(|f| attr(*f, "t") == Some("shared")) {
                        let formula = sheet["cells"][a]
                            .as_str()
                            .and_then(|v| v.strip_prefix('='))
                            .ok_or("Shared formula projection missing")?;
                        let offset = cell.range().start;
                        replace_ranges(
                            raw(&xml, cell),
                            vec![(
                                f.range().start - offset..f.range().end - offset,
                                format!("<f xmlns=\"{S}\">{}</f>", esc(formula)),
                            )],
                        )
                    } else {
                        raw(&xml, cell).to_string()
                    };
                contents.entry(r).or_default().insert(c, normalized.clone());
                source.insert(a, (cell, normalized));
            }
        }
        let mut copied_xml_bytes = xml.len();
        for (from, to) in &self.copies {
            let (r, c) = parse_address(to)?;
            let (sr, sc) = parse_address(from)?;
            let style = source
                .get(from.as_str())
                .and_then(|(n, _)| attr(*n, "s"))
                .or_else(|| row_styles.get(&sr).copied())
                .or_else(|| {
                    columns
                        .iter()
                        .rev()
                        .find(|(min, max, _)| sc + 1 >= *min && sc + 1 <= *max)
                        .map(|(_, _, s)| *s)
                })
                .unwrap_or("0");
            let mut copied = if let Some((node, text)) = source.get(from.as_str()) {
                // Attribute rewriting uses source ranges before normalization;
                // instead wrap the complete normalized record with its source
                // namespace declarations, then parse it on its own.
                let mut namespaces = String::new();
                for ns in node.namespaces() {
                    namespaces.push_str(&format!(
                        " xmlns{}=\"{}\"",
                        ns.name().map(|n| format!(":{n}")).unwrap_or_default(),
                        esc(ns.uri())
                    ));
                }
                let wrapped = format!("<root{namespaces}>{text}</root>");
                let parsed = parse(&wrapped)?;
                let n = parsed
                    .root_element()
                    .first_element_child()
                    .ok_or("Cell missing")?;
                let tag = text[1..]
                    .split(|c: char| c.is_whitespace() || matches!(c, '/' | '>'))
                    .next()
                    .unwrap();
                let mut attrs = String::new();
                for a in n
                    .attributes()
                    .filter(|a| a.namespace().is_some() || !matches!(a.name(), "r" | "s"))
                {
                    let prefix = a
                        .namespace()
                        .and_then(|uri| {
                            if uri == "http://www.w3.org/XML/1998/namespace" {
                                return Some("xml");
                            }
                            n.namespaces()
                                .find(|ns| ns.uri() == uri && ns.name().is_some())
                                .and_then(|ns| ns.name())
                        })
                        .map(|p| format!("{p}:"))
                        .unwrap_or_default();
                    attrs.push_str(&format!(" {prefix}{}=\"{}\"", a.name(), esc(a.value())));
                }
                let inner = n
                    .children()
                    .map(|child| raw(&wrapped, child))
                    .collect::<String>();
                // Make inherited row-level namespace declarations explicit:
                // the destination may be in a different row or a new row.
                format!("<{tag}{namespaces}{attrs} r=\"{to}\" s=\"{style}\">{inner}</{tag}>")
            } else {
                format!("<c xmlns=\"{S}\" r=\"{to}\" s=\"{style}\"/>")
            };
            if let Some(formula) = sheet["cells"][from]
                .as_str()
                .and_then(|v| v.strip_prefix('='))
            {
                let formula = copy_formula(formula, &names, self.sheet, from, to)?;
                // Formula cells keep source style/metadata but use a standalone
                // translated expression and no stale numeric cache.
                let extra = source
                    .get(from.as_str())
                    .map(|(node, _)| {
                        node.children()
                            .filter(|n| {
                                n.is_element() && !matches!(n.tag_name().name(), "f" | "v" | "is")
                            })
                            .map(|n| raw(&xml, n))
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                copied = fragment_inner(
                    &copied,
                    &format!("<f xmlns=\"{S}\">{}</f>{extra}", esc(&formula)),
                )?;
            }
            copied_xml_bytes += copied.len();
            if copied_xml_bytes > 64 * 1024 * 1024 {
                return Err(
                    "The filled worksheet exceeds the Office file size limit. Nothing changed."
                        .into(),
                );
            }
            contents.entry(r).or_default().insert(c, copied);
        }
        // Empty source rows still retain their geometry and extension children.
        for r in row_nodes.keys() {
            contents.entry(*r).or_default();
        }
        let mut rows = String::new();
        for (r, cells) in contents {
            let mut inner = cells.values().cloned().collect::<String>();
            if let Some(row) = row_nodes.get(&r) {
                for n in row
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() != "c")
                {
                    inner.push_str(raw(&xml, n));
                }
                rows.push_str(&fragment_inner(raw(&xml, *row), &inner)?);
            } else {
                rows.push_str(&format!("<row xmlns=\"{S}\" r=\"{}\">{inner}</row>", r + 1));
            }
        }
        pkg.set(
            path,
            replace_ranges(
                &xml,
                vec![(data.range(), fragment_inner(raw(&xml, data), &rows)?)],
            ),
        );
        let mut target = sheet.clone();
        crate::data_validation::copy_rules(
            sheets,
            self.sheet,
            &mut target,
            &self.copies,
            Some(pkg),
            None,
        )?;
        crate::data_validation::write(pkg, &target, Some(sheet), path)?;
        crate::conditional_copy::copy_rules(
            sheets,
            self.sheet,
            &mut target,
            &names,
            self.sheet,
            &self.copies,
            false,
        )?;
        crate::conditional_format::write(pkg, &target, Some(sheet), path)?;
        pkg.original = pkg.bytes()?;
        Ok(())
    }
}
