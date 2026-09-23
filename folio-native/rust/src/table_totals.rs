use super::*;

pub(super) fn capture(column: &mut Value, value: &str, table_name: &str) {
    column.as_object_mut().unwrap().remove("totalsFormula");
    column.as_object_mut().unwrap().remove("totalsLabel");
    if value.starts_with('=') {
        let function = [
            "sum",
            "average",
            "count",
            "countNums",
            "max",
            "min",
            "stdDev",
            "var",
        ]
        .into_iter()
        .find(|function| {
            value
                == format!(
                    "=SUBTOTAL({},{}[{}])",
                    subtotal_number(function).unwrap(),
                    table_name,
                    table_refs::escaped(column["name"].as_str().unwrap())
                )
        });
        column["totalsFunction"] = json!(function.unwrap_or("custom"));
        if function.is_none() {
            column["totalsFormula"] = json!(value);
        }
    } else {
        column["totalsFunction"] = json!("none");
        if !value.is_empty() {
            column["totalsLabel"] = json!(value.strip_prefix('\'').unwrap_or(value));
        }
    }
}

fn value(column: &Value, table_name: &str) -> Result<String, String> {
    let function = column["totalsFunction"].as_str().unwrap_or("none");
    if function == "custom" {
        return column["totalsFormula"]
            .as_str()
            .filter(|f| f.starts_with('=') && f.len() <= 10000)
            .map(String::from)
            .ok_or(
                "This total has no stored formula. Enter a custom total formula before showing it."
                    .into(),
            );
    }
    let code = subtotal_number(function)?;
    Ok(if code == 0 {
        column["totalsLabel"]
            .as_str()
            .map(text_cell)
            .unwrap_or_default()
    } else {
        format!(
            "=SUBTOTAL({code},{table_name}[{}])",
            table_refs::escaped(column["name"].as_str().unwrap())
        )
    })
}

pub(super) fn change(
    next: &mut Value,
    si: usize,
    ti: usize,
    request: &Value,
) -> Result<(), String> {
    let mut table = next["sheets"][si]["tables"][ti].clone();
    let mut area = area(&table)?;
    let name = table["name"].as_str().unwrap().to_string();
    let shown = table["totalsRows"] == 1;
    let explicit = request["action"] == "totalFunction";
    let enabled = if explicit {
        true
    } else {
        request["enabled"]
            .as_bool()
            .ok_or("Choose whether to show totals")?
    };
    // Preserve the actual totals cells, including old workspaces that did not
    // project custom totalsRowFormula into their saved metadata.
    if shown {
        for (ci, column) in table["columns"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            let raw = next["sheets"][si]["cells"][address(area.bottom, area.left + ci)]
                .as_str()
                .unwrap_or("");
            capture(column, raw, &name);
        }
    }
    let configured = table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c.get("totalsFunction").is_some() || c.get("totalsLabel").is_some());
    if explicit || (enabled && !configured) {
        let ci = request["column"]
            .as_u64()
            .unwrap_or((area.right - area.left) as u64) as usize;
        if ci > area.right - area.left {
            return Err("Totals column is out of range".into());
        }
        if !configured && area.right > area.left {
            table["columns"][0]["totalsLabel"] = json!("Total");
        }
        let function = request["function"].as_str().unwrap_or("sum");
        let c = &mut table["columns"][ci];
        c["totalsFunction"] = json!(function);
        c.as_object_mut().unwrap().remove("totalsLabel");
        if function == "custom" {
            let formula = request["formula"]
                .as_str()
                .filter(|f| f.starts_with('=') && f.len() <= 10000)
                .ok_or("Enter a total formula beginning with =")?;
            c["totalsFormula"] = json!(formula);
        } else {
            subtotal_number(function)?;
            c.as_object_mut().unwrap().remove("totalsFormula");
        }
    }
    if enabled && !shown {
        if area.bottom >= 1048575 {
            return Err("No room for a totals row".into());
        }
        area.bottom += 1;
        if (area.left..=area.right).any(|c| {
            next["sheets"][si]["cells"][address(area.bottom, c)]
                .as_str()
                .is_some_and(|v| !v.is_empty())
        }) {
            return Err(
                "Insert an empty worksheet row below this table before adding totals".into(),
            );
        }
    }
    if enabled {
        for (ci, column) in table["columns"].as_array().unwrap().iter().enumerate() {
            next["sheets"][si]["cells"][address(area.bottom, area.left + ci)] =
                json!(value(column, &name)?);
        }
    } else if shown {
        for c in area.left..=area.right {
            next["sheets"][si]["cells"]
                .as_object_mut()
                .unwrap()
                .remove(&address(area.bottom, c));
        }
        area.bottom = area.bottom.saturating_sub(1).max(area.top);
    }
    table["range"] = json!(area.text());
    table["totalsRows"] = json!(if enabled { 1 } else { 0 });
    next["sheets"][si]["tables"][ti] = table;
    Ok(())
}
