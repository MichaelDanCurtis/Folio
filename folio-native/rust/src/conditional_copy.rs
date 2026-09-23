//! Conditional rules follow immutable clipboard/fill sources. Destination
//! subtraction is geometric, so whole-column rules remain sparse.
use crate::{
    conditional_format as cf,
    data_validation::{compact_cells, subtract},
    formula::{address, parse_address},
    office::parse,
    structure::copy_formula,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn names(sheets: &Value) -> Vec<String> {
    sheets
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_owned())
        .collect()
}
fn reanchor(
    rule: &mut Value,
    names: &[String],
    index: usize,
    from: &str,
    to: &str,
    destination: Option<(&[String], usize)>,
) -> Result<(), String> {
    let shift = |value: &mut Value| -> Result<(), String> {
        let formula = value.as_str().ok_or("Conditional formula is missing")?;
        let shifted = copy_formula(formula.trim_start_matches('='), names, index, from, to)?;
        if let Some((target_names, target_index)) = destination {
            let checked = copy_formula(&shifted, target_names, target_index, to, to)?;
            if checked != shifted
                || (names != target_names
                    && crate::structure::missing_formula_sheet(
                        &shifted,
                        target_names,
                        target_index,
                        to,
                    )?)
            {
                return Err("A copied conditional formula refers to sheets or names unavailable in this workbook. Nothing changed.".into());
            }
        }
        *value = json!(shifted);
        Ok(())
    };
    if let Some(formulas) = rule.get_mut("formulas").and_then(Value::as_array_mut) {
        for formula in formulas {
            shift(formula)?;
        }
    }
    for path in ["/scale", "/dataBar/thresholds", "/iconSet/thresholds"] {
        if let Some(points) = rule.pointer_mut(path).and_then(Value::as_array_mut) {
            for point in points {
                if point["type"] == "formula" {
                    shift(&mut point["value"])?;
                }
            }
        }
    }
    Ok(())
}
fn portable(rule: &Value) -> Result<(), String> {
    if rule["unsupported"] == true || rule["groupExtras"].as_str().is_some_and(|s| !s.is_empty()) {
        return Err("Copying this conditional rule's additional Excel settings needs support. Nothing changed.".into());
    }
    // Copying a new rule cannot duplicate unmodeled identifiers/references.
    for (key, allowed) in [
        ("groupXml", &["sqref", "pivot"][..]),
        (
            "officeXml",
            &[
                "type",
                "operator",
                "priority",
                "stopIfTrue",
                "text",
                "rank",
                "percent",
                "bottom",
                "aboveAverage",
                "equalAverage",
                "stdDev",
                "timePeriod",
                "dxfId",
            ][..],
        ),
    ] {
        if let Some(xml) = rule[key].as_str() {
            let doc = parse(xml)?;
            if doc
                .root_element()
                .attributes()
                .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
            {
                return Err("Copying this conditional rule's additional Excel attributes needs support. Nothing changed.".into());
            }
        }
    }
    cf::validate_rule(rule)
}

pub fn copy_rules(
    source_sheets: &Value,
    source_index: usize,
    target: &mut Value,
    target_names: &[String],
    target_index: usize,
    copies: &[(String, String)],
    merge: bool,
) -> Result<(), String> {
    let source = &source_sheets[source_index];
    if copies.is_empty() {
        return Ok(());
    }
    if source["extendedConditionalFormats"] == true {
        return Err("Copying this worksheet's unmodeled extended conditional rules needs support. Nothing changed.".into());
    }
    if cf::rules(source).next().is_none()
        && cf::rules(target).next().is_none()
        && target["extendedConditionalFormats"] != true
    {
        return Ok(());
    }
    let source_names = names(source_sheets);
    let destinations = compact_cells(&copies.iter().map(|(_, to)| to.clone()).collect::<Vec<_>>())?;
    // A fill can reuse one source cell thousands of times. Test its membership
    // only once, retaining every destination and the original snapshot.
    let mut mappings: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for (from, to) in copies {
        mappings
            .entry(parse_address(from)?)
            .or_default()
            .push(to.clone());
    }
    let mut copied = vec![];
    for original in cf::rules(source) {
        let areas = cf::areas(original)?;
        let mut cells = vec![];
        for ((r, c), destinations) in &mappings {
            if areas.iter().any(|a| a.contains(*r, *c)) {
                cells.extend(destinations.iter().cloned());
            }
        }
        if cells.is_empty() {
            continue;
        }
        portable(original)?;
        let areas_new = compact_cells(&cells)?;
        let mut rule = original.clone();
        reanchor(
            &mut rule,
            &source_names,
            source_index,
            &address(areas[0].top, areas[0].left),
            &address(areas_new[0].top, areas_new[0].left),
            Some((target_names, target_index)),
        )?;
        rule["sqref"] = json!(areas_new
            .iter()
            .map(|a| a.text())
            .collect::<Vec<_>>()
            .join(" "));
        for key in ["officeDataBarId", "officeIconSetId"] {
            rule.as_object_mut().unwrap().remove(key);
        }
        copied.push(rule);
    }
    let mut retained = vec![];
    for original in cf::rules(target) {
        if merge {
            retained.push(original.clone());
            continue;
        }
        let old = cf::areas(original)?;
        let mut remaining = old.clone();
        for area in &destinations {
            remaining = remaining
                .into_iter()
                .flat_map(|a| subtract(a, *area))
                .collect();
        }
        if remaining == old {
            retained.push(original.clone());
            continue;
        }
        if remaining.is_empty() {
            continue;
        }
        if original["unsupported"] == true {
            return Err(
                "Part of the destination has an unmodeled conditional rule. Nothing changed."
                    .into(),
            );
        }
        let mut rule = original.clone();
        reanchor(
            &mut rule,
            target_names,
            target_index,
            &address(old[0].top, old[0].left),
            &address(remaining[0].top, remaining[0].left),
            None,
        )?;
        rule["sqref"] = json!(remaining
            .iter()
            .map(|a| a.text())
            .collect::<Vec<_>>()
            .join(" "));
        retained.push(rule);
    }
    if copied.is_empty() && retained == cf::rules(target).cloned().collect::<Vec<_>>() {
        return Ok(());
    }
    if target["extendedConditionalFormats"] == true {
        return Err("Changing this worksheet's unmodeled extended conditional rules needs support. Nothing changed.".into());
    }
    copied.extend(retained);
    if copied.len() > 4096 {
        return Err("Pasting would exceed the conditional-rule limit. Nothing changed.".into());
    }
    for (i, rule) in copied.iter_mut().enumerate() {
        rule["priority"] = json!(i + 1);
    }
    let mut candidate = target.clone();
    candidate["conditionalFormats"] = json!(copied);
    cf::validate(&candidate)?;
    *target = candidate;
    Ok(())
}
