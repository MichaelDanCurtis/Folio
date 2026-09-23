//! Table and filter metadata round-trips alongside retained worksheet parts.
use crate::{
    formula::{address, parse_address},
    office::*,
    tables::{self, Area},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn number(n: roxmltree::Node, key: &str, default: u64) -> u64 {
    attr(n, key).and_then(|v| v.parse().ok()).unwrap_or(default)
}
pub fn read(pkg: &Package, sheet_path: &str) -> Result<(Vec<Value>, Vec<usize>), String> {
    let xml = pkg.text(sheet_path)?;
    let d = parse(xml)?;
    let mut hidden: HashSet<usize> = d
        .descendants()
        .filter(|n| {
            n.has_tag_name((S, "row"))
                && attr(*n, "hidden").is_some_and(|v| v == "1" || v == "true")
        })
        .filter_map(|n| attr(n, "r").and_then(|v| v.parse().ok()))
        .collect();
    let mut tables = vec![];
    let related = crate::xlsx_structure::relationships(pkg, sheet_path)?;
    let linked: HashSet<_> = d
        .descendants()
        .filter(|n| n.has_tag_name((S, "tablePart")))
        .filter_map(|n| attr(n, "id"))
        .collect();
    for (_, kind, path) in related
        .iter()
        .filter(|(id, kind, _)| kind == "table" && linked.contains(id.as_str()))
    {
        let _ = kind;
        let xml = pkg.text(path)?;
        let doc = parse(xml)?;
        let root = doc.root_element();
        let mut table = json!({"id":format!("office-table-{}",path),"name":attr(root,"displayName").or(attr(root,"name")).ok_or("Table name missing")?,"range":attr(root,"ref").ok_or("Table range missing")?,"headerRows":number(root,"headerRowCount",1),"totalsRows":number(root,"totalsRowCount",0),"officePath":path,"officeId":number(root,"id",1),"filters":{},"filteredRows":[]});
        let mut cols = vec![];
        if let Some(columns) = child(root, "tableColumns") {
            for c in columns.children().filter(|n| n.is_element()) {
                let mut col = json!({"id":number(c,"id",cols.len() as u64+1),"name":attr(c,"name").ok_or("Table column name missing")?});
                if let Some(f) = child(c, "calculatedColumnFormula").and_then(|n| n.text()) {
                    col["formula"] = json!(format!("={f}"));
                }
                if let Some(f) = child(c, "totalsRowFormula").and_then(|n| n.text()) {
                    col["totalsFormula"] = json!(format!("={f}"));
                }
                if let Some(f) = attr(c, "totalsRowFunction") {
                    col["totalsFunction"] = json!(f);
                }
                if let Some(f) = attr(c, "totalsRowLabel") {
                    col["totalsLabel"] = json!(f);
                }
                cols.push(col);
            }
        }
        table["columns"] = json!(cols);
        if let Some(style) = child(root, "tableStyleInfo") {
            table["style"] = json!(attr(style, "name").unwrap_or("TableStyleMedium2"));
            for (field, key) in [
                ("rowStripes", "showRowStripes"),
                ("columnStripes", "showColumnStripes"),
                ("firstColumn", "showFirstColumn"),
                ("lastColumn", "showLastColumn"),
            ] {
                table[field] = json!(matches!(attr(style, key), Some("1" | "true")));
            }
        }
        if let Some(filter) = child(root, "autoFilter") {
            table["filters"] = read_filters(xml, filter);
        }
        if let Some(sort) = child(root, "sortState") {
            let a = tables::area(&table)?;
            let mut keys = vec![];
            let mut simple = true;
            for c in sort.children().filter(|n| n.is_element()) {
                if attr(c, "sortBy").is_some_and(|v| v != "value")
                    || attr(c, "customList").is_some()
                {
                    simple = false;
                    break;
                }
                if let Some(range) = attr(c, "ref") {
                    let r = Area::parse(range)?;
                    if r.left >= a.left && r.left <= a.right {
                        keys.push(json!({"column":r.left-a.left,"descending":matches!(attr(c,"descending"),Some("1"|"true"))}));
                    }
                }
            }
            if simple {
                table["sort"] = json!(keys);
                table["sortCaseSensitive"] =
                    json!(matches!(attr(sort, "caseSensitive"), Some("1" | "true")));
            }
        }
        let a = tables::area(&table)?;
        if !table["filters"].as_object().unwrap().is_empty() {
            let mut filtered: Vec<_> = hidden
                .iter()
                .copied()
                .filter(|r| {
                    *r > a.top + table["headerRows"].as_u64().unwrap_or(1) as usize
                        && *r <= a.bottom + 1 - table["totalsRows"].as_u64().unwrap_or(0) as usize
                })
                .collect();
            filtered.sort_unstable();
            for row in &filtered {
                hidden.remove(row);
            }
            table["filteredRows"] = json!(filtered);
        }
        tables.push(table);
    }
    let mut hidden: Vec<_> = hidden.into_iter().collect();
    hidden.sort_unstable();
    Ok((tables, hidden))
}

pub(crate) fn read_filters(xml: &str, filter: roxmltree::Node) -> Value {
    let mut rules = json!({});
    for fc in filter
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "filterColumn")
    {
        let col = number(fc, "colId", 0).to_string();
        let rule = if let Some(values) = child(fc, "filters").filter(|n| {
            n.children()
                .all(|c| !c.is_element() || c.tag_name().name() == "filter")
        }) {
            json!({"kind":"values","values":values.children().filter_map(|n|attr(n,"val")).collect::<Vec<_>>(),"blank":matches!(attr(values,"blank"),Some("1"|"true"))})
        } else if let Some(custom) = child(fc, "customFilters") {
            json!({"kind":"custom","join":if matches!(attr(custom,"and"),Some("1"|"true")){"and"}else{"or"},"conditions":custom.children().filter(|n|n.is_element()).map(|n|json!({"op":attr(n,"operator").unwrap_or("equal"),"value":attr(n,"val").unwrap_or("")})).collect::<Vec<_>>()})
        } else {
            json!({"kind":"unsupported","officeXml":raw(xml,fc)})
        };
        rules[col] = rule;
    }
    rules
}

pub(crate) fn inner(source: &str, node: roxmltree::Node, contents: &str) -> String {
    let s = raw(source, node);
    let end = opening_tag_end(s).unwrap();
    if s[..end].ends_with('/') {
        let tag = s[1..].split([' ', '/', '>']).next().unwrap();
        format!("{}>{contents}</{tag}>", &s[..end - 1])
    } else {
        format!("{}{contents}{}", &s[..=end], &s[s.rfind("</").unwrap()..])
    }
}
pub(crate) fn attributes(
    source: &str,
    node: roxmltree::Node,
    values: &[(&str, Option<String>)],
) -> String {
    let mut edits = vec![];
    let mut missing = String::new();
    let base = node.range().start;
    for (key, value) in values {
        if let Some(a) = node
            .attributes()
            .find(|a| a.namespace().is_none() && a.name() == *key)
        {
            match value {
                Some(v) => edits.push((
                    a.range_value().start - base..a.range_value().end - base,
                    esc(v),
                )),
                None => edits.push((a.range().start - base..a.range().end - base, String::new())),
            }
        } else if let Some(v) = value {
            missing.push_str(&format!(" {key}=\"{}\"", esc(v)));
        }
    }
    let s = raw(source, node);
    let end = opening_tag_end(s).unwrap();
    let at = if s.as_bytes()[end - 1] == b'/' {
        end - 1
    } else {
        end
    };
    edits.push((at..at, missing));
    replace_ranges(s, edits)
}
fn append(pkg: &mut Package, path: &str, extra: &str) -> Result<(), String> {
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let content = root.children().map(|n| raw(&xml, n)).collect::<String>() + extra;
    pkg.set(
        path,
        replace_ranges(&xml, vec![(root.range(), inner(&xml, root, &content))]),
    );
    Ok(())
}
pub(crate) fn replace_child(xml: &str, tag: &str, content: &str) -> Result<String, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    if let Some(node) = child(root, tag) {
        Ok(replace_ranges(xml, vec![(node.range(), content.into())]))
    } else {
        // extLst must remain last in table and worksheet child sequences.
        let position = child(root, "extLst")
            .map(|n| n.range().start)
            .unwrap_or_else(|| {
                root.range().start
                    + raw(xml, root)
                        .rfind("</")
                        .unwrap_or(raw(xml, root).len() - 2)
            });
        if raw(xml, root).ends_with("/>") {
            Ok(replace_ranges(
                xml,
                vec![(root.range(), inner(xml, root, content))],
            ))
        } else {
            Ok(replace_ranges(
                xml,
                vec![(position..position, content.into())],
            ))
        }
    }
}
pub(crate) fn filter_xml(table: &Value) -> Result<String, String> {
    let mut area = tables::area(table)?;
    area.bottom = (area.bottom + 1)
        .saturating_sub(table["totalsRows"].as_u64().unwrap_or(0) as usize)
        .saturating_sub(1)
        .max(area.top);
    let mut columns = String::new();
    for (col, rule) in table
        .get("filters")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let body = match rule["kind"].as_str() {
            Some("values") => format!(
                "<filters blank=\"{}\">{}</filters>",
                u8::from(rule["blank"] == true),
                rule["values"]
                    .as_array()
                    .ok_or("Invalid value filter")?
                    .iter()
                    .map(|v| format!("<filter val=\"{}\"/>", esc(v.as_str().unwrap_or(""))))
                    .collect::<String>()
            ),
            Some("custom") => {
                let mut conditions = String::new();
                for c in rule["conditions"]
                    .as_array()
                    .ok_or("Invalid custom filter")?
                {
                    let op = c["op"].as_str().unwrap_or("equal");
                    let value = c["value"].as_str().unwrap_or("");
                    let escape =
                        |s: &str| s.replace('~', "~~").replace('*', "~*").replace('?', "~?");
                    let (op, value) = match op {
                        "contains" => ("equal", format!("*{}*", escape(value))),
                        "beginsWith" => ("equal", format!("{}*", escape(value))),
                        "endsWith" => ("equal", format!("*{}", escape(value))),
                        _ => (op, value.to_string()),
                    };
                    conditions.push_str(&format!(
                        "<customFilter operator=\"{}\" val=\"{}\"/>",
                        esc(op),
                        esc(&value)
                    ));
                }
                format!(
                    "<customFilters and=\"{}\">{conditions}</customFilters>",
                    u8::from(rule["join"] == "and")
                )
            }
            Some("unsupported") => {
                columns.push_str(
                    rule["officeXml"]
                        .as_str()
                        .ok_or("Missing retained filter")?,
                );
                continue;
            }
            _ => return Err("Unsupported table filter".into()),
        };
        columns.push_str(&format!(
            "<filterColumn xmlns=\"{S}\" colId=\"{}\">{body}</filterColumn>",
            esc(col)
        ));
    }
    Ok(format!(
        "<autoFilter xmlns=\"{S}\" ref=\"{}\">{columns}</autoFilter>",
        area.text()
    ))
}
pub fn table_xml(table: &Value, original: Option<&str>, id: u64) -> Result<String, String> {
    let a = tables::area(table)?;
    let base=original.map(String::from).unwrap_or_else(||format!("{XML_HEAD}<table xmlns=\"{S}\" id=\"{id}\" name=\"Table\" displayName=\"Table\" ref=\"{}\"/>",a.text()));
    let d = parse(&base)?;
    let root = d.root_element();
    let mut xml = replace_ranges(
        &base,
        vec![(
            root.range(),
            attributes(
                &base,
                root,
                &[
                    ("name", Some(table["name"].as_str().unwrap().into())),
                    ("displayName", Some(table["name"].as_str().unwrap().into())),
                    ("ref", Some(a.text())),
                    (
                        "headerRowCount",
                        Some(table["headerRows"].as_u64().unwrap_or(1).to_string()),
                    ),
                    (
                        "totalsRowShown",
                        Some(if table["totalsRows"] == 1 { "1" } else { "0" }.into()),
                    ),
                    (
                        "totalsRowCount",
                        Some(table["totalsRows"].as_u64().unwrap_or(0).to_string()),
                    ),
                ],
            ),
        )],
    );
    let mut columns = String::new();
    for c in table["columns"].as_array().ok_or("Missing table columns")? {
        let id = c["id"].as_u64().ok_or("Missing table column id")?;
        let old = child(root, "tableColumns")
            .and_then(|n| n.children().find(|n| number(*n, "id", 0) == id));
        let mut col = if let Some(node) = old {
            let attrs = [
                (
                    "name",
                    Some(
                        c["name"]
                            .as_str()
                            .ok_or("Missing table column name")?
                            .into(),
                    ),
                ),
                (
                    "totalsRowFunction",
                    c["totalsFunction"].as_str().map(String::from),
                ),
                (
                    "totalsRowLabel",
                    c["totalsLabel"].as_str().map(String::from),
                ),
            ];
            let fragment = attributes(&base, node, &attrs);
            // Make inherited bindings explicit so retained extension attributes
            // and formulas remain parseable when this fragment is edited alone.
            let end = fragment.find('>').unwrap();
            let at = if fragment.as_bytes()[end - 1] == b'/' {
                end - 1
            } else {
                end
            };
            let ns = root
                .namespaces()
                .filter(|n| {
                    !node.namespaces().any(|own| {
                        own.name() == n.name()
                            && fragment[..end].contains(&format!(
                                "xmlns{}=",
                                n.name().map(|p| format!(":{p}")).unwrap_or_default()
                            ))
                    })
                })
                .map(|n| {
                    format!(
                        " xmlns{}=\"{}\"",
                        n.name().map(|p| format!(":{p}")).unwrap_or_default(),
                        esc(n.uri())
                    )
                })
                .collect::<String>();
            replace_ranges(&fragment, vec![(at..at, ns)])
        } else {
            format!(
                "<tableColumn xmlns=\"{S}\" id=\"{id}\" name=\"{}\"/>",
                esc(c["name"].as_str().unwrap())
            )
        };
        let cd = parse(&col)?;
        col = attributes(
            &col,
            cd.root_element(),
            &[
                (
                    "totalsRowFunction",
                    c["totalsFunction"].as_str().map(String::from),
                ),
                (
                    "totalsRowLabel",
                    c["totalsLabel"].as_str().map(String::from),
                ),
            ],
        );
        let d = parse(&col)?;
        let node = d.root_element();
        let mut contents = String::new();
        for (field, tag) in [
            ("formula", "calculatedColumnFormula"),
            ("totalsFormula", "totalsRowFormula"),
        ] {
            if field == "totalsFormula" && c["totalsFunction"] != "custom" {
                continue;
            }
            if let Some(formula) = c[field].as_str() {
                let text = esc(formula.strip_prefix('=').unwrap_or(formula));
                if let Some(original) = child(node, tag) {
                    contents.push_str(&inner(&col, original, &text));
                } else {
                    contents.push_str(&format!("<{tag} xmlns=\"{S}\">{text}</{tag}>"));
                }
            } else if let Some(original) = child(node, tag) {
                contents.push_str(raw(&col, original));
            }
        }
        for extra in node.children().filter(|n| {
            n.is_element()
                && !matches!(
                    n.tag_name().name(),
                    "calculatedColumnFormula" | "totalsRowFormula"
                )
        }) {
            contents.push_str(raw(&col, extra));
        }
        col = inner(&col, node, &contents);
        columns.push_str(&col);
    }
    // Rebuild children in their schema order; retain sortState/extension/style
    // extras from the original root, replacing only modeled children.
    let xd = parse(&xml)?;
    let xr = xd.root_element();
    let mut body = String::new();
    if table["headerRows"] != 0 {
        body.push_str(&filter_xml(table)?);
    }
    if let Some(keys) = table.get("sort").and_then(Value::as_array) {
        let first = a.top + table["headerRows"].as_u64().unwrap_or(1) as usize;
        let end = a.bottom + 1 - table["totalsRows"].as_u64().unwrap_or(0) as usize;
        if first < end && !keys.is_empty() {
            let conditions = keys
                .iter()
                .map(|k| {
                    let col = a.left + k["column"].as_u64().unwrap_or(0) as usize;
                    format!(
                        "<sortCondition ref=\"{}:{}\" descending=\"{}\"/>",
                        address(first, col),
                        address(end - 1, col),
                        u8::from(k["descending"] == true)
                    )
                })
                .collect::<String>();
            body.push_str(&format!("<sortState xmlns=\"{S}\" ref=\"{}:{}\" caseSensitive=\"{}\">{conditions}</sortState>",address(first,a.left),address(end-1,a.right),u8::from(table["sortCaseSensitive"]==true)));
        }
    } else if let Some(sort) = child(root, "sortState") {
        // Retained custom/color sorts are not executable in Folio yet, but a
        // boundary resize must not discard their choices or leave stale ranges.
        let first = a.top + table["headerRows"].as_u64().unwrap_or(1) as usize;
        let end = a.bottom + 1 - table["totalsRows"].as_u64().unwrap_or(0) as usize;
        let mut edits = vec![];
        if let Some(r) = sort.attributes().find(|r| r.name() == "ref") {
            edits.push((
                r.range_value(),
                format!(
                    "{}:{}",
                    address(first, a.left),
                    address(end.saturating_sub(1), a.right)
                ),
            ));
        }
        for condition in sort
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "sortCondition")
        {
            if let Some(r) = condition.attributes().find(|r| r.name() == "ref") {
                let old = Area::parse(r.value())?;
                if old.right < a.left || old.left > a.right || first >= end {
                    edits.push((condition.range(), String::new()));
                } else {
                    edits.push((
                        r.range_value(),
                        format!(
                            "{}:{}",
                            address(first, old.left.max(a.left)),
                            address(end - 1, old.right.min(a.right))
                        ),
                    ));
                }
            }
        }
        if first < end {
            let base_offset = sort.range().start;
            body.push_str(&replace_ranges(
                raw(&base, sort),
                edits
                    .into_iter()
                    .map(|(r, s)| (r.start - base_offset..r.end - base_offset, s))
                    .collect(),
            ));
        }
    }
    body.push_str(&format!(
        "<tableColumns xmlns=\"{S}\" count=\"{}\">{columns}</tableColumns>",
        table["columns"].as_array().unwrap().len()
    ));
    body.push_str(&format!("<tableStyleInfo xmlns=\"{S}\" name=\"{}\" showFirstColumn=\"{}\" showLastColumn=\"{}\" showRowStripes=\"{}\" showColumnStripes=\"{}\"/>",esc(table["style"].as_str().unwrap_or("TableStyleMedium2")),u8::from(table["firstColumn"]==true),u8::from(table["lastColumn"]==true),u8::from(table["rowStripes"]!=false),u8::from(table["columnStripes"]==true)));
    for extra in root.children().filter(|n| {
        n.is_element()
            && !matches!(
                n.tag_name().name(),
                "autoFilter" | "sortState" | "tableColumns" | "tableStyleInfo"
            )
    }) {
        body.push_str(raw(&base, extra));
    }
    xml = replace_ranges(&xml, vec![(xr.range(), inner(&xml, xr, &body))]);
    Ok(xml)
}

pub fn write(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    if sheet.get("tables").is_none() {
        crate::sheet_filter::write(pkg, sheet, path)?;
        crate::sheet_sort::write(pkg, sheet, path)?;
        return visibility(pkg, sheet, path);
    }
    let source = parse(pkg.text(path)?)?;
    for merge in source
        .descendants()
        .filter(|n| n.has_tag_name((S, "mergeCell")))
    {
        if let Some(range) = attr(merge, "ref") {
            let merged = Area::parse(range)?;
            if tables::list(sheet).any(|t| tables::area(t).is_ok_and(|a| a.overlaps(merged))) {
                return Err(
                    "Unmerge cells before including them in a table. Nothing changed.".into(),
                );
            }
        }
    }
    let wb = parse(pkg.text("xl/workbook.xml")?)?;
    for t in tables::list(sheet) {
        if wb
            .descendants()
            .filter(|n| n.tag_name().name() == "definedName" && attr(*n, "localSheetId").is_none())
            .any(|n| {
                attr(n, "name")
                    .is_some_and(|name| name.eq_ignore_ascii_case(t["name"].as_str().unwrap()))
            })
        {
            return Err("A workbook defined name already uses this table name".into());
        }
    }
    let relations = crate::xlsx_structure::relationships(pkg, path)?;
    let (dir, file) = path.rsplit_once('/').ok_or("Invalid worksheet path")?;
    let rel_path = format!("{dir}/_rels/{file}.rels");
    if !pkg.entries.contains_key(&rel_path) && tables::list(sheet).next().is_some() {
        pkg.set(
            &rel_path,
            format!("{XML_HEAD}<Relationships xmlns=\"{REL_NS}\"/>"),
        );
    }
    let mut parts = String::new();
    for t in tables::list(sheet) {
        let found = relations
            .iter()
            .filter(|(_, kind, _)| kind == "table")
            .find(|(_, _, p)| {
                pkg.text(p)
                    .ok()
                    .and_then(|s| parse(s).ok())
                    .is_some_and(|d| {
                        attr(d.root_element(), "displayName") == t["name"].as_str()
                            || t["officePath"] == p.as_str()
                    })
            });
        let (rid, table_path) = if let Some((rid, _, path)) = found {
            (rid.clone(), path.clone())
        } else {
            let table_path = (1..)
                .map(|i| format!("xl/tables/folio-table-{i}.xml"))
                .find(|p| !pkg.entries.contains_key(p))
                .unwrap();
            let existing = parse(pkg.text(&rel_path)?)?;
            let ids: HashSet<_> = existing
                .descendants()
                .filter_map(|n| attr(n, "Id"))
                .collect();
            let rid = (1..)
                .map(|i| format!("folioTable{i}"))
                .find(|i| !ids.contains(i.as_str()))
                .unwrap();
            append(
                pkg,
                &rel_path,
                &format!(
                    "<Relationship Id=\"{rid}\" Type=\"{R}/table\" Target=\"/{}\"/>",
                    esc(&table_path)
                ),
            )?;
            append(pkg,"[Content_Types].xml",&format!("<Override PartName=\"/{table_path}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml\"/>"))?;
            (rid, table_path)
        };
        let existing = pkg.text(&table_path).ok().map(String::from);
        let id = if let Some(xml) = &existing {
            number(parse(xml)?.root_element(), "id", 1)
        } else {
            pkg.entries
                .keys()
                .filter(|p| p.starts_with("xl/tables/") && p.ends_with(".xml"))
                .filter_map(|p| {
                    pkg.text(p)
                        .ok()
                        .and_then(|s| parse(s).ok())
                        .map(|d| number(d.root_element(), "id", 0))
                })
                .max()
                .unwrap_or(0)
                + 1
        };
        let xml = table_xml(t, existing.as_deref(), id)?;
        pkg.set(&table_path, xml);
        parts.push_str(&format!("<tablePart xmlns:r=\"{R}\" r:id=\"{rid}\"/>"));
    }
    let xml = pkg.text(path)?.to_string();
    let content = if parts.is_empty() {
        String::new()
    } else {
        format!(
            "<tableParts xmlns=\"{S}\" count=\"{}\">{parts}</tableParts>",
            tables::list(sheet).count()
        )
    };
    pkg.set(path, replace_child(&xml, "tableParts", &content)?);
    crate::sheet_filter::write(pkg, sheet, path)?;
    crate::sheet_sort::write(pkg, sheet, path)?;
    visibility(pkg, sheet, path)
}

fn visibility(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    let hidden = tables::hidden_rows(sheet);
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let data = child(d.root_element(), "sheetData").ok_or("Worksheet data missing")?;
    let mut rows = BTreeMap::new();
    for row in data.children().filter(|n| n.is_element()) {
        let r = number(row, "r", 0) as usize;
        rows.insert(
            r,
            attributes(
                &xml,
                row,
                &[("hidden", hidden.contains(&r).then(|| "1".into()))],
            ),
        );
    }
    for r in hidden {
        rows.entry(r)
            .or_insert_with(|| format!("<row xmlns=\"{S}\" r=\"{r}\" hidden=\"1\"/>"));
    }
    let contents = rows.values().cloned().collect::<String>();
    let result = replace_ranges(&xml, vec![(data.range(), inner(&xml, data, &contents))]);
    pkg.set(path, result);
    Ok(())
}

/// Record-bound notes and internal hyperlinks follow a sorted cell. Other
/// formulas still reference positions, as they do for a normal range sort.
pub fn sort_objects(
    pkg: &mut Package,
    sheet: &Value,
    table: &Value,
    map: &[(usize, usize)],
    copy_formulas: bool,
) -> Result<(), String> {
    let path = sheet["officePath"]
        .as_str()
        .ok_or("Missing worksheet source")?;
    crate::xlsx_reviews::normalize_anchors(pkg, path)?;
    let a = tables::area(table)?;
    let map: HashMap<_, _> = map.iter().copied().collect();
    let move_cell = |cell: &str| -> Result<String, String> {
        let (r, c) = parse_address(cell)?;
        Ok(if c >= a.left && c <= a.right {
            address(map.get(&r).copied().unwrap_or(r), c)
        } else {
            cell.into()
        })
    };
    // Sort complete XML cell records, including rich text and cell metadata.
    // Row heights remain attached to worksheet positions.
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    for row in d
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "row")
    {
        let r = number(row, "r", 1) as usize - 1;
        if number(row, "outlineLevel", 0) > 0 && map.get(&r).is_some_and(|to| *to != r) {
            return Err(
                "Sorting outlined row groups needs additional support. Nothing changed.".into(),
            );
        }
    }
    for n in d
        .descendants()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "mergeCell" | "f"))
    {
        if let Some(range) = attr(n, "ref") {
            if Area::parse(range)?.overlaps(a) && matches!(n.tag_name().name(), "mergeCell")
                || (n.tag_name().name() == "f"
                    && attr(n, "t").is_some_and(|t| t == "array" || t == "dataTable")
                    && Area::parse(range)?.overlaps(a))
            {
                return Err("Unmerge cells or remove the array/data-table range before moving these table cells. Nothing changed.".into());
            }
        }
    }
    let wb = parse(pkg.text("xl/workbook.xml")?)?;
    let names: Vec<_> = wb
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sheet")
        .filter_map(|n| attr(n, "name").map(String::from))
        .collect();
    let si = names
        .iter()
        .position(|name| Some(name.as_str()) == sheet["name"].as_str())
        .ok_or("Worksheet name missing")?;
    let data = child(d.root_element(), "sheetData").ok_or("Sheet data missing")?;
    let mut contents: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
    let mut row_nodes = BTreeMap::new();
    for row in data.children().filter(|n| n.is_element()) {
        let ri = number(row, "r", 1) as usize - 1;
        row_nodes.insert(ri, row);
        for cell in row
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "c")
        {
            let from = attr(cell, "r").ok_or("Cell address missing")?;
            let (_, ci) = parse_address(from)?;
            let to = move_cell(from)?;
            let (tr, _) = parse_address(&to)?;
            let mut edits = vec![];
            if from != to {
                let att = cell.attributes().find(|a| a.name() == "r").unwrap();
                edits.push((att.range_value(), to.clone()));
                if let Some(f) = child(cell, "f") {
                    let expr = sheet["cells"][from]
                        .as_str()
                        .and_then(|f| f.strip_prefix('='))
                        .ok_or("Formula projection missing")?;
                    let copied = if copy_formulas {
                        crate::structure::copy_formula(expr, &names, si, from, &to)?
                    } else {
                        expr.to_string()
                    };
                    let tag = raw(&xml, f)[1..].split([' ', '/', '>']).next().unwrap();
                    edits.push((f.range(), format!("<{tag}>{}</{tag}>", esc(&copied))));
                    if let Some(v) = child(cell, "v") {
                        edits.push((v.range(), String::new()));
                    }
                }
            }
            let base = cell.range().start;
            let fragment = replace_ranges(
                raw(&xml, cell),
                edits
                    .into_iter()
                    .map(|(r, s)| (r.start - base..r.end - base, s))
                    .collect(),
            );
            contents.entry(tr).or_default().insert(ci, fragment);
        }
    }
    let mut rows = String::new();
    let all: std::collections::BTreeSet<_> =
        row_nodes.keys().chain(contents.keys()).copied().collect();
    for ri in all {
        let body = contents
            .remove(&ri)
            .unwrap_or_default()
            .into_values()
            .collect::<String>();
        if let Some(node) = row_nodes.get(&ri) {
            let extras = node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() != "c")
                .map(|n| raw(&xml, n))
                .collect::<String>();
            rows.push_str(&inner(&xml, *node, &(body + &extras)));
        } else {
            rows.push_str(&format!("<row r=\"{}\">{body}</row>", ri + 1));
        }
    }
    pkg.set(
        path,
        replace_ranges(&xml, vec![(data.range(), inner(&xml, data, &rows))]),
    );
    let mut parts = crate::xlsx_structure::relationships(pkg, path)?;
    parts.push((String::new(), "worksheet".into(), path.into()));
    for (_, kind, path) in parts {
        if !matches!(
            kind.as_str(),
            "comments" | "threadedComment" | "worksheet" | "vmlDrawing"
        ) {
            continue;
        }
        let xml = pkg.text(&path)?.to_string();
        let d = parse(&xml)?;
        let mut edits = vec![];
        for node in d.descendants().filter(|n| n.is_element()) {
            match (kind.as_str(), node.tag_name().name()) {
                ("comments", "comment")
                | ("threadedComment", "threadedComment")
                | ("worksheet", "hyperlink") => {
                    if let Some(r) = node.attributes().find(|a| a.name() == "ref") {
                        if r.value().contains(':') {
                            if Area::parse(r.value())?.overlaps(a) {
                                return Err("Moving cells with multi-cell hyperlinks needs additional support. Nothing changed.".into());
                            }
                        } else {
                            edits.push((r.range_value(), move_cell(r.value())?));
                        }
                    }
                }
                ("vmlDrawing", "ClientData") => {
                    let row = child(node, "Row")
                        .and_then(|n| n.text())
                        .and_then(|v| v.parse::<usize>().ok());
                    let col = child(node, "Column")
                        .and_then(|n| n.text())
                        .and_then(|v| v.parse::<usize>().ok());
                    if let (Some(row), Some(col)) = (row, col) {
                        if col >= a.left && col <= a.right {
                            if let Some(to) = map.get(&row) {
                                if let Some(n) = child(node, "Row").and_then(|n| n.first_child()) {
                                    edits.push((n.range(), to.to_string()));
                                }
                                if let Some(n) = child(node, "Anchor").and_then(|n| n.first_child())
                                {
                                    let mut values: Vec<i32> = n
                                        .text()
                                        .unwrap_or("")
                                        .split(',')
                                        .map(|v| {
                                            v.trim().parse().map_err(|_| "Invalid comment anchor")
                                        })
                                        .collect::<Result<_, _>>()?;
                                    if values.len() == 8 {
                                        for i in [2, 6] {
                                            values[i] =
                                                (values[i] + *to as i32 - row as i32).max(0);
                                        }
                                        edits.push((
                                            n.range(),
                                            values
                                                .iter()
                                                .map(i32::to_string)
                                                .collect::<Vec<_>>()
                                                .join(", "),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        pkg.set(&path, replace_ranges(&xml, edits));
    }
    Ok(())
}

/// Attach generated package identities without replacing editable projections.
pub fn bind_paths(sheets: &mut Value, projection: &Value, styles: bool) -> Result<(), String> {
    for sheet in sheets.as_array_mut().ok_or("Worksheets missing")? {
        let imported = projection
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == sheet["name"])
            .ok_or("Updated worksheet missing from package")?;
        sheet["officePath"] = imported["officePath"].clone();
        if styles {
            sheet["officeStyles"] = imported["officeStyles"].clone();
        }
        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut) {
            for t in tables {
                if let Some(it) = tables::list(imported).find(|it| {
                    it["name"] == t["name"]
                        || (!t["officePath"].is_null() && t["officePath"] == it["officePath"])
                }) {
                    t["officePath"] = it["officePath"].clone();
                    t["officeId"] = it["officeId"].clone();
                }
            }
        }
    }
    Ok(())
}

/// Update structured references in names, chart sources and other retained
/// formula-bearing parts as well as in the editable cell projection.
pub fn rewrite_references(pkg: &mut Package, before: &Value, after: &Value) -> Result<(), String> {
    let new_tables: Vec<_> = after
        .as_array()
        .ok_or("Worksheets missing")?
        .iter()
        .flat_map(tables::list)
        .collect();
    let mut changes = vec![];
    for old in before
        .as_array()
        .ok_or("Worksheets missing")?
        .iter()
        .flat_map(tables::list)
    {
        let new = new_tables
            .iter()
            .find(|n| {
                n["id"] == old["id"]
                    || (!old["officePath"].is_null() && n["officePath"] == old["officePath"])
            })
            .copied();
        let mut columns = HashMap::new();
        if let Some(new) = new {
            for col in old["columns"].as_array().unwrap() {
                let replacement = new["columns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == col["id"])
                    .and_then(|c| c["name"].as_str());
                if replacement != col["name"].as_str() {
                    columns.insert(
                        col["name"].as_str().unwrap().to_lowercase(),
                        replacement.map(String::from),
                    );
                }
            }
        }
        if new.is_none() || new.is_some_and(|n| n["name"] != old["name"]) || !columns.is_empty() {
            changes.push((old, new, columns));
        }
    }
    if changes.is_empty() {
        return Ok(());
    }
    let paths: Vec<_> = pkg
        .entries
        .keys()
        .filter(|p| p.starts_with("xl/") && p.ends_with(".xml"))
        .cloned()
        .collect();
    for path in paths {
        let xml = pkg.text(&path)?.to_string();
        let d = parse(&xml)?;
        let mut edits = vec![];
        for n in d.descendants().filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "f" | "formula"
                        | "formula1"
                        | "formula2"
                        | "definedName"
                        | "calculatedColumnFormula"
                        | "totalsRowFormula"
                )
        }) {
            let Some(text) = n.text() else {
                continue;
            };
            let mut result = text.to_string();
            let local = before
                .as_array()
                .unwrap()
                .iter()
                .flat_map(tables::list)
                .find(|t| t["officePath"] == path)
                .and_then(|t| t["name"].as_str());
            for (old, new, columns) in &changes {
                let name = old["name"].as_str().unwrap();
                result = if let Some(new) = new {
                    let rewritten =
                        crate::table_refs::rename_columns(&result, local, name, columns)?;
                    crate::table_refs::rename_table(
                        &rewritten,
                        name,
                        new["name"].as_str().unwrap(),
                    )?
                } else {
                    crate::table_refs::invalidate_table(&result, name)?
                };
            }
            if result != text {
                if let Some(t) = n.children().find(|n| n.is_text()) {
                    edits.push((t.range(), esc(&result)));
                }
            }
        }
        if !edits.is_empty() {
            pkg.set(&path, replace_ranges(&xml, edits));
        }
    }
    Ok(())
}
