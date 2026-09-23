//! Direct, syntactic formula references from the same AST used for calculation.
//! Keep ranges sparse: A:A is one edge, never a million cell allocations.
use crate::formula::{address, parse_address, COLS, ROWS};
use ironcalc_base::{expressions::parser::Node, Model};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Area {
    sheet: usize,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
}
impl Area {
    fn cell(sheet: usize, row: usize, col: usize) -> Self {
        Self {
            sheet,
            r1: row,
            c1: col,
            r2: row,
            c2: col,
        }
    }
    fn contains(&self, other: Self) -> bool {
        self.sheet == other.sheet
            && self.r1 <= other.r1
            && self.r2 >= other.r2
            && self.c1 <= other.c1
            && self.c2 >= other.c2
    }
    fn range(&self) -> String {
        let first = address(self.r1, self.c1);
        if self.r1 == self.r2 && self.c1 == self.c2 {
            first
        } else {
            format!("{first}:{}", address(self.r2, self.c2))
        }
    }
    fn describe(&self, identity: &[(String, String)]) -> Value {
        let (id, name) = &identity[self.sheet];
        json!({"sheetId":id,"sheetName":name,"address":address(self.r1,self.c1),
            "range":self.range(),"isRange":self.r1 != self.r2 || self.c1 != self.c2})
    }
}

struct FormulaLinks {
    cell: Area,
    references: BTreeSet<Area>,
    warnings: BTreeSet<String>,
}

#[derive(Default)]
pub struct AuditIndex {
    formulas: Vec<FormulaLinks>,
}

impl AuditIndex {
    pub fn build(model: &Model<'_>, inputs: &[HashMap<String, String>]) -> Result<Self, String> {
        let mut formulas = Vec::new();
        for (sheet, cells) in inputs.iter().enumerate() {
            let worksheet = model.workbook.worksheet(sheet as u32)?;
            for at in cells.keys() {
                let (row, col) = parse_address(at)?;
                let Some(index) = worksheet
                    .cell(row as i32 + 1, col as i32 + 1)
                    .and_then(|c| c.get_formula())
                else {
                    continue;
                };
                let (node, _) = model
                    .parsed_formulas
                    .get(sheet)
                    .and_then(|f| f.get(index as usize))
                    .ok_or("Formula unavailable for tracing")?;
                let mut links = FormulaLinks {
                    cell: Area::cell(sheet, row, col),
                    references: BTreeSet::new(),
                    warnings: BTreeSet::new(),
                };
                links.collect(node, inputs.len());
                formulas.push(links);
            }
        }
        formulas.sort_by_key(|f| f.cell);
        Ok(Self { formulas })
    }

    pub fn query(
        &self,
        identity: &[(String, String)],
        sheet_id: &str,
        at: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Value, String> {
        let sheet = identity
            .iter()
            .position(|(id, _)| id == sheet_id)
            .ok_or("Worksheet missing")?;
        let (row, col) = parse_address(at)?;
        let cell = Area::cell(sheet, row, col);
        let selected = self.formulas.iter().find(|f| f.cell == cell);
        let precedents = selected.map(|f| &f.references);
        let dependents: Vec<_> = self
            .formulas
            .iter()
            .filter(|f| f.references.iter().any(|a| a.contains(cell)))
            .collect();
        let uncertain: Vec<_> = self
            .formulas
            .iter()
            .filter(|f| !f.warnings.is_empty())
            .collect();
        Ok(json!({
            "cell":cell.describe(identity), "isFormula":selected.is_some(),
            "precedents":precedents.into_iter().flatten().skip(offset).take(limit).map(|a| a.describe(identity)).collect::<Vec<_>>(),
            "precedentCount":precedents.map_or(0, |p| p.len()),
            "dependents":dependents.iter().skip(offset).take(limit).map(|f| f.cell.describe(identity)).collect::<Vec<_>>(),
            "dependentCount":dependents.len(),
            "warnings":selected.map(|f| f.warnings.iter().collect::<Vec<_>>()).unwrap_or_default(),
            "unresolvedFormulaCount":uncertain.len(),
            "unresolvedFormulas":uncertain.iter().skip(offset).take(limit).map(|f| {
                let mut d = f.cell.describe(identity); d["warnings"] = json!(f.warnings); d
            }).collect::<Vec<_>>(),
            "offset":offset,"limit":limit,
            "scope":"Direct references written in formulas, including every conditional branch. Calculated reference targets may be incomplete."
        }))
    }
}

impl FormulaLinks {
    fn collect(&mut self, root: &Node, sheet_count: usize) {
        let mut pending = vec![root];
        // Iterative traversal also covers formulas with many nested operators.
        while let Some(node) = pending.pop() {
            match node {
                Node::ReferenceKind {
                    sheet_index,
                    absolute_row,
                    absolute_column,
                    row,
                    column,
                    ..
                } => {
                    self.reference(
                        *sheet_index,
                        *row,
                        *column,
                        *row,
                        *column,
                        *absolute_row,
                        *absolute_column,
                        *absolute_row,
                        *absolute_column,
                        sheet_count,
                    );
                }
                Node::RangeKind {
                    sheet_index,
                    absolute_row1,
                    absolute_column1,
                    row1,
                    column1,
                    absolute_row2,
                    absolute_column2,
                    row2,
                    column2,
                    ..
                } => {
                    self.reference(
                        *sheet_index,
                        *row1,
                        *column1,
                        *row2,
                        *column2,
                        *absolute_row1,
                        *absolute_column1,
                        *absolute_row2,
                        *absolute_column2,
                        sheet_count,
                    );
                }
                Node::OpRangeKind { left, right } => {
                    self.warnings
                        .insert("A calculated range endpoint cannot yet be fully traced.".into());
                    pending.extend([left.as_ref(), right.as_ref()]);
                }
                Node::OpConcatenateKind { left, right }
                | Node::OpSumKind { left, right, .. }
                | Node::OpProductKind { left, right, .. }
                | Node::OpPowerKind { left, right }
                | Node::CompareKind { left, right, .. } => {
                    pending.extend([left.as_ref(), right.as_ref()])
                }
                Node::FunctionKind { kind, args } => {
                    let name = kind.to_xlsx_string();
                    if matches!(name.as_str(), "INDIRECT" | "OFFSET") {
                        self.warnings.insert(format!(
                            "{name} calculates a reference; its target cells are not yet traced."
                        ));
                    }
                    pending.extend(args);
                }
                Node::LambdaDefKind { body, .. } => pending.push(body),
                Node::LambdaCallKind { lambda, args } => {
                    pending.push(lambda);
                    pending.extend(args);
                }
                Node::NamedFunctionKind { name, args, .. } => {
                    self.warnings.insert(format!(
                        "The named function {name} cannot yet be fully traced."
                    ));
                    pending.extend(args);
                }
                Node::DefinedNameKind((name, ..)) => {
                    self.warnings
                        .insert(format!("The defined name {name} cannot yet be traced."));
                }
                Node::TableNameKind(name) => {
                    self.warnings.insert(format!("The bare table name {name} cannot yet be traced. Structured column references are supported."));
                }
                Node::SpillRangeOperator { child } => {
                    self.warnings
                        .insert("Only the anchor of a spilled range is traced.".into());
                    pending.push(child);
                }
                Node::ImplicitIntersection { child, .. } => pending.push(child),
                Node::UnaryKind { right, .. } => pending.push(right),
                Node::WrongReferenceKind { .. } | Node::WrongRangeKind { .. } => {
                    self.warnings.insert(
                        "A missing worksheet or invalid reference cannot be traced.".into(),
                    );
                }
                Node::ParseErrorKind { .. } => {
                    self.warnings.insert("The formula could not be parsed completely; some references may be missing.".into());
                }
                Node::ErrorKind(error) => {
                    if matches!(
                        error,
                        ironcalc_base::expressions::token::Error::REF
                            | ironcalc_base::expressions::token::Error::NAME
                    ) {
                        self.warnings.insert(
                            "An invalid reference or unresolved name cannot be traced.".into(),
                        );
                    }
                }
                Node::BooleanKind(_)
                | Node::NumberKind(_)
                | Node::StringKind(_)
                | Node::ArrayKind(_)
                | Node::NamedVariableKind { id: Some(_), .. }
                | Node::EmptyArgKind => {}
                Node::NamedVariableKind { name, id: None } => {
                    self.warnings
                        .insert(format!("The unresolved name {name} cannot be traced."));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reference(
        &mut self,
        sheet: u32,
        r1: i32,
        c1: i32,
        r2: i32,
        c2: i32,
        ar1: bool,
        ac1: bool,
        ar2: bool,
        ac2: bool,
        sheet_count: usize,
    ) {
        let resolve = |v: i32, absolute: bool, base: usize| {
            if absolute {
                i64::from(v) - 1
            } else {
                i64::from(v) + base as i64
            }
        };
        let r1 = resolve(r1, ar1, self.cell.r1);
        let c1 = resolve(c1, ac1, self.cell.c1);
        let r2 = resolve(r2, ar2, self.cell.r1);
        let c2 = resolve(c2, ac2, self.cell.c1);
        if sheet as usize >= sheet_count
            || [r1, r2].iter().any(|r| *r < 0 || *r >= ROWS as i64)
            || [c1, c2].iter().any(|c| *c < 0 || *c >= COLS as i64)
        {
            self.warnings
                .insert("An out-of-bounds reference cannot be traced.".into());
            return;
        }
        self.references.insert(Area {
            sheet: sheet as usize,
            r1: r1.min(r2) as usize,
            c1: c1.min(c2) as usize,
            r2: r1.max(r2) as usize,
            c2: c1.max(c2) as usize,
        });
    }
}
