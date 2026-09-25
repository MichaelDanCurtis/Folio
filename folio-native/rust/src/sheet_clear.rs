//! Home > Clear (X01.17): All, Formats, Contents, Comments and Notes, and
//! Hyperlinks, on a rectangular selection. Only populated entries are
//! visited, so whole rows and columns are cheap.
use crate::formula::parse_address;
use crate::office::{attr, esc, parse, raw, replace_ranges, Package};
use crate::tables::Area;
use serde_json::{json, Value};

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

#[derive(Clone, Copy, PartialEq)]
pub enum What {
    All,
    Formats,
    Contents,
    Notes,
    Hyperlinks,
}

impl What {
    pub fn from_request(r: &Value) -> Result<Self, String> {
        Ok(match r["what"].as_str() {
            Some("all") => Self::All,
            Some("formats") => Self::Formats,
            Some("contents") => Self::Contents,
            Some("notes") => Self::Notes,
            Some("hyperlinks") => Self::Hyperlinks,
            _ => return Err("Choose what to clear".into()),
        })
    }
    pub fn clears_hyperlinks(self) -> bool {
        matches!(self, Self::All | Self::Hyperlinks)
    }
}

fn inside(area: &Area, a: &str) -> bool {
    parse_address(a)
        .is_ok_and(|(r, c)| r >= area.top && r <= area.bottom && c >= area.left && c <= area.right)
}

fn keys_in(sheet: &Value, field: &str, area: &Area) -> Vec<String> {
    sheet[field]
        .as_object()
        .into_iter()
        .flat_map(|m| m.keys())
        .filter(|a| inside(area, a))
        .cloned()
        .collect()
}

/// The style a cell takes when its own format is cleared: its row's style,
/// else its column's, else the workbook default ("0"), with the style's
/// projection for display.
pub type LineStyle<'a> = &'a dyn Fn(&str) -> Option<(String, Value)>;

/// Clears the workspace projection. `line_style` is `None` for workbooks
/// with no source package, where cleared cells simply lose their style.
pub fn change(
    workspace: &Value,
    r: &Value,
    line_style: Option<LineStyle>,
) -> Result<Value, String> {
    let what = What::from_request(r)?;
    let area = Area::parse(r["range"].as_str().ok_or("Select cells to clear")?.trim())?;
    let mut next = workspace.clone();
    let sheet = next["sheets"]
        .as_array_mut()
        .and_then(|s| s.iter_mut().find(|s| s["id"] == r["sheetId"]))
        .ok_or("Worksheet missing")?;
    if matches!(what, What::All | What::Formats) {
        let mut styled = keys_in(sheet, "cellStyles", &area);
        styled.extend(keys_in(sheet, "officeStyles", &area));
        styled.sort();
        styled.dedup();
        for a in styled {
            if let Some(m) = sheet["cellStyles"].as_object_mut() {
                m.remove(&a);
            }
            if let Some(m) = sheet["officeStyles"].as_object_mut() {
                m.remove(&a);
            }
            // A cell kept in a file names its style explicitly, or the export
            // would keep the old one.
            if let Some((id, spec)) = line_style.and_then(|f| f(&a)) {
                if !sheet["officeStyles"].is_object() {
                    sheet["officeStyles"] = json!({});
                }
                sheet["officeStyles"][&a] = json!(id);
                if id != "0" {
                    if !sheet["cellStyles"].is_object() {
                        sheet["cellStyles"] = json!({});
                    }
                    sheet["cellStyles"][&a] = spec;
                }
            }
        }
    }
    if matches!(what, What::All | What::Contents) {
        for a in keys_in(sheet, "cells", &area) {
            let formatted = !sheet["cellStyles"][&a].is_null()
                || sheet["officeStyles"][&a].as_str().is_some_and(|s| s != "0");
            if formatted {
                sheet["cells"][&a] = json!("");
            } else {
                sheet["cells"].as_object_mut().unwrap().remove(&a);
                if let Some(m) = sheet["officeStyles"].as_object_mut() {
                    m.remove(&a);
                }
            }
        }
    }
    if matches!(what, What::All | What::Notes) {
        for a in keys_in(sheet, crate::sheet_reviews::KEY, &area) {
            sheet[crate::sheet_reviews::KEY]
                .as_object_mut()
                .unwrap()
                .remove(&a);
        }
    }
    if what == What::Formats {
        // A blank cell kept only for its format goes with the format.
        for a in keys_in(sheet, "cells", &area) {
            if sheet["cells"][&a] == ""
                && sheet["cellStyles"][&a].is_null()
                && sheet["officeStyles"][&a].as_str().is_none_or(|s| s == "0")
            {
                sheet["cells"].as_object_mut().unwrap().remove(&a);
                if let Some(m) = sheet["officeStyles"].as_object_mut() {
                    m.remove(&a);
                }
            }
        }
    }
    Ok(next)
}

/// Removes the worksheet's hyperlinks from the cells in `range`. A link
/// covering a larger range keeps the parts outside the selection.
pub fn clear_hyperlinks(pkg: &mut Package, sheet_path: &str, range: &str) -> Result<(), String> {
    let area = Area::parse(range.trim())?;
    let Ok(xml) = pkg.text(sheet_path).map(String::from) else {
        return Ok(());
    };
    let d = parse(&xml)?;
    let Some(list) = d.descendants().find(|n| n.has_tag_name((S, "hyperlinks"))) else {
        return Ok(());
    };
    let mut kept = String::new();
    let mut changed = false;
    let mut dropped_ids = vec![];
    for link in list.children().filter(|n| n.has_tag_name((S, "hyperlink"))) {
        let record = raw(&xml, link);
        let Some(target) = attr(link, "ref").and_then(|r| Area::parse(r).ok()) else {
            kept.push_str(record);
            continue;
        };
        let pieces = subtract(&target, &area);
        if pieces.len() == 1 && pieces[0] == target {
            kept.push_str(record);
            continue;
        }
        changed = true;
        if pieces.is_empty() {
            if let Some(id) = link.attribute((R, "id")) {
                dropped_ids.push(id.to_string());
            }
        }
        let record = crate::office::scoped_record(&xml, link);
        for piece in pieces {
            let d = parse(&record)?;
            kept.push_str(&crate::xlsx_tables::attributes(
                &record,
                d.root_element(),
                &[("ref", Some(compact(piece)))],
            ));
        }
    }
    if !changed {
        return Ok(());
    }
    let updated = if kept.is_empty() {
        replace_ranges(&xml, vec![(list.range(), String::new())])
    } else {
        crate::office::replace_inner(&xml, list, &kept)
    };
    // Drop relationships nothing refers to any more.
    let (dir, name) = sheet_path.rsplit_once('/').unwrap_or(("", sheet_path));
    let rels_path = format!("{dir}/_rels/{name}.rels");
    if let Ok(rels) = pkg.text(&rels_path).map(String::from) {
        let rd = parse(&rels)?;
        let unused: Vec<_> = rd
            .root_element()
            .children()
            .filter(|n| {
                n.is_element()
                    && attr(*n, "Id").is_some_and(|id| {
                        dropped_ids.iter().any(|d| d == id)
                            && !updated.contains(&format!("\"{}\"", esc(id)))
                    })
            })
            .map(|n| (n.range(), String::new()))
            .collect();
        if !unused.is_empty() {
            pkg.set(&rels_path, replace_ranges(&rels, unused));
        }
    }
    pkg.set(sheet_path, updated);
    Ok(())
}

fn compact(area: Area) -> String {
    if area.top == area.bottom && area.left == area.right {
        crate::formula::address(area.top, area.left)
    } else {
        area.text()
    }
}

/// `target` minus `hole`, as up to four rectangles (above, below, left,
/// right of the hole).
fn subtract(target: &Area, hole: &Area) -> Vec<Area> {
    if hole.bottom < target.top
        || hole.top > target.bottom
        || hole.right < target.left
        || hole.left > target.right
    {
        return vec![*target];
    }
    let mut out = vec![];
    if target.top < hole.top {
        out.push(Area {
            bottom: hole.top - 1,
            ..*target
        });
    }
    if target.bottom > hole.bottom {
        out.push(Area {
            top: hole.bottom + 1,
            ..*target
        });
    }
    let top = target.top.max(hole.top);
    let bottom = target.bottom.min(hole.bottom);
    if target.left < hole.left {
        out.push(Area {
            top,
            bottom,
            left: target.left,
            right: hole.left - 1,
        });
    }
    if target.right > hole.right {
        out.push(Area {
            top,
            bottom,
            left: hole.right + 1,
            right: target.right,
        });
    }
    out
}
