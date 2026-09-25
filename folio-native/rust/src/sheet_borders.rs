//! Home > Borders (X04.07): Excel's border presets on a rectangular
//! selection, as one edit of the cells' `border` styles.
use crate::formula::address;
use crate::tables::Area;
use serde_json::{json, Value};

/// Cells one Borders command may change: Excel stores whole-row and
/// whole-column borders as row and column styles, which Folio doesn't edit.
pub const MAX_CELLS: usize = 20_000;

pub fn change(workspace: &Value, r: &Value) -> Result<Value, String> {
    let preset = r["preset"].as_str().ok_or("Choose a border")?;
    if !matches!(
        preset,
        "bottom" | "top" | "left" | "right" | "none" | "all" | "outside" | "thickBox"
    ) {
        return Err("Choose a border".into());
    }
    let line_style = if preset == "thickBox" {
        "thick"
    } else {
        r["line"].as_str().unwrap_or("thin")
    };
    let mut line = json!({"style": line_style});
    if let Some(color) = r["color"].as_str() {
        line["color"] = json!(color);
    }
    // Checks the line style and colour.
    crate::xlsx_styles::validate_format(json!({"border":{"top":line}}).as_object().unwrap())?;
    let area = Area::parse(r["range"].as_str().ok_or("Select cells")?.trim())?;
    let count = (area.bottom - area.top + 1).saturating_mul(area.right - area.left + 1);
    if count > MAX_CELLS {
        return Err("Borders can be applied to up to 20,000 cells at once".into());
    }
    let mut next = workspace.clone();
    let sheet = next["sheets"]
        .as_array_mut()
        .and_then(|s| s.iter_mut().find(|s| s["id"] == r["sheetId"]))
        .ok_or("Worksheet missing")?;
    if !sheet["cellStyles"].is_object() {
        sheet["cellStyles"] = json!({});
    }
    for row in area.top..=area.bottom {
        for column in area.left..=area.right {
            let sides: Vec<&str> = match preset {
                "none" => vec![],
                "all" => vec!["top", "left", "bottom", "right"],
                _ => {
                    let edge = |side: &str| match side {
                        "top" => row == area.top,
                        "bottom" => row == area.bottom,
                        "left" => column == area.left,
                        _ => column == area.right,
                    };
                    let wanted = match preset {
                        "outside" | "thickBox" => vec!["top", "left", "bottom", "right"],
                        one => vec![one],
                    };
                    wanted.into_iter().filter(|s| edge(s)).collect()
                }
            };
            if preset != "none" && sides.is_empty() {
                continue;
            }
            let a = address(row, column);
            let style = &mut sheet["cellStyles"][&a];
            if !style.is_object() {
                *style = json!({});
            }
            if preset == "none" {
                style["border"] = json!({});
            } else {
                if !style["border"].is_object() {
                    style["border"] = json!({});
                }
                for side in sides {
                    style["border"][side] = line.clone();
                }
            }
            if sheet["cells"].get(&a).is_none() {
                sheet["cells"][&a] = json!("");
            }
        }
    }
    Ok(next)
}
