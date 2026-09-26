use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{NamedVariable, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    model::Model,
};

use super::r#let::assign_variable_ids;

/// FOLIO PATCH: an evaluated value as an argument node.
fn value_node(value: &CalcResult, cell: CellReferenceIndex) -> Node {
    match value {
        CalcResult::Number(n) => Node::NumberKind(*n),
        CalcResult::String(s) => Node::StringKind(s.clone()),
        CalcResult::Boolean(b) => Node::BooleanKind(*b),
        CalcResult::Array(a) => Node::ArrayKind(a.clone()),
        CalcResult::Error { error, .. } => Node::ErrorKind(error.clone()),
        CalcResult::EmptyCell | CalcResult::EmptyArg => Node::EmptyArgKind,
        CalcResult::Range { left, right } => Node::RangeKind {
            sheet_name: None,
            sheet_index: left.sheet,
            absolute_row1: true,
            absolute_column1: true,
            row1: left.row,
            column1: left.column,
            absolute_row2: true,
            absolute_column2: true,
            row2: right.row,
            column2: right.column,
        },
        CalcResult::Lambda(_) => {
            let _ = cell;
            Node::ErrorKind(Error::VALUE)
        }
    }
}

impl<'a> Model<'a> {
    /// Evaluates the body of a named lambda with the given call-site argument nodes.
    /// Optional parameters not covered by call_args receive EmptyArg.
    pub(crate) fn call_lambda(
        &mut self,
        lambda_result: CalcResult,
        call_args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        // FOLIO PATCH: a function named as a value.
        if let CalcResult::Lambda(id) = &lambda_result {
            if let Some(kind) = self.eta_functions.get(id).cloned() {
                return self.evaluate_function(&kind, call_args, cell);
            }
        }
        let (parameters, body) = match lambda_result {
            CalcResult::Lambda(id) => match self.lambdas.get(&id) {
                Some(l) => l.clone(),
                None => {
                    return CalcResult::new_error(
                        Error::NAME,
                        cell,
                        "Lambda not found in store".to_string(),
                    )
                }
            },
            other if other.is_error() => return other,
            _ => return CalcResult::new_error(Error::VALUE, cell, "Expected a LAMBDA".to_string()),
        };

        if let Err(e) = check_arg_count(&parameters, call_args.len(), cell) {
            return e;
        }

        let mut patched_body = body;
        let mut bound_ids: Vec<usize> = Vec::with_capacity(parameters.len());

        for param in &parameters {
            let raw_id = self.get_next_variable_id();
            assign_variable_ids(&mut patched_body, &param.name, raw_id as u32);
            bound_ids.push(raw_id);
        }

        for (i, raw_id) in bound_ids.iter().enumerate() {
            let val = if i < call_args.len() {
                self.evaluate_node_in_context(&call_args[i], cell)
            } else {
                CalcResult::EmptyArg
            };
            self.variable_stack.insert(*raw_id, val);
        }

        let result = self.evaluate_node_in_context(&patched_body, cell);

        for raw_id in bound_ids {
            self.variable_stack.remove(&raw_id);
        }

        result
    }

    /// Like `call_lambda` but accepts already-evaluated `CalcResult` values instead of `Node`s.
    /// Optional parameters not covered by `values` receive EmptyArg.
    pub(crate) fn call_lambda_with_values(
        &mut self,
        lambda_result: CalcResult,
        values: Vec<CalcResult>,
        cell: CellReferenceIndex,
    ) -> CalcResult {
        // FOLIO PATCH: a function named as a value.
        if let CalcResult::Lambda(id) = &lambda_result {
            if let Some(kind) = self.eta_functions.get(id).cloned() {
                let args: Vec<Node> = values.iter().map(|v| value_node(v, cell)).collect();
                return self.evaluate_function(&kind, &args, cell);
            }
        }
        let (parameters, body) = match lambda_result {
            CalcResult::Lambda(id) => match self.lambdas.get(&id) {
                Some(l) => l.clone(),
                None => {
                    return CalcResult::new_error(
                        Error::NAME,
                        cell,
                        "Lambda not found in store".to_string(),
                    )
                }
            },
            other if other.is_error() => return other,
            _ => return CalcResult::new_error(Error::VALUE, cell, "Expected a LAMBDA".to_string()),
        };

        if let Err(e) = check_arg_count(&parameters, values.len(), cell) {
            return e;
        }

        let mut patched_body = body;
        let mut bound_ids: Vec<usize> = Vec::with_capacity(parameters.len());

        for param in &parameters {
            let raw_id = self.get_next_variable_id();
            assign_variable_ids(&mut patched_body, &param.name, raw_id as u32);
            bound_ids.push(raw_id);
        }

        for (i, raw_id) in bound_ids.iter().enumerate() {
            let val = if i < values.len() {
                values[i].clone()
            } else {
                CalcResult::EmptyArg
            };
            self.variable_stack.insert(*raw_id, val);
        }

        let result = self.evaluate_node_in_context(&patched_body, cell);

        for raw_id in bound_ids {
            self.variable_stack.remove(&raw_id);
        }

        result
    }

    /// FOLIO PATCH: an argument that takes a LAMBDA. A function's name there
    /// is a lambda that calls it, as in Excel (`BYROW(A1:C3, SUM)`).
    pub(crate) fn evaluate_lambda_argument(&mut self, node: &Node, cell: CellReferenceIndex) -> CalcResult {
        if let Node::NamedVariableKind { name, id: None } = node {
            if let Some(kind) = self.language.functions.lookup(name) {
                let id = self.get_next_lambda_id();
                self.lambdas.insert(id, (vec![], Node::EmptyArgKind));
                self.eta_functions.insert(id, kind);
                return CalcResult::Lambda(id);
            }
        }
        self.evaluate_node_in_context(node, cell)
    }

    pub(crate) fn fn_lambda(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        // Reached when LAMBDA is stored as FunctionKind::Lambda (e.g. imported from xlsx).
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let last = args.len() - 1;
        let body = args[last].clone();
        let parameters: Vec<NamedVariable> = args[..last]
            .iter()
            .filter_map(|n| {
                if let Node::NamedVariableKind { name, id } = n {
                    let (clean, is_optional) = if let Some(s) = name.strip_prefix("_xlop.") {
                        (s.to_string(), true)
                    } else if let Some(s) = name.strip_prefix("_xlpm.") {
                        (s.to_string(), false)
                    } else {
                        (name.clone(), false)
                    };
                    Some(NamedVariable {
                        name: clean,
                        id: *id,
                        is_optional,
                    })
                } else {
                    None
                }
            })
            .collect();

        let id = self.get_next_lambda_id();
        self.lambdas.insert(id, (parameters, body));
        CalcResult::Lambda(id)
    }
}

fn check_arg_count(
    parameters: &[NamedVariable],
    provided: usize,
    cell: CellReferenceIndex,
) -> Result<(), CalcResult> {
    let required = parameters.iter().filter(|p| !p.is_optional).count();
    let total = parameters.len();
    if provided < required || provided > total {
        let msg = if required == total {
            format!("LAMBDA expected {} argument(s), got {}", total, provided)
        } else {
            format!(
                "LAMBDA expected between {} and {} argument(s), got {}",
                required, total, provided
            )
        };
        Err(CalcResult::new_error(Error::VALUE, cell, msg))
    } else {
        Ok(())
    }
}
