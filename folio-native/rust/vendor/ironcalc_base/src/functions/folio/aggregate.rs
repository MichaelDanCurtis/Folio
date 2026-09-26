//! AGGREGATE(function_num, options, ref1, [ref2], …) and
//! AGGREGATE(function_num, options, array, k).
use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    functions::{subtotal::CellTableStatus, Function},
    model::Model,
};

/// The values AGGREGATE reads, after its options are applied.
#[derive(Default)]
struct Collected {
    numbers: Vec<f64>,
    /// Text, logical values and (when not ignored) errors: COUNTA counts them.
    others: usize,
    /// The first error, when errors are not ignored.
    error: Option<CalcResult>,
}

struct Options {
    hidden: bool,
    errors: bool,
    nested: bool,
}

fn is_total(node: &Node) -> bool {
    matches!(
        node,
        Node::FunctionKind {
            kind: Function::Subtotal | Function::Aggregate,
            ..
        }
    )
}

impl Model<'_> {
    /// A cell whose formula is a SUBTOTAL or AGGREGATE.
    fn cell_is_total(&self, sheet: u32, row: i32, column: i32) -> bool {
        let Some(cell) = self.workbook.worksheets[sheet as usize]
            .sheet_data
            .get(&row)
            .and_then(|r| r.get(&column))
        else {
            return false;
        };
        cell.get_formula()
            .and_then(|f| self.parsed_formulas[sheet as usize].get(f as usize))
            .is_some_and(|(node, _)| match node {
                Node::ImplicitIntersection { child, .. } => is_total(child),
                node => is_total(node),
            })
    }

    fn aggregate_value(&mut self, value: CalcResult, options: &Options, into: &mut Collected) {
        match value {
            CalcResult::Number(n) => into.numbers.push(n),
            CalcResult::String(_) | CalcResult::Boolean(_) => into.others += 1,
            error @ CalcResult::Error { .. } => {
                if !options.errors {
                    into.others += 1;
                    into.error.get_or_insert(error);
                }
            }
            _ => {}
        }
    }

    fn aggregate_collect(
        &mut self,
        args: &[Node],
        options: &Options,
        cell: CellReferenceIndex,
    ) -> Result<Collected, CalcResult> {
        let mut collected = Collected::default();
        for arg in args {
            if options.nested && is_total(arg) {
                continue;
            }
            match self.evaluate_node_with_reference(arg, cell) {
                CalcResult::Range { left, right } => {
                    if left.sheet != right.sheet {
                        return Err(CalcResult::new_error(
                            Error::VALUE,
                            cell,
                            "Ranges are in different sheets".to_string(),
                        ));
                    }
                    let (left, right) = self.clamp_to_data(left, right);
                    for row in left.row..=right.row {
                        if options.hidden {
                            let status = self
                                .cell_hidden_status(left.sheet, row, left.column)
                                .map_err(|m| CalcResult::new_error(Error::ERROR, cell, m))?;
                            if status != CellTableStatus::Normal {
                                continue;
                            }
                        }
                        for column in left.column..=right.column {
                            if options.nested && self.cell_is_total(left.sheet, row, column) {
                                continue;
                            }
                            let value = self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column,
                            });
                            self.aggregate_value(value, options, &mut collected);
                        }
                    }
                }
                CalcResult::Array(rows) => {
                    for value in rows.into_iter().flatten() {
                        let value = match value {
                            ArrayNode::Number(n) => CalcResult::Number(n),
                            ArrayNode::Boolean(b) => CalcResult::Boolean(b),
                            ArrayNode::String(s) => CalcResult::String(s),
                            ArrayNode::Error(e) => CalcResult::new_error(e, cell, String::new()),
                            ArrayNode::Empty => continue,
                        };
                        self.aggregate_value(value, options, &mut collected);
                    }
                }
                CalcResult::Lambda(_) => {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Expected a value".to_string(),
                    ))
                }
                value => self.aggregate_value(value, options, &mut collected),
            }
        }
        Ok(collected)
    }

    pub(crate) fn fn_aggregate(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let function = match self.get_number(&args[0], cell) {
            Ok(f) => f.trunc() as i32,
            Err(e) => return e,
        };
        let option = if matches!(args[1], Node::EmptyArgKind) {
            0
        } else {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i32,
                Err(e) => return e,
            }
        };
        if !(1..=19).contains(&function) || !(0..=7).contains(&option) {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "Invalid AGGREGATE function or option".to_string(),
            );
        }
        let options = Options {
            hidden: matches!(option, 1 | 3 | 5 | 7),
            errors: matches!(option, 2 | 3 | 6 | 7),
            nested: option <= 3,
        };
        let (inputs, k) = if function >= 14 {
            if args.len() != 4 {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "This AGGREGATE function needs an array and k".to_string(),
                );
            }
            match self.get_number(&args[3], cell) {
                Ok(k) => (&args[2..3], k),
                Err(e) => return e,
            }
        } else {
            (&args[2..], 0.0)
        };
        let collected = match self.aggregate_collect(inputs, &options, cell) {
            Ok(c) => c,
            Err(e) => return e,
        };
        // COUNT and COUNTA don't return errors.
        match function {
            2 => return CalcResult::Number(collected.numbers.len() as f64),
            3 => return CalcResult::Number((collected.numbers.len() + collected.others) as f64),
            _ => {}
        }
        if let Some(error) = collected.error {
            return error;
        }
        let mut v = collected.numbers;
        let n = v.len() as f64;
        let error = |e: Error| CalcResult::new_error(e, cell, "AGGREGATE".to_string());
        let mean = v.iter().sum::<f64>() / n;
        let squares = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
        let sorted = |v: &mut Vec<f64>| v.sort_by(|a, b| a.total_cmp(b));
        let result = match function {
            1 if v.is_empty() => return error(Error::DIV),
            1 => mean,
            4 => v
                .iter()
                .copied()
                .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))))
                .unwrap_or(0.0),
            5 => v
                .iter()
                .copied()
                .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.min(x))))
                .unwrap_or(0.0),
            6 if v.is_empty() => 0.0,
            6 => v.iter().product(),
            7 if v.len() < 2 => return error(Error::DIV),
            7 => (squares / (n - 1.0)).sqrt(),
            8 if v.is_empty() => return error(Error::DIV),
            8 => (squares / n).sqrt(),
            9 => v.iter().sum(),
            10 if v.len() < 2 => return error(Error::DIV),
            10 => squares / (n - 1.0),
            11 if v.is_empty() => return error(Error::DIV),
            11 => squares / n,
            12 if v.is_empty() => return error(Error::NUM),
            12 => {
                sorted(&mut v);
                let m = v.len() / 2;
                if v.len() % 2 == 1 {
                    v[m]
                } else {
                    (v[m - 1] + v[m]) / 2.0
                }
            }
            13 => {
                // The most frequent value; the first one in the data on a tie.
                let mut best: Option<(f64, usize)> = None;
                for x in &v {
                    let count = v.iter().filter(|y| *y == x).count();
                    if count > 1 && best.is_none_or(|(_, c)| count > c) {
                        best = Some((*x, count));
                    }
                }
                match best {
                    Some((x, _)) => x,
                    None => return error(Error::NA),
                }
            }
            14 | 15 => {
                let k = k.ceil();
                if k < 1.0 || k > n {
                    return error(Error::NUM);
                }
                sorted(&mut v);
                if function == 14 {
                    v[v.len() - k as usize]
                } else {
                    v[k as usize - 1]
                }
            }
            16..=19 => {
                if v.is_empty() {
                    return error(Error::NUM);
                }
                sorted(&mut v);
                let (p, exclusive) = match function {
                    16 => (k, false),
                    17 => (k.trunc() / 4.0, false),
                    18 => (k, true),
                    _ => (k.trunc() / 4.0, true),
                };
                let valid = match function {
                    17 => (0.0..=4.0).contains(&k.trunc()),
                    19 => (1.0..=3.0).contains(&k.trunc()),
                    _ => (0.0..=1.0).contains(&p),
                };
                if !valid {
                    return error(Error::NUM);
                }
                match percentile(&v, p, exclusive) {
                    Some(x) => x,
                    None => return error(Error::NUM),
                }
            }
            _ => return error(Error::VALUE),
        };
        CalcResult::Number(result)
    }
}

/// PERCENTILE.INC / PERCENTILE.EXC of sorted values.
fn percentile(v: &[f64], p: f64, exclusive: bool) -> Option<f64> {
    let n = v.len() as f64;
    let rank = if exclusive {
        let r = p * (n + 1.0);
        if r < 1.0 || r > n {
            return None;
        }
        r - 1.0
    } else {
        p * (n - 1.0)
    };
    let low = rank.floor() as usize;
    let high = (low + 1).min(v.len() - 1);
    Some(v[low] + (rank - low as f64) * (v[high] - v[low]))
}
