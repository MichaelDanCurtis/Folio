//! The address walk in `targets` must pick exactly the rows the original did.
use crate::row_reflow::{targets, Targets};
use crate::{formula::parse_address, office::*, sheet_geometry::automatic_row};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn targets_reference(
    before: &Value,
    before_values: &Value,
    after: &Value,
    after_values: &Value,
    before_display: &Value,
    after_display: &Value,
    package: Option<&Package>,
) -> Result<Targets, String> {
    let mut result = Targets::new();
    for sheet in after["sheets"].as_array().ok_or("Missing worksheets")? {
        let id = sheet["id"].as_str().ok_or("Missing worksheet id")?;
        let old = before["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or(&Value::Null);
        let broad = [
            "columnWidths",
            "hiddenColumns",
            "defaultColumnWidth",
            "styleDefaults",
            "tables",
            "autoFilter",
            "hiddenRows",
            "defaultRowHeight",
            "defaultRowHeightCustom",
        ]
        .iter()
        .any(|key| old[key] != sheet[key]);
        let mut rows = BTreeSet::new();
        let mut addresses = BTreeSet::new();
        for source in [old, sheet] {
            for key in ["cells", "cellStyles"] {
                addresses.extend(
                    source[key]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(|(a, _)| a),
                );
            }
            if broad {
                for key in ["rowHeights", "rowHeightModes"] {
                    rows.extend(
                        source[key]
                            .as_object()
                            .into_iter()
                            .flatten()
                            .filter_map(|(r, _)| r.parse::<usize>().ok()),
                    );
                }
                rows.extend(
                    source["styleDefaults"]["rows"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .filter_map(|(r, _)| r.parse::<usize>().ok()),
                );
            }
        }
        addresses.extend(
            before_values[id]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(a, _)| a),
        );
        addresses.extend(
            after_values[id]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(a, _)| a),
        );
        for a in addresses {
            if broad
                || old["cells"][a] != sheet["cells"][a]
                || old["cellStyles"][a] != sheet["cellStyles"][a]
                || before_values[id][a] != after_values[id][a]
                || before_display[id][a] != after_display[id][a]
            {
                rows.insert(parse_address(a)?.0 + 1);
            }
        }
        let hidden = crate::tables::hidden_rows(sheet);
        let filtered = crate::tables::filtered_rows(sheet);
        rows.retain(|r| {
            (1..=1048576).contains(r)
                && !hidden.contains(r)
                && !filtered.contains(r)
                && sheet["rowHeights"][r.to_string()].as_f64() != Some(0.0)
                && automatic_row(sheet, &r.to_string())
        });
        if !rows.is_empty() {
            if let (Some(pkg), Some(path)) = (package, sheet["officePath"].as_str()) {
                let doc = parse(pkg.text(path)?)?;
                if let Some(merges) = child(doc.root_element(), "mergeCells") {
                    for node in merges.children().filter(|n| n.is_element()) {
                        let area = crate::tables::Area::parse(
                            attr(node, "ref").ok_or("Invalid merged range")?,
                        )?;
                        rows.retain(|r| *r <= area.top || *r > area.bottom + 1);
                    }
                }
            }
        }
        if !rows.is_empty() {
            result.insert(id.into(), rows);
        }
    }
    Ok(result)
}

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

fn random_map(r: &mut Lcg, values: &[Value]) -> Value {
    let mut m = serde_json::Map::new();
    for _ in 0..r.below(12) {
        let a = format!("{}{}", ["A", "B", "C"][r.below(3) as usize], 1 + r.below(8));
        m.insert(a, values[r.below(values.len() as u64) as usize].clone());
    }
    Value::Object(m)
}

#[test]
fn targets_match_the_original_on_random_workbooks() {
    let mut r = Lcg(17);
    let texts = [json!("1"), json!("x"), json!("=A1")];
    let styles = [json!({"bold":true}), json!({"numFmt":"0.0"})];
    let numbers = [json!(1.0), json!("x"), json!(2.0)];
    let display = [json!({"text":"1"}), json!({"text":"x"})];
    for case in 0..500 {
        let sheet = |r: &mut Lcg| {
            let mut s = json!({"id":"s","name":"S","cells":random_map(r, &texts),
                               "cellStyles":random_map(r, &styles)});
            if r.below(4) == 0 {
                s["columnWidths"] = json!({"1": 10 + r.below(3)});
            }
            if r.below(3) == 0 {
                s["rowHeights"] = json!({"2": 30, "5": 0});
                let mode = ["auto", "manual"][r.below(2) as usize];
                s["rowHeightModes"] = json!({ "2": mode });
            }
            s
        };
        let before = json!({"sheets":[sheet(&mut r)]});
        let after = json!({"sheets":[sheet(&mut r)]});
        let values = |r: &mut Lcg| json!({"s": random_map(r, &numbers)});
        let (bv, av) = (values(&mut r), values(&mut r));
        let shows = |r: &mut Lcg| json!({"s": random_map(r, &display)});
        let (bd, ad) = (shows(&mut r), shows(&mut r));
        assert_eq!(
            targets(&before, &bv, &after, &av, &bd, &ad, None, None),
            targets_reference(&before, &bv, &after, &av, &bd, &ad, None),
            "case {case}"
        );
    }
}
