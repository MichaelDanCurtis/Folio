//! One display projection for the grid, sizing, and clipboard. Formatting never
//! changes a calculated value or the source OOXML number-format code.
use crate::{formula::parse_address, office::Package};
use serde_json::{json, Value};
use ssf_rs::{format::choose_fmt, Value as SsfValue};
use std::collections::HashMap;

pub fn settings(pkg: Option<&Package>) -> (bool, Vec<String>) {
    let date1904 = pkg
        .and_then(|p| p.text("xl/workbook.xml").ok())
        .and_then(|xml| roxmltree::Document::parse(xml).ok())
        .is_some_and(|d| {
            d.descendants().any(|n| {
                n.has_tag_name((S, "workbookPr"))
                    && matches!(n.attribute("date1904"), Some("1" | "true"))
            })
        });
    let palette = pkg
        .and_then(|p| p.text("xl/styles.xml").ok())
        .and_then(|xml| roxmltree::Document::parse(xml).ok())
        .and_then(|d| {
            d.descendants()
                .find(|n| n.has_tag_name((S, "indexedColors")))
                .map(|n| {
                    n.children()
                        .filter(|n| n.has_tag_name((S, "rgbColor")))
                        .map(|n| n.attribute("rgb").unwrap_or("").to_string())
                        .collect()
                })
        })
        .unwrap_or_default();
    (date1904, palette)
}
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub fn code<'a>(sheet: &'a Value, address: &str) -> &'a str {
    let defaults = &sheet["styleDefaults"];
    let mut base = &defaults["default"];
    if let Ok((r, c)) = parse_address(address) {
        if let Some(columns) = defaults["columns"].as_array() {
            for col in columns {
                if col["min"].as_u64().is_some_and(|n| c + 1 >= n as usize)
                    && col["max"].as_u64().is_some_and(|n| c < n as usize)
                {
                    base = &col["style"];
                }
            }
        }
        if let Some(row) = defaults["rows"].get((r + 1).to_string()) {
            base = row;
        }
    }
    if !sheet["officeStyles"][address].is_null() {
        base = &defaults["default"];
    }
    sheet["cellStyles"][address]["numFmt"]
        .as_str()
        .or_else(|| base["numFmt"].as_str())
        .unwrap_or("General")
}

/// Split only grammar tokens: quoted/escaped characters never become layout
/// directives. Markers are chosen outside the source's characters.
fn layout_code(code: &str, value: &str) -> Result<(String, HashMap<char, (String, char)>), String> {
    ssf_rs::checked::validate_shape(code)?;
    let mut result = String::new();
    let mut directives = HashMap::new();
    let mut chars = code.chars().peekable();
    let mut quoted = false;
    let mut bracket = false;
    let mut marker = 0xe000;
    while let Some(c) = chars.next() {
        match c {
            '"' if !bracket => {
                quoted = !quoted;
                result.push(c);
            }
            '\\' if !quoted && !bracket => {
                result.push(c);
                result.push(
                    chars
                        .next()
                        .ok_or("Escape must be followed by a character.")?,
                );
            }
            '[' if !quoted => {
                bracket = true;
                result.push(c);
            }
            ']' if !quoted => {
                if !bracket {
                    return Err("Unmatched format bracket.".into());
                }
                bracket = false;
                result.push(c);
            }
            '_' | '*' if !quoted && !bracket => {
                let glyph = chars
                    .next()
                    .ok_or("Spacing must be followed by a character.")?;
                while code.contains(char::from_u32(marker).unwrap())
                    || value.contains(char::from_u32(marker).unwrap())
                {
                    marker += 1;
                }
                let token = char::from_u32(marker).unwrap();
                marker += 1;
                directives.insert(
                    token,
                    (if c == '_' { "skip" } else { "fill" }.into(), glyph),
                );
                result.push('"');
                result.push(token);
                result.push('"');
            }
            _ => result.push(c),
        }
    }
    if quoted {
        return Err("Close the quoted text in the number format.".into());
    }
    if bracket {
        return Err("Close the bracket in the number format.".into());
    }
    Ok((result, directives))
}

fn section_color(section: &str, palette: &[String]) -> Option<String> {
    let mut quoted = false;
    let mut chars = section.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' | '_' | '*' if !quoted => {
                chars.next();
            }
            '[' if !quoted => {
                let token: String = chars.by_ref().take_while(|c| *c != ']').collect();
                let lower = token.to_ascii_lowercase();
                let named = match lower.as_str() {
                    "black" => Some("#000000"),
                    "blue" => Some("#0000FF"),
                    "cyan" => Some("#00FFFF"),
                    "green" => Some("#00FF00"),
                    "magenta" => Some("#FF00FF"),
                    "red" => Some("#FF0000"),
                    "white" => Some("#FFFFFF"),
                    "yellow" => Some("#FFFF00"),
                    _ => None,
                };
                if let Some(color) = named {
                    return Some(color.into());
                }
                if let Some(index) = lower
                    .strip_prefix("color")
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .filter(|n| (1..=56).contains(n))
                {
                    if let Some(rgb) = palette.get(index + 7).filter(|s| {
                        (s.len() == 6 || s.len() == 8) && s.bytes().all(|b| b.is_ascii_hexdigit())
                    }) {
                        return Some(format!("#{}", &rgb[rgb.len() - 6..]));
                    }
                    return Some(ironcalc_base::colors::get_indexed_color((index + 7) as i32));
                }
            }
            _ => {}
        }
    }
    None
}

/// Classify the selected numeric section once in the engine. The desktop
/// renderer must not reparse conditions, escaped literals or format sections.
fn number_width_mode(section: &str, is_date: bool) -> Option<&'static str> {
    let mut body = section.trim();
    while let Some(rest) = body.strip_prefix('[') {
        let end = rest.find(']')?;
        body = rest[end + 1..].trim_start();
    }
    if body.eq_ignore_ascii_case("General") {
        return Some("general");
    }
    if is_date {
        return Some("fixed");
    }
    let mut chars = section.chars();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' | '_' | '*' if !quoted => {
                chars.next();
            }
            '[' if !quoted => {
                chars.by_ref().take_while(|c| *c != ']').for_each(drop);
            }
            '0' | '#' | '?' if !quoted => return Some("fixed"),
            _ => {}
        }
    }
    None
}

pub fn plain(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        Value::Number(n) => ssf_rs::format_str("General", n.as_f64().unwrap_or(0.0))
            .unwrap_or_else(|_| n.to_string()),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn render_inner(
    value: &Value,
    code: &str,
    date1904: bool,
    palette: &[String],
) -> Result<Value, String> {
    let (layout, directives) = layout_code(code, &plain(value))?;
    let input = match value {
        Value::Number(n) => SsfValue::Num(n.as_f64().ok_or("Invalid number")?),
        _ => SsfValue::Text(plain(value)),
    };
    // Boolean cells do not participate in number/text sections in Excel.
    if value.is_boolean() || value.is_null() {
        return Ok(json!({"text":plain(value)}));
    }
    let (numeric_sections, section) = choose_fmt(code, &input)?;
    let is_date = ssf_rs::is_date(&section);
    let invalid_date = value.as_f64().is_some_and(|v| {
        is_date && (v < 0.0 || v >= 2_958_466.0 - if date1904 { 1462.0 } else { 0.0 })
    });
    let output = if invalid_date {
        "########".into()
    } else {
        ssf_rs::format(&layout, &input, date1904)?
    };
    if output.len() > 100_000 {
        return Err("The formatted result is too long.".into());
    }
    let mut text = String::new();
    let mut run = String::new();
    let mut segments = vec![];
    for c in output.chars() {
        if let Some((kind, glyph)) = directives.get(&c) {
            if !run.is_empty() {
                segments.push(json!({"kind":"text","text":run}));
                run.clear();
            }
            segments.push(json!({"kind":kind,"text":glyph.to_string()}));
            if kind == "skip" {
                text.push(' ');
            }
        } else {
            text.push(c);
            run.push(c);
        }
    }
    if !run.is_empty() {
        segments.push(json!({"kind":"text","text":run}));
    }
    let mut result = json!({"text":text});
    if value.is_number() && numeric_sections > 0 {
        if invalid_date {
            result["numberWidth"] = json!("invalidDate");
        } else if let Some(mode) = number_width_mode(&section, is_date) {
            result["numberWidth"] = json!(mode);
        }
    }
    if segments.iter().any(|s| s["kind"] != "text") {
        result["segments"] = json!(segments);
    }
    if let Some(color) = section_color(&section, palette) {
        result["color"] = json!(color);
    }
    if invalid_date {
        result["warning"] = json!("This value is outside the workbook's displayable date range.");
    }
    Ok(result)
}

pub fn render(value: &Value, code: &str, date1904: bool, palette: &[String]) -> Value {
    // A malformed imported format must not panic across FFI or hide the value.
    match std::panic::catch_unwind(|| render_inner(value, code, date1904, palette)) {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => json!({"text":plain(value),"warning":e}),
        Err(_) => json!({"text":plain(value),"warning":"This number format is not supported yet."}),
    }
}

pub fn validate(code: &str) -> Result<(), String> {
    layout_code(code, "")?;
    for value in [json!(1234.567), json!(-1234.567), json!(0), json!("text")] {
        if let Some(w) = render(&value, code, false, &[])["warning"].as_str() {
            // A negative numeric date is an invalid value, not an invalid code.
            if w.contains("outside the workbook") {
                continue;
            }
            return Err(w.into());
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct Display {
    previous: Value,
    previous_values: Value,
    cache: HashMap<String, Value>,
    pub values: Value,
    pub date1904: bool,
    pub palette: Vec<String>,
}
impl Display {
    pub fn sync(
        &mut self,
        sheets: &Value,
        calculated: &Value,
        errors: &Value,
        generation: u64,
        conditional: &Value,
        date1904: bool,
        palette: Vec<String>,
    ) {
        let state = json!({"conditional":conditional,"generation":generation,"errors":errors,"date1904":date1904,"palette":palette,"styles":sheets.as_array().map(|s| s.iter().map(|s| json!({"id":s["id"],"cellStyles":s["cellStyles"],"officeStyles":s["officeStyles"],"styleDefaults":s["styleDefaults"]})).collect::<Vec<_>>())});
        if state == self.previous && *calculated == self.previous_values {
            return;
        }
        if self.date1904 != date1904 || self.palette != palette {
            self.cache.clear();
        }
        self.date1904 = date1904;
        self.palette = palette;
        let mut all = serde_json::Map::new();
        for sheet in sheets.as_array().into_iter().flatten() {
            let id = sheet["id"].as_str().unwrap_or("");
            let mut display = serde_json::Map::new();
            for (address, raw) in sheet["cells"].as_object().into_iter().flatten() {
                let fallback = json!(raw
                    .as_str()
                    .unwrap_or("")
                    .strip_prefix('\'')
                    .unwrap_or(raw.as_str().unwrap_or("")));
                let value = calculated[id].get(address).unwrap_or(&fallback);
                if errors[id][address] == true {
                    display.insert(
                        address.clone(),
                        json!({"text":plain(value),"isError":true,"conditionalStyle":conditional[id][address]}),
                    );
                    continue;
                }
                let format = conditional[id][address]["numFmt"]
                    .as_str()
                    .unwrap_or_else(|| code(sheet, address));
                let key = json!([format, value]).to_string();
                let output = if let Some(v) = self.cache.get(&key) {
                    v.clone()
                } else {
                    let result = render(value, format, date1904, &self.palette);
                    if self.cache.len() >= 16384 {
                        self.cache.clear();
                    }
                    self.cache.insert(key, result.clone());
                    result
                };
                let mut output = output;
                if let Some(style) = conditional[id].get(address) {
                    output["conditionalStyle"] = style.clone();
                }
                display.insert(address.clone(), output);
            }
            all.insert(id.into(), Value::Object(display));
        }
        self.values = Value::Object(all);
        self.previous = state;
        self.previous_values = calculated.clone();
    }
}
