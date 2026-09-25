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
    /// Per sheet, the addresses (and parsed positions) whose input is a formula.
    formulas: Vec<HashMap<String, (usize, usize)>>,
    /// The next update must rebuild the model from scratch.
    rebuild: bool,
    pub changed: Changed,
    table_state: Vec<Value>,
    /// Defined names formulas can use (from the workbook package), and the
    /// ones the current model was built with.
    pub(crate) names: Vec<crate::xlsx_names::Name>,
    applied_names: Vec<crate::xlsx_names::Name>,
    /// Defined names the engine refused, for the workbook warning.
    pub(crate) name_errors: Vec<String>,
    pub values: Value,
    pub cell_errors: Value,
    pub display: crate::number_format::Display,
    pub generation: u64,
    pub error: Option<String>,
    /// Identifies this calculator and counts its updates. Unlike `generation`,
    /// which staged layout edits overwrite, this pair never repeats, so it can
    /// key caches of calculated results.
    instance: u64,
    updates: u64,
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
            formulas: vec![],
            rebuild: false,
            changed: Changed::All,
            table_state: vec![],
            names: vec![],
            applied_names: vec![],
            name_errors: vec![],
            values: json!({}),
            cell_errors: json!({}),
            display: Default::default(),
            generation: 0,
            error: None,
            instance: {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            },
            updates: 0,
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
    /// Brings the model up to date with `sheets`, finding changed inputs by
    /// comparing every cell.
    pub fn sync_calendar(&mut self, sheets: &Value, date1904: bool) {
        self.sync_changes(sheets, date1904, None);
    }
    /// Like `sync_calendar`, but when `hint` is given only those cells can
    /// have changed input since the last sync, so the rest are not compared.
    /// Changes whenever the calculator evaluates (values, errors or the
    /// calculation clock may have changed).
    pub fn version(&self) -> (u64, u64) {
        (self.instance, self.updates)
    }
    pub fn sync_changes(
        &mut self,
        sheets: &Value,
        date1904: bool,
        hint: Option<&[(usize, String)]>,
    ) {
        let result = self.update(sheets, date1904, hint);
        if result.is_err() {
            self.updates += 1;
        }
        if let Err(e) = result {
            self.audit = None;
            self.error = Some(e);
            self.values = json!({});
            self.cell_errors = json!({});
            self.changed = Changed::All;
            // The model may hold part of the failed update; start over next time.
            self.rebuild = true;
        }
    }
    fn update(
        &mut self,
        sheets: &Value,
        date1904: bool,
        hint: Option<&[(usize, String)]>,
    ) -> Result<(), String> {
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
        let table_state: Vec<_> = sheets
            .iter()
            .map(|s| json!({"tables":s.get("tables"),"hiddenRows":s.get("hiddenRows"),"autoFilter":s.get("autoFilter")}))
            .collect();
        let rebuild = self.rebuild
            || identity != self.identity
            || self.model.is_none()
            || table_state != self.table_state
            || self.names != self.applied_names
            || date1904 != self.date1904;
        let input = |i: usize, a: &str| -> Option<&str> {
            sheets[i]["cells"].get(a).map(|v| v.as_str().unwrap_or(""))
        };
        let changed: Vec<(usize, String)> = if rebuild {
            crate::perf_stages::record("full.calc_rebuild", Instant::now());
            sheets
                .iter()
                .enumerate()
                .flat_map(|(i, s)| {
                    s["cells"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(move |(a, _)| (i, a.clone()))
                })
                .collect()
        } else if let Some(hint) = hint {
            hint.iter()
                .filter(|(i, a)| self.inputs[*i].get(a).map(String::as_str) != input(*i, a))
                .cloned()
                .collect()
        } else {
            crate::perf_stages::record("full.calc_scan", Instant::now());
            let mut changed = vec![];
            for (i, sheet) in sheets.iter().enumerate() {
                let cells = sheet["cells"].as_object().ok_or("Invalid cells")?;
                let old = &self.inputs[i];
                for (a, v) in cells {
                    if old.get(a).map(String::as_str) != Some(v.as_str().unwrap_or("")) {
                        changed.push((i, a.clone()));
                    }
                }
                changed.extend(
                    old.keys()
                        .filter(|a| !cells.contains_key(*a))
                        .map(|a| (i, a.clone())),
                );
            }
            changed
        };
        if !rebuild
            && changed.is_empty()
            && !self.force
            && self.clock.is_some_and(|old| old.same_calendar(clock))
        {
            self.changed = Changed::Cells(vec![]);
            return Ok(());
        }
        if rebuild {
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
            // Names come before any cell input, so formulas parse against them.
            self.name_errors.clear();
            for (name, scope, formula) in &self.names {
                if let Err(e) = model.new_defined_name(name, scope.map(|i| i as u32), formula) {
                    self.name_errors.push(format!("{name}: {e}"));
                }
            }
            self.applied_names = self.names.clone();
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
            self.formulas = vec![HashMap::new(); sheets.len()];
            self.values = Value::Object(
                identity
                    .iter()
                    .map(|(id, _)| (id.clone(), json!({})))
                    .collect(),
            );
            self.cell_errors = self.values.clone();
        }
        crate::perf_stages::record("calc.inputs", started);
        let started = Instant::now();
        let model = self.model.as_mut().unwrap();
        let table_names: Vec<String> = model.workbook.tables.keys().cloned().collect();
        let sheet_names: Vec<String> = identity.iter().map(|(_, name)| name.clone()).collect();
        let mut positions = Vec::with_capacity(changed.len());
        for (i, address) in &changed {
            let (r, c) = parse_address(address)?;
            positions.push((r, c));
            let mut value = input(*i, address).unwrap_or("").to_string();
            match input(*i, address) {
                Some(v) => self.inputs[*i].insert(address.clone(), v.to_string()),
                None => self.inputs[*i].remove(address),
            };
            if value.starts_with('=') {
                self.formulas[*i].insert(address.clone(), (r, c));
                let context = crate::tables::at(&sheets[*i], r, c).and_then(|t| t["name"].as_str());
                value = crate::table_refs::qualify(&value, context, &table_names).unwrap_or(value);
                if let Some(expanded) = crate::sheet_spans::expand(&value, &sheet_names) {
                    value = expanded;
                }
            } else {
                self.formulas[*i].remove(address);
            }
            model.set_user_input(*i as u32, r as i32 + 1, c as i32 + 1, value)?;
        }
        model.set_calculation_clock(clock.millis, clock.offset_seconds)?;
        crate::perf_stages::record("calc.set", started);
        crate::perf_stages::time("calc.evaluate", || model.evaluate());
        let started = Instant::now();
        // Only edited cells and formulas can hold a new value after evaluation.
        let mut read: Vec<(usize, &str, usize, usize)> = changed
            .iter()
            .zip(&positions)
            .filter(|((i, a), _)| !self.formulas[*i].contains_key(a))
            .map(|((i, a), (r, c))| (*i, a.as_str(), *r, *c))
            .collect();
        for (i, formulas) in self.formulas.iter().enumerate() {
            read.extend(formulas.iter().map(|(a, (r, c))| (i, a.as_str(), *r, *c)));
        }
        let mut updated = vec![];
        for (i, a, r, c) in read {
            let id = identity[i].0.as_str();
            let values = self.values[id].as_object_mut().ok_or("Invalid values")?;
            let errors = self.cell_errors[id]
                .as_object_mut()
                .ok_or("Invalid errors")?;
            let (value, error) = if self.inputs[i].contains_key(a) {
                let v = match model.get_cell_value_by_index(i as u32, r as i32 + 1, c as i32 + 1)? {
                    CellValue::None => json!(""),
                    CellValue::Number(n) => json!(n),
                    CellValue::String(s) => json!(s),
                    CellValue::Boolean(b) => json!(b),
                };
                let error = model.get_cell_type(i as u32, r as i32 + 1, c as i32 + 1)?
                    == ironcalc_base::types::CellType::ErrorValue;
                (Some(v), error)
            } else {
                (None, false)
            };
            let same_value = values.get(a) == value.as_ref();
            let same_error = errors.contains_key(a) == error;
            if same_value && same_error {
                continue;
            }
            match value {
                Some(v) => values.insert(a.to_string(), v),
                None => values.remove(a),
            };
            if error {
                errors.insert(a.to_string(), json!(true));
            } else {
                errors.remove(a);
            }
            updated.push((i, a.to_string()));
        }
        crate::perf_stages::record("calc.collect", started);
        self.changed = if rebuild {
            Changed::All
        } else {
            Changed::Cells(updated)
        };
        self.rebuild = false;
        self.date1904 = date1904;
        self.identity = identity;
        self.table_state = table_state;
        self.clock = Some(clock);
        self.force = false;
        self.generation += 1;
        self.updates += 1;
        self.audit = None;
        self.error = None;
        Ok(())
    }
}

/// Cells whose calculated value or error flag changed in the last sync.
#[derive(Clone, Debug, PartialEq)]
pub enum Changed {
    All,
    Cells(Vec<(usize, String)>),
}
impl Default for Changed {
    fn default() -> Self {
        Self::All
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
