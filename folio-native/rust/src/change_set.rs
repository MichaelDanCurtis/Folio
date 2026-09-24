//! What a commit changed, in the terms the calculator and display care about.
use crate::json_patch::{Key, Op, Patch};

#[derive(Clone, Debug, PartialEq)]
pub enum ChangeSet {
    /// Only `sheets[i].cells[address]` values were set or removed; the sheet
    /// list and everything else in the workspace are unchanged.
    Cells(Vec<(usize, String)>),
    /// Anything else: callers take their full path.
    Structural,
}

impl ChangeSet {
    pub fn from_patch(patch: &Patch) -> Self {
        let mut cells = Vec::with_capacity(patch.ops.len());
        for op in &patch.ops {
            let path = match op {
                Op::Set { path, .. } | Op::Remove { path } => path,
            };
            match path.as_slice() {
                [Key::Field(sheets), Key::Index(i), Key::Field(field), Key::Field(address)]
                    if sheets == "sheets" && field == "cells" =>
                {
                    cells.push((*i, address.clone()))
                }
                _ => return Self::Structural,
            }
        }
        Self::Cells(cells)
    }
}

#[cfg(test)]
mod tests {
    use super::ChangeSet;
    use crate::json_patch::diff;
    use serde_json::{json, Value};

    fn workbook() -> Value {
        json!({"revision":1,"title":"t","document":{"type":"doc"},"sheets":[
            {"id":"a","name":"A","cells":{"A1":"1","B1":"=A1"}},
            {"id":"b","name":"B","cells":{"A1":"x"},"cellStyles":{"A1":{"bold":true}}}
        ]})
    }
    fn changes(edit: impl FnOnce(&mut Value)) -> ChangeSet {
        let before = workbook();
        let mut after = before.clone();
        edit(&mut after);
        ChangeSet::from_patch(&diff(&after, &before).without_field("revision"))
    }

    #[test]
    fn cell_values_set_or_cleared_are_cells() {
        assert_eq!(
            changes(|w| w["sheets"][1]["cells"]["A1"] = json!("y")),
            ChangeSet::Cells(vec![(1, "A1".into())])
        );
        assert_eq!(
            changes(|w| {
                w["sheets"][0]["cells"]
                    .as_object_mut()
                    .unwrap()
                    .remove("B1");
                w["sheets"][0]["cells"]["C3"] = json!("=B1");
            }),
            ChangeSet::Cells(vec![(0, "C3".into()), (0, "B1".into())])
        );
        assert_eq!(
            changes(|w| w["revision"] = json!(9)),
            ChangeSet::Cells(vec![])
        );
    }

    #[test]
    fn anything_else_is_structural() {
        let structural: Vec<Box<dyn FnOnce(&mut Value)>> = vec![
            Box::new(|w| w["sheets"][1]["cellStyles"]["A1"]["bold"] = json!(false)),
            Box::new(|w| w["sheets"][0]["name"] = json!("Renamed")),
            Box::new(|w| w["title"] = json!("u")),
            Box::new(|w| w["document"]["content"] = json!([])),
            Box::new(|w| w["sheets"][0]["tables"] = json!([])),
            Box::new(|w| w["sheets"][0]["hiddenRows"] = json!([2])),
            Box::new(|w| w["sheets"][0]["autoFilter"] = json!({})),
            Box::new(|w| w["sheets"][0]["mergedRanges"] = json!(["A1:B1"])),
            Box::new(|w| w["sheets"][0]["conditionalFormats"] = json!([])),
            Box::new(|w| w["sheets"][0]["cells"] = json!("broken")),
            Box::new(|w| {
                w["sheets"][0]["cells"]["A1"] = json!("2");
                w["sheets"][0]["rowHeights"] = json!({"1":20});
            }),
            Box::new(|w| w["sheets"].as_array_mut().unwrap().pop().map(drop).unwrap()),
        ];
        for (i, edit) in structural.into_iter().enumerate() {
            assert_eq!(changes(edit), ChangeSet::Structural, "case {i}");
        }
    }
}
