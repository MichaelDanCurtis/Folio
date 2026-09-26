//! FOLIO PATCH: Excel worksheet functions IronCalc lacks (X08.03).
//!
//! - `aggregate`: AGGREGATE.
//! - `text`: BAHTTEXT, DBCS/JIS, PHONETIC, ENCODEURL, HYPERLINK and EUROCONVERT.
//! - `filterxml`: FILTERXML.
//! - `groupby`: GROUPBY and PIVOTBY.
//! - `ets`: the FORECAST.ETS family.
//!
//! Function names used as values (`BYROW(A1:C3, SUM)`) are in
//! `logical/lambda.rs`.
mod aggregate;
mod ets;
mod filterxml;
mod groupby;
mod text;

use crate::{
    calc_result::CalcResult,
    expressions::{parser::ArrayNode, token::Error, types::CellReferenceIndex},
};

/// An array result: a single value when it is 1×1, as Excel returns it.
pub(crate) fn array_result(rows: Vec<Vec<ArrayNode>>, cell: CellReferenceIndex) -> CalcResult {
    if rows.is_empty() || rows[0].is_empty() {
        return CalcResult::new_error(Error::CALC, cell, "Empty result".to_string());
    }
    if rows.len() == 1 && rows[0].len() == 1 {
        return match &rows[0][0] {
            ArrayNode::Number(n) => CalcResult::Number(*n),
            ArrayNode::Boolean(b) => CalcResult::Boolean(*b),
            ArrayNode::String(s) => CalcResult::String(s.clone()),
            ArrayNode::Error(e) => CalcResult::new_error(e.clone(), cell, String::new()),
            ArrayNode::Empty => CalcResult::EmptyCell,
        };
    }
    CalcResult::Array(rows)
}
