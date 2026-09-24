use crate::{calculation_clock::CalculationClock, formula::parse_address};
use ironcalc_base::{cell::CellValue, Model};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Instant};

/// Keeps the calculation model alive across document edits and applies only
/// changed inputs. IronCalc owns Excel-style formula parsing and evaluation.
pub struct WorkbookCalculator {
    pub(crate) date1904: bool,
    pub(crate) clock: Option<CalculationClock>,
    pub(crate) clock_override: Option<CalculationClock>,
    force: bool,
    model: Option<Model<'static>>,
    audit: Option<crate::formula_audit::AuditIndex>,
    identity: Vec<(String, String)>,
    inputs: Vec<HashMap<String, String>>,
    table_state: Vec<Value>,
    pub values: Value,
    pub cell_errors: Value,
    pub display: crate::number_format::Display,
    pub generation: u64,
    pub error: Option<String>,
}
impl Default for WorkbookCalculator {
    fn default() -> Self {
        Self {
            date1904: false,
            clock: None,
            clock_override: None,
            force: false,
            model: None,
            audit: None,
            identity: vec![],
            inputs: vec![],
            table_state: vec![],
            values: json!({}),
            cell_errors: json!({}),
            display: Default::default(),
            generation: 0,
            error: None,
        }
    }
}
impl WorkbookCalculator {
    /// A disposable numerical-search model. Preserve the host's ephemeral
    /// calendar and filtered rows as well as the parsed workbook inputs.
    pub fn is_spill_cell(&self, sheet: usize, row: usize, col: usize) -> Result<bool, String> {
        let model = self
            .model
            .as_ref()
            .ok_or("Calculation engine unavailable")?;
        Ok(matches!(
            model
                .workbook
                .worksheet(sheet as u32)?
                .cell(row as i32 + 1, col as i32 + 1),
            Some(ironcalc_base::types::Cell::SpillCell { .. })
        ))
    }
    pub fn isolated_model(&self, sheets: &Value) -> Result<Model<'static>, String> {
        if let Some(error) = &self.error {
            return Err(format!("Calculation engine unavailable: {error}"));
        }
        let mut model = Model::from_workbook(
            self.model
                .as_ref()
                .ok_or("Calculation engine unavailable")?
                .workbook
                .clone(),
            "en",
        )?;
        model.set_date1904(self.date1904);
        if let Some(clock) = self.clock {
            model.set_calculation_clock(clock.millis, clock.offset_seconds)?;
        }
        for (i, sheet) in sheets
            .as_array()
            .ok_or("Invalid sheets")?
            .iter()
            .enumerate()
        {
            model.set_filtered_rows(
                i as u32,
                crate::tables::filtered_rows(sheet)
                    .into_iter()
                    .map(|r| r as i32)
                    .collect(),
            )?;
        }
        Ok(model)
    }

    pub fn audit(
        &mut self,
        sheet_id: &str,
        at: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Value, String> {
        if let Some(error) = &self.error {
            return Err(format!("Calculation engine unavailable: {error}"));
        }
        if self.audit.is_none() {
            self.audit = Some(crate::formula_audit::AuditIndex::build(
                self.model
                    .as_ref()
                    .ok_or("Calculation engine unavailable")?,
                &self.inputs,
            )?);
        }
        let mut result =
            self.audit
                .as_ref()
                .unwrap()
                .query(&self.identity, sheet_id, at, offset, limit)?;
        let index = self
            .identity
            .iter()
            .position(|(id, _)| id == sheet_id)
            .ok_or("Worksheet missing")?;
        result["input"] = json!(self.inputs[index].get(at).cloned().unwrap_or_default());
        result["value"] = self.values[sheet_id][at].clone();
        result["display"] = self.display.values[sheet_id][at]["text"].clone();
        result["isError"] = json!(self.cell_errors[sheet_id][at] == true);
        result["calculationGeneration"] = json!(self.generation);
        Ok(result)
    }
    pub fn expression(&mut self, sheet_id: &str, at: &str, formula: &str) -> Result<Value, String> {
        let sheet = self
            .identity
            .iter()
            .position(|(id, _)| id == sheet_id)
            .ok_or("Validation worksheet missing")?;
        let (row, col) = parse_address(at)?;
        let rows = self
            .model
            .as_mut()
            .ok_or("Calculation engine unavailable")?
            .evaluate_expression_cells(
                formula,
                sheet as u32,
                row as i32 + 1,
                col as i32 + 1,
                32767,
            )?;
        Ok(json!(rows
            .into_iter()
            .map(|row| row
                .into_iter()
                .map(|v| match v {
                    CellValue::None => Value::Null,
                    CellValue::Number(n) => json!(n),
                    CellValue::String(s) => json!(s),
                    CellValue::Boolean(b) => json!(b),
                })
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()))
    }

    #[cfg(test)]
    pub fn sync(&mut self, sheets: &Value) {
        self.sync_calendar(sheets, false);
    }
    pub fn invalidate(&mut self) {
        self.force = true;
    }
    pub fn sync_calendar(&mut self, sheets: &Value, date1904: bool) {
        if let Err(e) = self.update(sheets, date1904) {
            self.audit = None;
            self.error = Some(e);
            self.values = json!({});
            self.cell_errors = json!({});
        }
    }
    fn update(&mut self, sheets: &Value, date1904: bool) -> Result<(), String> {
        let started = Instant::now();
        let clock = self.clock_override.unwrap_or_else(CalculationClock::system);
        let sheets = sheets.as_array().ok_or("Invalid sheets")?;
        let identity: Vec<_> = sheets
            .iter()
            .map(|s| {
                (
                    s["id"].as_str().unwrap_or("").to_string(),
                    s["name"].as_str().unwrap_or("Sheet").to_string(),
                )
            })
            .collect();
        let inputs: Vec<HashMap<String, String>> = sheets
            .iter()
            .map(|s| {
                s["cells"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(a, v)| (a.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .collect();
        let table_state: Vec<_> = sheets
            .iter()
            .map(|s| json!({"tables":s.get("tables"),"hiddenRows":s.get("hiddenRows"),"autoFilter":s.get("autoFilter")}))
            .collect();
        if identity == self.identity
            && inputs == self.inputs
            && table_state == self.table_state
            && date1904 == self.date1904
            && !self.force
            && self.clock.is_some_and(|old| old.same_calendar(clock))
        {
            return Ok(());
        }
        if identity != self.identity
            || self.model.is_none()
            || table_state != self.table_state
            || date1904 != self.date1904
        {
            let mut model = Model::new_empty("Folio", "en", "UTC", "en")?;
            for (i, (_, name)) in identity.iter().enumerate() {
                if i == 0 {
                    model.rename_sheet_by_index(0, name)?;
                } else {
                    model.add_sheet(name)?;
                }
            }
            model.workbook.tables = crate::tables::iron_tables(sheets)?;
            model = Model::from_workbook(model.workbook, "en")?;
            model.set_date1904(date1904);
            for (i, sheet) in sheets.iter().enumerate() {
                model.set_filtered_rows(
                    i as u32,
                    crate::tables::filtered_rows(sheet)
                        .into_iter()
                        .map(|r| r as i32)
                        .collect(),
                )?;
                for row in crate::tables::hidden_rows(sheet) {
                    model.set_row_hidden(i as u32, row as i32, true)?;
                }
            }
            self.model = Some(model);
            self.inputs = vec![HashMap::new(); sheets.len()];
        }
        crate::perf_stages::record("calc.inputs", started);
        let started = Instant::now();
        let model = self.model.as_mut().unwrap();
        let table_names: Vec<String> = model.workbook.tables.keys().cloned().collect();
        for (i, cells) in inputs.iter().enumerate() {
            let old = &self.inputs[i];
            for address in old
                .keys()
                .chain(cells.keys())
                .collect::<std::collections::HashSet<_>>()
            {
                if old.get(address) == cells.get(address) {
                    continue;
                }
                let (r, c) = parse_address(address)?;
                let mut input = cells.get(address).cloned().unwrap_or_default();
                if input.starts_with('=') {
                    let context =
                        crate::tables::at(&sheets[i], r, c).and_then(|t| t["name"].as_str());
                    input =
                        crate::table_refs::qualify(&input, context, &table_names).unwrap_or(input);
                }
                model.set_user_input(i as u32, r as i32 + 1, c as i32 + 1, input)?;
            }
        }
        model.set_calculation_clock(clock.millis, clock.offset_seconds)?;
        crate::perf_stages::record("calc.set", started);
        crate::perf_stages::time("calc.evaluate", || model.evaluate());
        let started = Instant::now();
        let mut all = serde_json::Map::new();
        let mut all_errors = serde_json::Map::new();
        for (i, cells) in inputs.iter().enumerate() {
            let mut values = serde_json::Map::new();
            let mut errors = serde_json::Map::new();
            for a in cells.keys() {
                let (r, c) = parse_address(a)?;
                let v = match model.get_cell_value_by_index(i as u32, r as i32 + 1, c as i32 + 1)? {
                    CellValue::None => json!(""),
                    CellValue::Number(n) => json!(n),
                    CellValue::String(s) => json!(s),
                    CellValue::Boolean(b) => json!(b),
                };
                if model.get_cell_type(i as u32, r as i32 + 1, c as i32 + 1)?
                    == ironcalc_base::types::CellType::ErrorValue
                {
                    errors.insert(a.clone(), json!(true));
                }
                values.insert(a.clone(), v);
            }
            all.insert(identity[i].0.clone(), Value::Object(values));
            all_errors.insert(identity[i].0.clone(), Value::Object(errors));
        }
        crate::perf_stages::record("calc.collect", started);
        self.values = Value::Object(all);
        self.cell_errors = Value::Object(all_errors);
        self.date1904 = date1904;
        self.identity = identity;
        self.inputs = inputs;
        self.table_state = table_state;
        self.clock = Some(clock);
        self.force = false;
        self.generation += 1;
        self.audit = None;
        self.error = None;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excel_formulas_cross_sheets_and_cached_document_changes() {
        let sheets = json!([{"id":"a","name":"Inputs","cells":{"A1":"10","A2":"20","B1":"=IF(SUM(A1:A2)>25,ROUND(AVERAGE(A1:A2),2),0)","AA1":"=COUNTIF(A1:A2,\">12\")"}},{"id":"b","name":"Summary","cells":{"A1":"=Inputs!B1*2","B1":"=VLOOKUP(20,Inputs!A1:A2,1,FALSE)"}}]);
        let mut calc = WorkbookCalculator::default();
        calc.sync(&sheets);
        assert_eq!(calc.error, None);
        assert_eq!(calc.values["b"]["A1"], json!(30.0));
        assert_eq!(calc.values["a"]["AA1"], json!(1.0));
        assert_eq!(calc.values["b"]["B1"], json!(20.0));
        let g = calc.generation;
        calc.sync(&sheets);
        assert_eq!(calc.generation, g);
    }
}
