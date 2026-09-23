//! Incremental, disposable one-input numerical search. Trials never enter
//! workspace history; an accepted value uses the ordinary measured cell edit.
use crate::{
    calculation::WorkbookCalculator, formula::parse_address, office::Package, tables::Area,
};
use ironcalc_base::{cell::CellValue, types::CellType, Model};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Cell {
    pub sheet_id: String,
    pub address: String,
    sheet: usize,
    row: usize,
    col: usize,
}
impl Cell {
    fn read(w: &Value, request: &Value) -> Result<Self, String> {
        let sheet_id = request["sheetId"].as_str().ok_or("Choose a worksheet")?;
        let address = request["address"]
            .as_str()
            .ok_or("Enter a cell address")?
            .trim()
            .replace('$', "")
            .to_uppercase();
        let (row, col) = parse_address(&address)?;
        let sheet = w["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .position(|s| s["id"] == sheet_id)
            .ok_or("Worksheet missing")?;
        Ok(Self {
            sheet_id: sheet_id.into(),
            address,
            sheet,
            row,
            col,
        })
    }
    fn result(&self, model: &Model<'_>) -> Result<f64, String> {
        if model.get_cell_type(self.sheet as u32, self.row as i32 + 1, self.col as i32 + 1)?
            == CellType::ErrorValue
        {
            return Err("The formula returned a calculation error for this trial.".into());
        }
        match model.get_cell_value_by_index(
            self.sheet as u32,
            self.row as i32 + 1,
            self.col as i32 + 1,
        )? {
            CellValue::Number(n) if n.is_finite() => Ok(n),
            _ => Err("The formula did not return a finite number for this trial.".into()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
    f: f64,
}

pub struct Session {
    pub token: u64,
    workspace_id: Value,
    revision: Value,
    generation: u64,
    pub target: Cell,
    changing: Cell,
    original: String,
    start: f64,
    goal: f64,
    tolerance: f64,
    limit: usize,
    model: Model<'static>,
    points: Vec<Point>,
    tried: Vec<f64>,
    best: Option<Point>,
    verifying: Option<Point>,
    expansion: u32,
    status: &'static str,
    reason: String,
    last_error: Option<String>,
}

#[derive(Clone)]
pub struct Accepted {
    pub edit: Value,
    pub unchanged: bool,
    target: Cell,
    result: f64,
    tolerance: f64,
}
impl Accepted {
    pub fn check(&self, calc: &WorkbookCalculator) -> Result<(), String> {
        let value = &calc.values[&self.target.sheet_id][&self.target.address];
        if calc.cell_errors[&self.target.sheet_id][&self.target.address] == true
            || value
                .as_f64()
                .is_none_or(|n| !n.is_finite() || (n - self.result).abs() > self.tolerance)
        {
            return Err(
                "The formula result changed while applying Goal Seek. Run the search again.".into(),
            );
        }
        Ok(())
    }
}

impl Session {
    pub fn new(
        w: &Value,
        calc: &WorkbookCalculator,
        pkg: Option<&Package>,
        r: &Value,
        token: u64,
    ) -> Result<Self, String> {
        if r["workspaceId"] != w["id"]
            || r["baseRevision"] != w["revision"]
            || r["calculationGeneration"] != calc.generation
        {
            return Err("The workbook changed. Reopen Goal Seek.".into());
        }
        let target = Cell::read(w, &r["target"])?;
        let changing = Cell::read(w, &r["changing"])?;
        let goal = r["goal"]
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or("Enter a finite target value")?;
        let tolerance = match r.get("tolerance") {
            Some(v) => v.as_f64().ok_or("Enter a numerical maximum difference")?,
            None => 0.000001,
        };
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("Maximum difference must be greater than zero".into());
        }
        let limit = match r.get("maxTrials") {
            Some(v) => v.as_u64().ok_or("Enter a whole number of trials")? as usize,
            None => 200,
        };
        if !(4..=10000).contains(&limit) {
            return Err("Choose 4–10,000 trials".into());
        }
        if !w["sheets"][target.sheet]["cells"][&target.address]
            .as_str()
            .is_some_and(|s| s.starts_with('='))
        {
            return Err("Set cell must contain a formula".into());
        }
        let original = w["sheets"][changing.sheet]["cells"][&changing.address]
            .as_str()
            .unwrap_or("")
            .to_owned();
        if original.starts_with('=') {
            return Err("By changing cell must contain a number or be blank, not a formula".into());
        }
        let start = if original.is_empty() {
            0.0
        } else {
            calc.values[&changing.sheet_id][&changing.address]
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or("By changing cell must contain a number or be blank")?
        };
        check_editable(&w["sheets"][changing.sheet], &changing, pkg)?;
        let model = calc.isolated_model(&w["sheets"])?;
        if matches!(
            model
                .workbook
                .worksheet(changing.sheet as u32)?
                .cell(changing.row as i32 + 1, changing.col as i32 + 1),
            Some(ironcalc_base::types::Cell::SpillCell { .. })
        ) {
            return Err("A spilled result cannot be the changing cell".into());
        }
        Ok(Self {
            token,
            workspace_id: w["id"].clone(),
            revision: w["revision"].clone(),
            generation: calc.generation,
            target,
            changing,
            original,
            start,
            goal,
            tolerance,
            limit,
            model,
            points: vec![],
            tried: vec![],
            best: None,
            verifying: None,
            expansion: 0,
            status: "running",
            reason: String::new(),
            last_error: None,
        })
    }
    pub fn check(&self, w: &Value, generation: u64, token: &Value) -> Result<(), String> {
        if *token != self.token
            || self.workspace_id != w["id"]
            || self.revision != w["revision"]
            || self.generation != generation
        {
            return Err(
                "This Goal Seek result expired because the workbook changed. Run it again.".into(),
            );
        }
        Ok(())
    }
    fn evaluate(&mut self, x: f64) -> Result<f64, String> {
        self.model.set_user_input(
            self.changing.sheet as u32,
            self.changing.row as i32 + 1,
            self.changing.col as i32 + 1,
            x.to_string(),
        )?;
        self.model.evaluate();
        self.target.result(&self.model)
    }
    fn fresh(&self, x: f64) -> bool {
        x.is_finite() && !self.tried.contains(&x)
    }
    fn next(&mut self) -> Option<f64> {
        if self.tried.is_empty() {
            return Some(self.start);
        }
        // When a sign change is available, keep the next trial strictly inside
        // that bracket. Fall back to bisection when interpolation hugs an edge.
        let mut sorted = self.points.clone();
        sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
        let bracket = sorted
            .windows(2)
            .filter(|p| p[0].f.is_sign_positive() != p[1].f.is_sign_positive())
            .min_by(|a, b| (a[1].x - a[0].x).total_cmp(&(b[1].x - b[0].x)));
        if let Some(pair) = bracket {
            let (a, b) = (pair[0], pair[1]);
            let midpoint = a.x / 2.0 + b.x / 2.0;
            let secant = a.x + (b.x - a.x) * (-a.f / (b.f - a.f));
            let margin = (b.x - a.x) * 0.1;
            if secant > a.x + margin && secant < b.x - margin && self.fresh(secant) {
                return Some(secant);
            }
            return self.fresh(midpoint).then_some(midpoint);
        }
        // Secant search handles roots without a sign change (for example x^2).
        // Interleave expanding probes so a flat slope/domain error cannot stall it.
        if self.points.len() >= 2 && self.tried.len() % 4 != 0 {
            sorted.sort_by(|a, b| a.f.abs().total_cmp(&b.f.abs()));
            let a = sorted[0];
            if let Some(b) = sorted.iter().skip(1).find(|b| b.f != a.f) {
                let x = a.x - a.f * ((b.x - a.x) / (b.f - a.f));
                if self.fresh(x) {
                    return Some(x);
                }
            }
        }
        for _ in 0..1100 {
            let magnitude =
                (self.start.abs().max(1.0) * 0.01) * 2.0_f64.powi((self.expansion / 2) as i32);
            let sign = if self.expansion % 2 == 0 { 1.0 } else { -1.0 };
            self.expansion += 1;
            let x = self.start + sign * magnitude;
            if self.fresh(x) {
                return Some(x);
            }
            if !magnitude.is_finite() {
                return None;
            }
        }
        None
    }
    pub fn advance(&mut self) -> Value {
        // One trial per request gives Flutter an opportunity to stop between
        // workbook calculations and lets saves/reads use the engine queue.
        if self.status != "running" {
            return self.response();
        }
        if let Some(candidate) = self.verifying.take() {
            self.tried.push(candidate.x);
            match self.evaluate(candidate.x) {
                Ok(y)
                    if (y - self.goal).abs() <= self.tolerance
                        && (y - candidate.y).abs() <= self.tolerance =>
                {
                    self.best = Some(Point { y, ..candidate });
                    self.status = "converged";
                    self.reason = "A matching input value was found.".into();
                }
                _ => {
                    self.status = "unstable";
                    self.reason = "The formula changed when the same input was evaluated again. No stable result is available.".into();
                }
            }
            return self.response();
        }
        if self.tried.len() >= self.limit - 1 {
            self.status = "limit";
            self.reason = "The trial limit was reached without matching the target.".into();
            return self.response();
        }
        let Some(x) = self.next() else {
            self.status = "stalled";
            self.reason =
                "The search cannot improve this result at the available precision.".into();
            return self.response();
        };
        self.tried.push(x);
        match self.evaluate(x) {
            Ok(y) => {
                let scale = self.goal.abs().max(1.0);
                let f = y / scale - self.goal / scale;
                if f.is_finite() {
                    let point = Point { x, y, f };
                    self.points.push(point);
                    if self.best.is_none_or(|b| point.f.abs() < b.f.abs()) {
                        self.best = Some(point);
                    }
                    if (y - self.goal).abs() <= self.tolerance {
                        self.verifying = Some(point);
                    }
                }
            }
            Err(error) => self.last_error = Some(error),
        }
        self.response()
    }
    pub fn stop(&mut self) -> Value {
        if self.status == "running" {
            self.status = "stopped";
            self.reason = "The search was stopped. Review the best trial below.".into();
        }
        self.response()
    }
    pub fn accept(&self, r: &Value) -> Result<Accepted, String> {
        if self.status == "running" || self.status == "unstable" {
            return Err("There is no completed result to apply".into());
        }
        let best = self.best.ok_or("No numerical result is available")?;
        if self.status != "converged" && r["acceptBest"] != true {
            return Err("The target was not reached. Explicitly choose Apply best value to keep this trial.".into());
        }
        let value = if best.x == self.start {
            self.original.clone()
        } else {
            best.x.to_string()
        };
        Ok(Accepted {
            edit: json!({"op":"enterCell","sheetId":self.changing.sheet_id,"address":self.changing.address,
            "value":value,"baseRevision":self.revision,"allowInvalid":r["allowInvalid"]==true}),
            unchanged: best.x == self.start,
            target: self.target.clone(),
            result: best.y,
            tolerance: self.tolerance,
        })
    }
    pub fn response(&self) -> Value {
        json!({"token":self.token,"status":self.status,"reason":self.reason,"trials":self.tried.len(),
            "maxTrials":self.limit,"goal":self.goal,"tolerance":self.tolerance,"originalInput":self.original,
            "startValue":self.start,"input":self.best.map(|p|p.x),"result":self.best.map(|p|p.y),
            "difference":self.best.map(|p|p.y-self.goal),"lastTrialError":self.last_error,
            "target":{"sheetId":self.target.sheet_id,"address":self.target.address},
            "changing":{"sheetId":self.changing.sheet_id,"address":self.changing.address},
            "canApply":self.best.is_some() && !matches!(self.status,"running"|"unstable")})
    }
}

pub fn check_scenario_input(
    w: &Value,
    calc: &WorkbookCalculator,
    pkg: Option<&Package>,
    sheet_id: &Value,
    address: &Value,
) -> Result<(), String> {
    let cell = Cell::read(w, &json!({"sheetId":sheet_id,"address":address}))?;
    check_editable(&w["sheets"][cell.sheet], &cell, pkg)?;
    if calc.is_spill_cell(cell.sheet, cell.row, cell.col)? {
        return Err("A spilled result cannot be a scenario input".into());
    }
    Ok(())
}

fn check_editable(sheet: &Value, cell: &Cell, pkg: Option<&Package>) -> Result<(), String> {
    for range in sheet["mergedRanges"].as_array().into_iter().flatten() {
        if Area::parse(range.as_str().ok_or("Invalid merge")?)?.contains(cell.row, cell.col) {
            return Err("Choose an unmerged changing cell".into());
        }
    }
    for table in crate::tables::list(sheet) {
        let area = crate::tables::area(table)?;
        if area.contains(cell.row, cell.col)
            && cell.row < area.top + table["headerRows"].as_u64().unwrap_or(1) as usize
        {
            return Err("A table heading cannot be the changing cell".into());
        }
    }
    if let Some((pkg, path)) = pkg.zip(sheet["officePath"].as_str()) {
        let dom = crate::office::parse(pkg.text(path)?)?;
        if crate::office::child(dom.root_element(), "sheetProtection").is_some() {
            return Err("Changing inputs on a protected worksheet needs additional support".into());
        }
        for node in dom
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "f")
        {
            if matches!(node.attribute("t"), Some("array" | "dataTable"))
                && node
                    .attribute("ref")
                    .is_some_and(|r| Area::parse(r).is_ok_and(|a| a.contains(cell.row, cell.col)))
            {
                return Err("Choose a changing cell outside array and data-table formulas".into());
            }
        }
    }
    Ok(())
}
