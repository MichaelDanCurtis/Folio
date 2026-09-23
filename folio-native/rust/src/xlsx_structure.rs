//! Apply a structural edit to retained OOXML parts before establishing a new
//! editable baseline. The original package remains available to workspace Undo.
use crate::{
    formula::parse_address,
    office::*,
    structure::{Change, Edit},
};
use roxmltree::Node;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Workbook,
    Worksheet(usize),
    Comments(usize),
    Drawing(usize),
    Vml(usize),
    Chart,
    Table(usize),
}
impl Part {
    fn sheet(self) -> Option<usize> {
        match self {
            Self::Worksheet(i)
            | Self::Comments(i)
            | Self::Drawing(i)
            | Self::Vml(i)
            | Self::Table(i) => Some(i),
            _ => None,
        }
    }
}

fn related_path(owner: &str, target: &str) -> Result<String, String> {
    let mut parts: Vec<&str> = if target.starts_with('/') {
        vec![]
    } else {
        owner
            .rsplit_once('/')
            .map(|p| p.0.split('/').collect())
            .unwrap_or_default()
    };
    for part in target.trim_start_matches('/').split('/') {
        match part {
            ".." => {
                parts.pop().ok_or("Invalid part relationship")?;
            }
            "." => {}
            _ => parts.push(part),
        }
    }
    Ok(parts.join("/"))
}
pub(crate) fn relationships(
    pkg: &Package,
    path: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let (dir, name) = path.rsplit_once('/').ok_or("Invalid part path")?;
    let rel = format!("{dir}/_rels/{name}.rels");
    let Ok(xml) = pkg.text(&rel) else {
        return Ok(vec![]);
    };
    let d = parse(xml)?;
    d.descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().name() == "Relationship"
                && attr(*n, "TargetMode") != Some("External")
        })
        .map(|n| {
            Ok((
                attr(n, "Id").unwrap_or("").into(),
                attr(n, "Type")
                    .unwrap_or("")
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .into(),
                related_path(
                    path,
                    attr(n, "Target").ok_or("Missing relationship target")?,
                )?,
            ))
        })
        .collect()
}

pub fn transform(pkg: &mut Package, change: &Change) -> Result<(), String> {
    if !matches!(change.edit, Edit::Move(_))
        && pkg.entries.keys().any(|p| {
            p.starts_with("xl/pivot")
                || p.starts_with("xl/externalLinks/")
                || p.ends_with("vbaProject.bin")
        })
    {
        return Err("Structural editing of workbooks with pivots, external workbook links or macros is not supported yet. Nothing changed.".into());
    }
    let relations = relationships(pkg, "xl/workbook.xml")?;
    let wb = parse(pkg.text("xl/workbook.xml")?)?;
    let mut parts = HashMap::new();
    parts.insert("xl/workbook.xml".to_string(), Part::Workbook);
    for (i, sheet) in wb
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sheet")
        .enumerate()
    {
        let rid = attr(sheet, "id").ok_or("Missing worksheet relationship")?;
        let path = &relations
            .iter()
            .find(|r| r.0 == rid)
            .ok_or("Missing worksheet part")?
            .2;
        parts.insert(path.clone(), Part::Worksheet(i));
        for (_, kind, target) in relationships(pkg, path)? {
            let part = match kind.as_str() {
                "comments" | "threadedComment" => Some(Part::Comments(i)),
                "drawing" => Some(Part::Drawing(i)),
                "vmlDrawing" => Some(Part::Vml(i)),
                "table" => Some(Part::Table(i)),
                _ => None,
            };
            if let Some(part) = part {
                parts.insert(target, part);
            }
        }
    }
    for path in pkg
        .entries
        .keys()
        .filter(|p| p.starts_with("xl/charts/") && p.ends_with(".xml"))
    {
        parts.insert(path.clone(), Part::Chart);
    }
    let owners = parts
        .iter()
        .filter(|(_, part)| matches!(part, Part::Worksheet(_)))
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    if matches!(change.edit, Edit::Axis { .. }) {
        for owner in owners {
            crate::xlsx_reviews::normalize_anchors(pkg, &owner)?;
        }
    }
    for (path, part) in parts {
        if matches!(change.edit, Edit::Move(_)) && part != Part::Workbook {
            continue;
        }
        // Deleted sheet parts remain unreachable from the workbook's sheet list.
        // Retain those opaque parts; Undo uses the complete original package.
        if matches!(change.edit, Edit::Delete) && part.sheet() == Some(change.sheet) {
            continue;
        }
        let mut xml = pkg.text(&path)?.to_string();
        if let Part::Worksheet(sheet) = part {
            let d = parse(&xml)?;
            let masters = crate::xlsx::shared_formulas(&d);
            let mut replacements = vec![];
            for f in d.descendants().filter(|n| {
                n.is_element() && n.tag_name().name() == "f" && attr(*n, "t") == Some("shared")
            }) {
                let (from, expr) = masters
                    .get(attr(f, "si").ok_or("Shared formula index missing")?)
                    .ok_or("Shared formula master missing")?;
                let to = attr(f.parent().ok_or("Missing formula cell")?, "r")
                    .ok_or("Missing formula address")?;
                let expanded =
                    crate::structure::copy_formula(expr, &change.names, sheet, from, to)?;
                let tag = raw(&xml, f)[1..].split([' ', '/', '>']).next().unwrap();
                replacements.push((f.range(), format!("<{tag}>{}</{tag}>", esc(&expanded))));
            }
            xml = replace_ranges(&xml, replacements);
        }
        let d = parse(&xml)?;
        if part == Part::Worksheet(change.sheet) {
            if let Edit::Axis {
                rows,
                insert,
                at,
                count,
            } = change.edit
            {
                for f in d
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "f")
                {
                    if attr(f, "t") == Some("dataTable") {
                        return Err("Structural editing of what-if data tables is not supported yet. Nothing changed.".into());
                    }
                    if attr(f, "t") == Some("array") {
                        let range = attr(f, "ref").ok_or("Missing array range")?;
                        let ends: Vec<_> = range.split(':').collect();
                        let a = parse_address(ends[0])?;
                        let b = parse_address(ends[ends.len() - 1])?;
                        let (lo, hi) = if rows {
                            (a.0 as i32 + 1, b.0 as i32 + 1)
                        } else {
                            (a.1 as i32 + 1, b.1 as i32 + 1)
                        };
                        let partial = if insert {
                            at > lo && at <= hi
                        } else {
                            at <= hi && at + count > lo && !(at <= lo && at + count > hi)
                        };
                        if partial {
                            return Err(
                                "You cannot change part of an array formula. Nothing changed."
                                    .into(),
                            );
                        }
                    }
                }
            }
        }
        let mut conditional = serde_json::json!({});
        if part == Part::Worksheet(change.sheet) && matches!(change.edit, Edit::Axis { .. }) {
            crate::conditional_format::read(pkg, &path, &mut conditional)?;
        }
        let movable_bars = conditional["extendedConditionalFormats"] == false
            && crate::conditional_format::rules(&conditional).all(|r| r["unsupported"] != true);
        if part == Part::Worksheet(change.sheet)
            && matches!(change.edit, Edit::Axis { .. })
            && d.descendants().any(|n| {
                n.is_element()
                    && n.tag_name().name() == "extLst"
                    && n.children().any(|e| {
                        e.is_element()
                            && !crate::sheet_filter::is_visibility_extension(e)
                            && !(movable_bars && crate::conditional_data_bar::known_extension(e))
                    })
            })
        {
            return Err("This worksheet contains extension ranges that cannot yet be moved safely. Nothing changed.".into());
        }
        let result = rewrite(&xml, d.root_element(), part, change)?;
        pkg.set(
            &path,
            replace_ranges(&xml, vec![(d.root_element().range(), result)]),
        );
        if part == Part::Worksheet(change.sheet) && matches!(change.edit, Edit::Axis { .. }) {
            crate::sheet_view::repair_topology(pkg, &path)?;
        }
    }
    // A calculation chain is an obsolete address index after a structural edit.
    pkg.entries.remove("xl/calcChain.xml");
    for path in ["xl/_rels/workbook.xml.rels", "[Content_Types].xml"] {
        let xml = pkg.text(path)?.to_string();
        let d = parse(&xml)?;
        let mut removals = vec![];
        for n in d.descendants().filter(|n| n.is_element()) {
            if attr(n, "Type").is_some_and(|v| v.ends_with("/calcChain"))
                || attr(n, "PartName") == Some("/xl/calcChain.xml")
            {
                removals.push((n.range(), String::new()));
            }
        }
        pkg.set(path, replace_ranges(&xml, removals));
    }
    crate::xlsx::force_recalculation(pkg)
}

fn reindex(change: &Change, index: usize) -> Option<usize> {
    match change.edit {
        Edit::Delete if index == change.sheet => None,
        Edit::Delete if index > change.sheet => Some(index - 1),
        Edit::Move(to) if index == change.sheet => Some(to),
        Edit::Move(to) if change.sheet < to && index > change.sheet && index <= to => {
            Some(index - 1)
        }
        Edit::Move(to) if to < change.sheet && index >= to && index < change.sheet => {
            Some(index + 1)
        }
        _ => Some(index),
    }
}

fn mapped_ranges(change: &Change, text: &str) -> Result<String, String> {
    let mut result = vec![];
    for range in text.split_whitespace() {
        if let Some(range) = change.range(range)? {
            result.push(range);
        }
    }
    Ok(result.join(" "))
}

fn rewrite(xml: &str, node: Node, part: Part, change: &Change) -> Result<String, String> {
    if !node.is_element() {
        return Ok(raw(xml, node).into());
    }
    let tag = node.tag_name().name();
    let target = part.sheet() == Some(change.sheet);
    let axis = matches!(change.edit, Edit::Axis { .. });
    let rows = matches!(change.edit, Edit::Axis { rows: true, .. });
    let mut edits = vec![];
    let start = node.range().start;
    let mut set_attr = |name: &str, value: String| {
        if let Some(a) = node.attributes().find(|a| a.name() == name) {
            edits.push((a.range_value(), esc(&value)));
        }
    };
    if target
        && axis
        && node.has_tag_name((crate::conditional_data_bar::X14, "conditionalFormatting"))
    {
        if let Some(sqref) = node
            .children()
            .find(|n| n.has_tag_name((crate::conditional_data_bar::XM, "sqref")))
            .and_then(|n| n.text())
        {
            if mapped_ranges(change, sqref)?.is_empty() {
                return Ok(String::new());
            }
        }
    }
    if tag == "cfvo" && attr(node, "type") == Some("formula") && matches!(part, Part::Worksheet(_))
    {
        if let Some(f) = attr(node, "val") {
            set_attr("val", change.formula(f, part.sheet().unwrap(), "A1")?);
        }
    }
    if matches!(part, Part::Worksheet(_)) && tag == "hyperlink" {
        if let Some(location) = attr(node, "location") {
            set_attr(
                "location",
                change.formula(location, part.sheet().unwrap(), "A1")?,
            );
        }
    }

    if part == Part::Workbook {
        if tag == "sheet" {
            if attr(node, "name") == Some(&change.names[change.sheet]) {
                match &change.edit {
                    Edit::Rename(name) => set_attr("name", name.clone()),
                    Edit::Delete => return Ok(String::new()),
                    _ => {}
                }
            }
        }
        for key in ["localSheetId", "activeTab", "firstSheet"] {
            if let Some(i) = attr(node, key).and_then(|v| v.parse::<usize>().ok()) {
                if let Some(to) = reindex(change, i) {
                    set_attr(key, to.to_string());
                } else if key == "localSheetId" {
                    return Ok(String::new());
                } else {
                    set_attr(key, change.sheet.min(change.names.len() - 2).to_string());
                }
            }
        }
    }
    if target && axis {
        if part == Part::Worksheet(change.sheet) {
            if tag == "inputCells"
                && node.has_tag_name((
                    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                    "inputCells",
                ))
            {
                return crate::scenarios::transform_input(xml, node, change);
            }
            match tag {
                "row" if rows => {
                    if let Some(r) = attr(node, "r").and_then(|s| s.parse::<i32>().ok()) {
                        match change.point(r) {
                            Some(to) if to <= 1048576 => set_attr("r", to.to_string()),
                            Some(_) => return Err("A row would exceed worksheet bounds".into()),
                            None => return Ok(String::new()),
                        }
                    }
                }
                "c" | "cellWatch" => {
                    if let Some(a) = attr(node, "r") {
                        match change.cell(a)? {
                            Some(a) => set_attr("r", a),
                            None => return Ok(String::new()),
                        }
                    }
                }
                "col" if !rows => {
                    let a = attr(node, "min")
                        .and_then(|v| v.parse().ok())
                        .ok_or("Invalid column span")?;
                    let b = attr(node, "max")
                        .and_then(|v| v.parse().ok())
                        .ok_or("Invalid column span")?;
                    if let Some((a, b)) = change.interval(a, b) {
                        if b > 16384 {
                            return Err("Column formatting would exceed worksheet bounds".into());
                        }
                        set_attr("min", a.to_string());
                        set_attr("max", b.to_string());
                    } else {
                        return Ok(String::new());
                    }
                }
                "pane" => {
                    if attr(node, "state").is_some_and(|s| s.starts_with("frozen")) {
                        let key = if rows { "ySplit" } else { "xSplit" };
                        let count = attr(node, key)
                            .unwrap_or("0")
                            .parse::<f64>()
                            .map_err(|_| "Invalid frozen pane count")?;
                        if !count.is_finite()
                            || count < 0.0
                            || count.fract() != 0.0
                            || count >= if rows { 1048576.0 } else { 16384.0 }
                        {
                            return Err("Invalid frozen pane count".into());
                        }
                        let moved = crate::sheet_view::moved_count(change, count as usize);
                        if moved >= if rows { 1048576 } else { 16384 } {
                            return Err("Frozen panes would exceed the worksheet bounds".into());
                        }
                        set_attr(key, moved.to_string());
                    }
                    if let Some(cell) = attr(node, "topLeftCell") {
                        // The pane boundary is a position, so clamp it to the
                        // deletion boundary when the old top-left cell is removed.
                        let (r, c) = parse_address(cell)?;
                        let boundary = match change.edit {
                            Edit::Axis { at, .. } => at,
                            _ => unreachable!(),
                        };
                        let to = change
                            .point(if rows { r as i32 + 1 } else { c as i32 + 1 })
                            .unwrap_or(boundary);
                        let cell = if rows {
                            crate::formula::address(to as usize - 1, c)
                        } else {
                            crate::formula::address(r, to as usize - 1)
                        };
                        set_attr("topLeftCell", cell);
                    }
                }
                "brk" => {
                    let parent = node.parent().map(|p| p.tag_name().name()).unwrap_or("");
                    if (rows && parent == "rowBreaks") || (!rows && parent == "colBreaks") {
                        if let Some(v) = attr(node, "id").and_then(|v| v.parse().ok()) {
                            match change.point(v) {
                                Some(v) => set_attr("id", v.to_string()),
                                None => return Ok(String::new()),
                            }
                        }
                    }
                }
                _ => {}
            }
            for key in ["ref", "sqref", "activeCell", "topLeftCell"] {
                if tag == "pane" || (key == "ref" && tag == "f") {
                    continue;
                }
                if let Some(value) = attr(node, key) {
                    let moved = mapped_ranges(change, value)?;
                    if moved.is_empty() {
                        if matches!(tag, "dimension" | "selection" | "sheetView") {
                            set_attr(key, "A1".into());
                        } else {
                            return Ok(String::new());
                        }
                    } else {
                        set_attr(key, moved);
                    }
                }
            }
            if tag == "filterColumn" && !rows {
                let filter = node.parent().ok_or("Invalid filter")?;
                let range = attr(filter, "ref").ok_or("Missing filter range")?;
                let first = range.split(':').next().unwrap();
                let (_, old_col) = parse_address(first)?;
                let id = attr(node, "colId")
                    .and_then(|v| v.parse::<i32>().ok())
                    .ok_or("Invalid filter column")?;
                let Some(to) = change.point(old_col as i32 + 1 + id) else {
                    return Ok(String::new());
                };
                let Some(range) = change.range(range)? else {
                    return Ok(String::new());
                };
                let (_, new_col) = parse_address(range.split(':').next().unwrap())?;
                set_attr("colId", (to - new_col as i32 - 1).to_string());
            }
        }
        if matches!(part, Part::Comments(_)) && matches!(tag, "comment" | "threadedComment") {
            if let Some(a) = attr(node, "ref") {
                match change.cell(a)? {
                    Some(to) => set_attr("ref", to),
                    None => return Ok(String::new()),
                }
            }
        }
        if matches!(part, Part::Vml(_)) && tag == "shape" {
            for child in node.descendants().filter(|n| {
                n.is_element() && n.tag_name().name() == if rows { "Row" } else { "Column" }
            }) {
                if child
                    .text()
                    .and_then(|v| v.parse::<i32>().ok())
                    .is_some_and(|v| change.point(v + 1).is_none())
                {
                    return Ok(String::new());
                }
            }
        }
    }

    let mut children = Vec::new();
    for child in node.children() {
        children.push((child, rewrite(xml, child, part, change)?));
    }
    let mut replacement_text: Option<String> = None;
    if target && axis && node.has_tag_name((crate::conditional_data_bar::XM, "sqref")) {
        replacement_text = Some(esc(&mapped_ranges(change, node.text().unwrap_or(""))?));
    }
    if tag == "f" && matches!(part, Part::Worksheet(_) | Part::Chart)
        || tag == "definedName" && part == Part::Workbook
        || matches!(tag, "formula" | "formula1" | "formula2") && matches!(part, Part::Worksheet(_))
        || matches!(tag, "calculatedColumnFormula" | "totalsRowFormula")
            && matches!(part, Part::Table(_))
    {
        if let Some(formula) = node.text() {
            let sheet = part.sheet().unwrap_or_else(|| {
                attr(node, "localSheetId")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
            });
            let mut formula = formula.to_string();
            if target && axis && matches!(tag, "formula" | "formula1" | "formula2") {
                // Relative validation/formatting rules are based at the first
                // surviving cell, even when the old anchor itself was deleted.
                if let Some(range) = node.ancestors().find_map(|n| attr(n, "sqref")) {
                    let first = range
                        .split_whitespace()
                        .next()
                        .unwrap_or("A1")
                        .split(':')
                        .next()
                        .unwrap();
                    if change.cell(first)?.is_none() {
                        let (mut r, mut c) = parse_address(first)?;
                        if let Edit::Axis {
                            rows,
                            at,
                            count,
                            insert: false,
                        } = change.edit
                        {
                            if rows {
                                r = (at + count - 1) as usize;
                            } else {
                                c = (at + count - 1) as usize;
                            }
                            formula = crate::structure::copy_formula(
                                &formula,
                                &change.names,
                                sheet,
                                first,
                                &crate::formula::address(r, c),
                            )?;
                        }
                    }
                }
            }
            replacement_text = Some(esc(&if tag == "definedName" {
                change.defined_formula(&formula, sheet)?
            } else {
                change.formula(&formula, sheet, "A1")?
            }));
        }
        if tag == "f" && target && axis {
            if matches!(attr(node, "t"), Some("shared" | "dataTable")) {
                return Err("This worksheet contains shared or data-table formulas that cannot yet be structurally edited. Nothing changed.".into());
            }
            if let Some(range) = attr(node, "ref") {
                if attr(node, "t") == Some("array") {
                    let ends: Vec<_> = range.split(':').collect();
                    let a = parse_address(ends[0])?;
                    let b = parse_address(ends[ends.len() - 1])?;
                    let (lo, hi) = if rows {
                        (a.0 + 1, b.0 + 1)
                    } else {
                        (a.1 + 1, b.1 + 1)
                    };
                    if let Edit::Axis {
                        at, count, insert, ..
                    } = change.edit
                    {
                        if if insert {
                            at as usize > lo && at as usize <= hi
                        } else {
                            (at as usize <= hi) && ((at + count) as usize > lo)
                        } {
                            return Err(
                                "You cannot change part of an array formula. Nothing changed."
                                    .into(),
                            );
                        }
                    }
                }
                if let Some(to) = change.range(range)? {
                    set_attr("ref", to);
                } else {
                    return Ok(String::new());
                }
            }
        }
    }
    if !matches!(change.edit, Edit::Move(_)) {
        if matches!(part, Part::Worksheet(_))
            && tag == "v"
            && node.parent().is_some_and(|p| child(p, "f").is_some())
        {
            return Ok(String::new());
        }
        if part == Part::Chart && matches!(tag, "numCache" | "strCache") {
            return Ok(String::new());
        }
    }
    if target && axis {
        let coordinate = matches!(part, Part::Drawing(_))
            && tag == if rows { "row" } else { "col" }
            || matches!(part, Part::Vml(_)) && tag == if rows { "Row" } else { "Column" };
        if coordinate {
            if let Some(v) = node.text().and_then(|v| v.parse::<i32>().ok()) {
                let at = match change.edit {
                    Edit::Axis { at, .. } => at,
                    _ => 1,
                };
                replacement_text = Some((change.point(v + 1).unwrap_or(at) - 1).to_string());
            }
        }
        if matches!(part, Part::Vml(_)) && tag == "Anchor" {
            let mut values: Vec<i32> = node
                .text()
                .unwrap_or("")
                .split(',')
                .map(|v| v.trim().parse().map_err(|_| "Invalid comment anchor"))
                .collect::<Result<_, _>>()?;
            if values.len() == 8 {
                for i in if rows { [2, 6] } else { [0, 4] } {
                    let at = match change.edit {
                        Edit::Axis { at, .. } => at,
                        _ => 1,
                    };
                    values[i] = change.point(values[i] + 1).unwrap_or(at) - 1;
                }
                replacement_text = Some(
                    values
                        .iter()
                        .map(i32::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
        }
    }
    if matches!(
        tag,
        "mergeCells" | "dataValidations" | "rowBreaks" | "colBreaks"
    ) && target
        && axis
    {
        let count = children
            .iter()
            .filter(|(n, s)| n.is_element() && !s.is_empty())
            .count();
        set_attr("count", count.to_string());
        if matches!(tag, "rowBreaks" | "colBreaks") {
            set_attr("manualBreakCount", count.to_string());
        }
    }
    if part == Part::Workbook && tag == "sheets" {
        if let Edit::Move(to) = change.edit {
            let mut elements: Vec<_> = children
                .iter()
                .filter(|(n, _)| n.is_element())
                .map(|(_, s)| s.clone())
                .collect();
            let s = elements.remove(change.sheet);
            elements.insert(to, s);
            let mut sorted = elements.into_iter();
            for (n, s) in &mut children {
                if n.is_element() {
                    *s = sorted.next().unwrap();
                }
            }
        }
    }
    if let Some(text) = replacement_text {
        // Formula and coordinate nodes contain text only. Preserve their prefix
        // and attributes while replacing escaped character content.
        let mut first = true;
        for (n, s) in &mut children {
            if n.is_text() {
                *s = if first {
                    first = false;
                    text.clone()
                } else {
                    String::new()
                };
            }
        }
    }
    for (child, s) in children {
        if s != raw(xml, child) {
            edits.push((child.range(), s));
        }
    }
    let local = edits
        .into_iter()
        .map(|(r, s)| (r.start - start..r.end - start, s))
        .collect();
    Ok(replace_ranges(raw(xml, node), local))
}
