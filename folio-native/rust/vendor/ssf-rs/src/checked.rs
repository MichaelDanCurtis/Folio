//! Folio's bounded entry points shared by formula and cell-display callers.
use crate::{format::choose_fmt, Value};

pub fn validate_shape(code: &str) -> Result<(), String> {
    if code.chars().count() > 255 {
        return Err("Use up to 255 characters in a number format.".into());
    }
    let mut chars = code.chars();
    let mut quoted = false;
    let mut bracket = false;
    let mut sections = 1;
    while let Some(c) = chars.next() {
        match c {
            '"' if !bracket => quoted = !quoted,
            '\\' | '_' | '*' if !quoted && !bracket => {
                if chars.next().is_none() {
                    return Err("Escape or spacing must be followed by a character.".into());
                }
            }
            '[' if !quoted => {
                if bracket {
                    return Err("Nested format brackets are invalid.".into());
                }
                bracket = true;
            }
            ']' if !quoted => {
                if !bracket {
                    return Err("Unmatched format bracket.".into());
                }
                bracket = false;
            }
            ';' if !quoted && !bracket => sections += 1,
            _ => {}
        }
    }
    if quoted {
        return Err("Close the quoted text in the number format.".into());
    }
    if bracket {
        return Err("Close the bracket in the number format.".into());
    }
    if sections > 4 {
        return Err("Use at most four number-format sections.".into());
    }
    Ok(())
}

/// TEXT produces characters, not a measured cell layout. Fill characters are
/// omitted and skip-width directives become spaces, as in the base formatter.
pub fn number(code: &str, value: f64, date1904: bool) -> Result<String, String> {
    validate_shape(code)?;
    if !value.is_finite() {
        return Err("Invalid number".into());
    }
    let input = Value::Num(value);
    let (_, section) = choose_fmt(code, &input)?;
    if crate::is_date(&section)
        && (value < 0.0 || value >= 2958466.0 - if date1904 { 1462.0 } else { 0.0 })
    {
        return Err("Date is outside the workbook's date system.".into());
    }
    match std::panic::catch_unwind(|| crate::format(code, &input, date1904)) {
        Ok(Ok(text)) if text.encode_utf16().count() <= 32767 => Ok(text),
        Ok(Ok(_)) => Err("Formatted text exceeds the cell text limit.".into()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("This number format is not supported yet.".into()),
    }
}
