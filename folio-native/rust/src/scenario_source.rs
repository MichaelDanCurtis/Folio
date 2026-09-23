//! Frozen, read-only source for cross-workbook scenario merging.
use crate::{office::*, scenarios};
use serde_json::{json, Value};
pub struct Source {
    pub token: u64,
    sheets: Value,
    package: Package,
}
impl Source {
    pub fn read(path: &str, token: u64) -> Result<Self, String> {
        let package = Package::read(path)?;
        let (sheets, _) = crate::xlsx::import(&package)?;
        Ok(Self {
            token,
            sheets,
            package,
        })
    }
    pub fn preview(&self) -> Value {
        let sheets: Vec<_> = self
            .sheets
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| scenarios::items(s).next().is_some())
            .map(|s| {
                json!({"id":s["id"],"name":s["name"],"scenarios":s[scenarios::KEY],
                "unavailableReason":scenarios::check_sheet(s,Some(&self.package)).err()})
            })
            .collect();
        json!({"token":self.token,"sheets":sheets})
    }
    pub fn merge(
        &self,
        workspace: &Value,
        request: &Value,
        target: Option<&Package>,
    ) -> Result<Value, String> {
        if request["sourceToken"] != self.token {
            return Err("The source workbook expired. Choose it again.".into());
        }
        let source = self
            .sheets
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == request["sourceSheetId"])
            .ok_or("Source worksheet missing")?;
        scenarios::check_sheet(source, Some(&self.package))?;
        let mut selected = Vec::new();
        for index in request["indices"]
            .as_array()
            .ok_or("Choose scenarios to copy")?
        {
            let mut scenario = scenarios::selected(source, &json!({"index":index}))?.clone();
            for input in scenario["inputs"].as_array_mut().unwrap() {
                if let Some(id) = input["numFmtId"].as_u64().filter(|n| *n >= 164) {
                    let xml = self.package.text("xl/styles.xml")?;
                    let doc = parse(xml)?;
                    let record = child(doc.root_element(), "numFmts")
                        .and_then(|n| {
                            n.children().find(|n| {
                                attr(*n, "numFmtId").and_then(|v| v.parse::<u64>().ok()) == Some(id)
                            })
                        })
                        .ok_or("The source scenario's custom number format is missing")?;
                    input["_numberFormat"] = json!(standalone_xml(xml, record));
                }
            }
            selected.push(scenario);
        }
        scenarios::merge_items(workspace, request, target, selected)
    }
}
