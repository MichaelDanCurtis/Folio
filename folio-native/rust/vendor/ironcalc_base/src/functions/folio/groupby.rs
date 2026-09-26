//! GROUPBY(row_fields, values, function, [field_headers], [total_depth],
//! [sort_order], [filter_array], [field_relationship]) and
//! PIVOTBY(row_fields, col_fields, values, function, [field_headers],
//! [row_total_depth], [row_sort_order], [col_total_depth], [col_sort_order],
//! [filter_array]).
//!
//! Not yet: generated headers (field_headers 2 shows no header row),
//! PIVOTBY's relative_to, several value columns in PIVOTBY, and column
//! subtotals.
use std::cmp::Ordering;

use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    model::Model,
};

type Grid = Vec<Vec<ArrayNode>>;

/// Excel's sort order: numbers, text (ignoring case), logical values,
/// errors, then blanks.
fn rank(n: &ArrayNode) -> u8 {
    match n {
        ArrayNode::Number(_) => 0,
        ArrayNode::String(_) => 1,
        ArrayNode::Boolean(_) => 2,
        ArrayNode::Error(_) => 3,
        ArrayNode::Empty => 4,
    }
}

fn compare(a: &ArrayNode, b: &ArrayNode) -> Ordering {
    match (a, b) {
        (ArrayNode::Number(x), ArrayNode::Number(y)) => x.total_cmp(y),
        (ArrayNode::String(x), ArrayNode::String(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
        (ArrayNode::Boolean(x), ArrayNode::Boolean(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    }
}

fn truthy(n: &ArrayNode) -> bool {
    match n {
        ArrayNode::Number(x) => *x != 0.0,
        ArrayNode::Boolean(b) => *b,
        ArrayNode::String(s) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

fn text(s: &str) -> ArrayNode {
    ArrayNode::String(s.to_string())
}

/// A blank cell in the result. A spilled empty value would show as 0.
fn blank() -> ArrayNode {
    text("")
}

struct Group {
    key: ArrayNode,
    rows: Vec<usize>,
    children: Vec<Group>,
}

/// Groups `rows` by the key at `level`, then by the later keys inside.
fn build(rows: &[usize], keys: &Grid, level: usize) -> Vec<Group> {
    let mut groups: Vec<Group> = vec![];
    for &r in rows {
        let key = &keys[r][level];
        match groups
            .iter_mut()
            .find(|g| compare(&g.key, key) == Ordering::Equal)
        {
            Some(g) => g.rows.push(r),
            None => groups.push(Group {
                key: key.clone(),
                rows: vec![r],
                children: vec![],
            }),
        }
    }
    if level + 1 < keys.first().map_or(0, Vec::len) {
        for g in &mut groups {
            g.children = build(&g.rows, keys, level + 1);
        }
    }
    groups
}

/// How the result is laid out.
struct Layout {
    headers: bool,
    show_headers: bool,
    totals: bool,
    subtotals: bool,
    top: bool,
}

fn layout(field_headers: Option<i32>, depth: i32, keys: usize, detected: bool) -> Layout {
    let headers = match field_headers {
        Some(1 | 3) => true,
        Some(_) => false,
        None => detected,
    };
    Layout {
        headers,
        show_headers: field_headers == Some(3),
        totals: depth != 0,
        subtotals: depth.abs() >= 2 && keys > 1,
        top: depth < 0,
    }
}

/// A first row of text over a second row of numbers reads as headers.
fn detect_headers(values: &Grid) -> bool {
    values.len() > 1
        && values[0].iter().all(|v| matches!(v, ArrayNode::String(_)))
        && values[1].iter().any(|v| matches!(v, ArrayNode::Number(_)))
}

impl Model<'_> {
    fn optional_number(
        &mut self,
        args: &[Node],
        index: usize,
        cell: CellReferenceIndex,
    ) -> Result<Option<i32>, CalcResult> {
        match args.get(index) {
            None | Some(Node::EmptyArgKind) => Ok(None),
            Some(a) => self.get_number(a, cell).map(|n| Some(n.trunc() as i32)),
        }
    }

    /// Sort keys: 1-based column numbers, negative for descending.
    fn sort_specs(
        &mut self,
        args: &[Node],
        index: usize,
        cell: CellReferenceIndex,
    ) -> Result<Vec<i32>, CalcResult> {
        match args.get(index) {
            None | Some(Node::EmptyArgKind) => Ok(vec![]),
            Some(a) => {
                let grid = self.eval_to_array(a, cell)?;
                let mut specs = vec![];
                for v in grid.into_iter().flatten() {
                    match v {
                        ArrayNode::Number(n) if n.trunc() != 0.0 => specs.push(n.trunc() as i32),
                        ArrayNode::Empty => {}
                        _ => {
                            return Err(CalcResult::new_error(
                                Error::VALUE,
                                cell,
                                "Invalid sort order".to_string(),
                            ))
                        }
                    }
                }
                Ok(specs)
            }
        }
    }

    /// The function applied to one group's values, as a column.
    fn group_value(
        &mut self,
        function: &CalcResult,
        column: &Grid,
        rows: &[usize],
        index: usize,
        cell: CellReferenceIndex,
    ) -> ArrayNode {
        let values: Grid = rows
            .iter()
            .map(|r| vec![column[*r][index].clone()])
            .collect();
        match self.call_lambda_with_values(function.clone(), vec![CalcResult::Array(values)], cell)
        {
            CalcResult::Number(n) => ArrayNode::Number(n),
            CalcResult::String(s) => ArrayNode::String(s),
            CalcResult::Boolean(b) => ArrayNode::Boolean(b),
            CalcResult::Error { error, .. } => ArrayNode::Error(error),
            CalcResult::EmptyCell | CalcResult::EmptyArg => ArrayNode::Number(0.0),
            CalcResult::Array(a) if a.len() == 1 && a[0].len() == 1 => a[0][0].clone(),
            _ => ArrayNode::Error(Error::CALC),
        }
    }

    /// Orders groups at every level: by the sort keys that name this level's
    /// field (`1..=fields`) or a value column (`fields + 1..`), then by key.
    #[allow(clippy::too_many_arguments)]
    fn sort_groups(
        &mut self,
        groups: &mut Vec<Group>,
        level: usize,
        fields: usize,
        specs: &[i32],
        function: &CalcResult,
        values: &Grid,
        cell: CellReferenceIndex,
    ) {
        let mut scored: Vec<(Vec<(ArrayNode, bool)>, Group)> = vec![];
        for g in groups.drain(..) {
            let mut score = vec![];
            for s in specs {
                let (column, descending) = (s.unsigned_abs() as usize, *s < 0);
                if column == level + 1 {
                    score.push((g.key.clone(), descending));
                } else if column > fields && column - fields <= values.first().map_or(0, Vec::len) {
                    let v = self.group_value(function, values, &g.rows, column - fields - 1, cell);
                    score.push((v, descending));
                }
            }
            if !specs.iter().any(|s| s.unsigned_abs() as usize == level + 1) {
                score.push((g.key.clone(), false));
            }
            scored.push((score, g));
        }
        scored.sort_by(|(a, _), (b, _)| {
            for ((x, descending), (y, _)) in a.iter().zip(b) {
                let o = compare(x, y);
                if o != Ordering::Equal {
                    return if *descending { o.reverse() } else { o };
                }
            }
            Ordering::Equal
        });
        for (_, mut g) in scored {
            self.sort_groups(
                &mut g.children,
                level + 1,
                fields,
                specs,
                function,
                values,
                cell,
            );
            groups.push(g);
        }
    }

    /// The arguments both functions share: fields, values and the filter.
    #[allow(clippy::type_complexity)]
    fn grouping_inputs(
        &mut self,
        fields: &[&Node],
        values: &Node,
        filter: Option<&Node>,
        field_headers: Option<i32>,
        cell: CellReferenceIndex,
    ) -> Result<(Vec<Grid>, Grid, Vec<usize>, bool), CalcResult> {
        let values = self.eval_to_array(values, cell)?;
        let mut grids = vec![];
        for f in fields {
            let grid = self.eval_to_array(f, cell)?;
            if grid.len() != values.len() {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "Fields and values must have the same number of rows".to_string(),
                ));
            }
            grids.push(grid);
        }
        let headers = match field_headers {
            Some(1 | 3) => true,
            Some(_) => false,
            None => detect_headers(&values),
        };
        let first = usize::from(headers);
        let keep = match filter.filter(|f| !matches!(f, Node::EmptyArgKind)) {
            Some(f) => {
                let filter = self.eval_to_array(f, cell)?;
                if filter.len() != values.len() {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "The filter must have one value per row".to_string(),
                    ));
                }
                (first..values.len())
                    .filter(|r| filter[*r].first().is_some_and(truthy))
                    .collect()
            }
            None => (first..values.len()).collect(),
        };
        Ok((grids, values, keep, headers))
    }

    fn function_argument(
        &mut self,
        node: &Node,
        cell: CellReferenceIndex,
    ) -> Result<CalcResult, CalcResult> {
        match self.evaluate_node_in_context(node, cell) {
            f @ CalcResult::Lambda(_) => Ok(f),
            e @ CalcResult::Error { .. } => Err(e),
            _ => Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "Expected a function or LAMBDA".to_string(),
            )),
        }
    }

    pub(crate) fn fn_groupby(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(3..=8).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |model: &mut Self| -> Result<Grid, CalcResult> {
            let field_headers = model.optional_number(args, 3, cell)?;
            let depth = model.optional_number(args, 4, cell)?;
            let specs = model.sort_specs(args, 5, cell)?;
            let table = model.optional_number(args, 7, cell)? == Some(1);
            let function = model.function_argument(&args[2], cell)?;
            let (fields, values, rows, detected) =
                model.grouping_inputs(&[&args[0]], &args[1], args.get(6), field_headers, cell)?;
            let keys = fields.into_iter().next().unwrap_or_default();
            let k = keys.first().map_or(0, Vec::len);
            let m = values.first().map_or(0, Vec::len);
            if k == 0 || m == 0 {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "Empty input".to_string(),
                ));
            }
            let mut l = layout(
                field_headers,
                depth.unwrap_or(if table { 1 } else { 2 }),
                k,
                detected,
            );
            if table {
                l.subtotals = false;
            }
            let mut groups = build(&rows, &keys, 0);
            model.sort_groups(&mut groups, 0, k, &specs, &function, &values, cell);

            let mut out: Grid = vec![];
            if l.headers && l.show_headers {
                let mut header: Vec<ArrayNode> = keys[0].clone();
                header.extend(values[0].iter().cloned());
                out.push(header);
            }
            let total_row = |model: &mut Self, rows: &[usize], label: Vec<ArrayNode>| {
                let mut row = label;
                for i in 0..m {
                    row.push(model.group_value(&function, &values, rows, i, cell));
                }
                row
            };
            let mut grand = vec![text("Total")];
            grand.resize(k, blank());
            if l.totals && l.top {
                out.push(total_row(model, &rows, grand.clone()));
            }
            // Depth first: each group's rows, with its subtotal before or after.
            let mut stack: Vec<(&Group, Vec<ArrayNode>, bool)> =
                groups.iter().rev().map(|g| (g, vec![], false)).collect();
            while let Some((g, prefix, closing)) = stack.pop() {
                let mut p = prefix.clone();
                p.push(g.key.clone());
                let mut label = p.clone();
                label.resize(k, blank());
                if g.children.is_empty() {
                    out.push(total_row(model, &g.rows, p));
                } else if closing {
                    out.push(total_row(model, &g.rows, label));
                } else {
                    if l.subtotals && l.top {
                        out.push(total_row(model, &g.rows, label));
                    }
                    if l.subtotals && !l.top {
                        stack.push((g, prefix, true));
                    }
                    for child in g.children.iter().rev() {
                        stack.push((child, p.clone(), false));
                    }
                }
            }
            if l.totals && !l.top {
                out.push(total_row(model, &rows, grand));
            }
            Ok(out)
        };
        match run(self) {
            Ok(grid) => super::array_result(grid, cell),
            Err(e) => e,
        }
    }

    pub(crate) fn fn_pivotby(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(4..=11).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let run = |model: &mut Self| -> Result<Grid, CalcResult> {
            let field_headers = model.optional_number(args, 4, cell)?;
            let row_depth = model.optional_number(args, 5, cell)?;
            let row_specs = model.sort_specs(args, 6, cell)?;
            let col_depth = model.optional_number(args, 7, cell)?.unwrap_or(1);
            let col_specs = model.sort_specs(args, 8, cell)?;
            if args
                .get(10)
                .is_some_and(|a| !matches!(a, Node::EmptyArgKind))
            {
                return Err(CalcResult::new_error(
                    Error::NIMPL,
                    cell,
                    "PIVOTBY's relative_to is not supported yet".to_string(),
                ));
            }
            let function = model.function_argument(&args[3], cell)?;
            let (fields, values, rows, detected) = model.grouping_inputs(
                &[&args[0], &args[1]],
                &args[2],
                args.get(9),
                field_headers,
                cell,
            )?;
            let row_keys = &fields[0];
            let col_keys = &fields[1];
            let k = row_keys.first().map_or(0, Vec::len);
            let c = col_keys.first().map_or(0, Vec::len);
            if k == 0 || c == 0 || values.first().map_or(0, Vec::len) != 1 {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "PIVOTBY takes one column of values".to_string(),
                ));
            }
            let l = layout(field_headers, row_depth.unwrap_or(2), k, detected);
            let mut groups = build(&rows, row_keys, 0);
            // Row sort keys beyond the fields sort by the row totals.
            let totals: Grid = values.clone();
            model.sort_groups(&mut groups, 0, k, &row_specs, &function, &totals, cell);

            // Columns: each combination of column keys, sorted by its keys.
            let mut columns: Vec<(Vec<ArrayNode>, Vec<usize>)> = vec![];
            for &r in &rows {
                match columns.iter_mut().find(|(key, _)| {
                    key.iter()
                        .zip(&col_keys[r])
                        .all(|(a, b)| compare(a, b) == Ordering::Equal)
                }) {
                    Some((_, members)) => members.push(r),
                    None => columns.push((col_keys[r].clone(), vec![r])),
                }
            }
            columns.sort_by(|(a, _), (b, _)| {
                let mut order: Vec<(usize, bool)> = col_specs
                    .iter()
                    .filter(|s| (1..=c).contains(&(s.unsigned_abs() as usize)))
                    .map(|s| (s.unsigned_abs() as usize - 1, *s < 0))
                    .collect();
                order.extend((0..c).map(|i| (i, false)));
                for (i, descending) in order {
                    let o = compare(&a[i], &b[i]);
                    if o != Ordering::Equal {
                        return if descending { o.reverse() } else { o };
                    }
                }
                Ordering::Equal
            });
            let total_column = col_depth != 0;
            let total_left = col_depth < 0;

            let mut out: Grid = vec![];
            for level in 0..c {
                let mut header = vec![blank(); k];
                if l.show_headers && level + 1 == c {
                    header = row_keys[0].clone();
                }
                let total = if level == 0 { text("Total") } else { blank() };
                if total_column && total_left {
                    header.push(total.clone());
                }
                header.extend(columns.iter().map(|(key, _)| key[level].clone()));
                if total_column && !total_left {
                    header.push(total);
                }
                out.push(header);
            }
            let line = |model: &mut Self, members: &[usize], label: Vec<ArrayNode>| {
                let mut row = label;
                let mut cells = vec![];
                for (_, column_rows) in &columns {
                    let both: Vec<usize> = members
                        .iter()
                        .copied()
                        .filter(|r| column_rows.contains(r))
                        .collect();
                    cells.push(if both.is_empty() {
                        blank()
                    } else {
                        model.group_value(&function, &values, &both, 0, cell)
                    });
                }
                let total = model.group_value(&function, &values, members, 0, cell);
                if total_column && total_left {
                    row.push(total.clone());
                }
                row.extend(cells);
                if total_column && !total_left {
                    row.push(total);
                }
                row
            };
            let mut grand = vec![text("Total")];
            grand.resize(k, blank());
            if l.totals && l.top {
                out.push(line(model, &rows, grand.clone()));
            }
            let mut stack: Vec<(&Group, Vec<ArrayNode>, bool)> =
                groups.iter().rev().map(|g| (g, vec![], false)).collect();
            while let Some((g, prefix, closing)) = stack.pop() {
                let mut p = prefix.clone();
                p.push(g.key.clone());
                let mut label = p.clone();
                label.resize(k, blank());
                if g.children.is_empty() {
                    out.push(line(model, &g.rows, p));
                } else if closing {
                    out.push(line(model, &g.rows, label));
                } else {
                    if l.subtotals && l.top {
                        out.push(line(model, &g.rows, label));
                    }
                    if l.subtotals && !l.top {
                        stack.push((g, prefix, true));
                    }
                    for child in g.children.iter().rev() {
                        stack.push((child, p.clone(), false));
                    }
                }
            }
            if l.totals && !l.top {
                out.push(line(model, &rows, grand));
            }
            Ok(out)
        };
        match run(self) {
            Ok(grid) => super::array_result(grid, cell),
            Err(e) => e,
        }
    }
}
