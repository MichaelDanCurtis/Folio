//! BAHTTEXT, DBCS/JIS, PHONETIC, ENCODEURL, HYPERLINK and EUROCONVERT.
use crate::{
    calc_result::CalcResult,
    expressions::{parser::Node, token::Error, types::CellReferenceIndex},
    model::Model,
};

const THAI_DIGITS: [&str; 10] = [
    "ศูนย์",
    "หนึ่ง",
    "สอง",
    "สาม",
    "สี่",
    "ห้า",
    "หก",
    "เจ็ด",
    "แปด",
    "เก้า",
];
const THAI_PLACES: [&str; 6] = ["", "สิบ", "ร้อย", "พัน", "หมื่น", "แสน"];

/// Thai words for a whole number, as BAHTTEXT writes them: groups of six
/// digits joined by ล้าน (million), สิบ for 10, ยี่สิบ for 20, and เอ็ด for a
/// final 1 after higher digits.
fn thai_number(n: u64) -> String {
    if n == 0 {
        return THAI_DIGITS[0].to_string();
    }
    let digits: Vec<u32> = n
        .to_string()
        .chars()
        .map(|c| c.to_digit(10).unwrap_or(0))
        .collect();
    let mut out = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        let position = len - 1 - i;
        let place = position % 6;
        let d = *d as usize;
        if d != 0 {
            let higher = digits[..i].iter().any(|x| *x != 0);
            out.push_str(match (place, d) {
                (1, 1) => "",
                (1, 2) => "ยี่",
                (0, 1) if higher => "เอ็ด",
                _ => THAI_DIGITS[d],
            });
            out.push_str(THAI_PLACES[place]);
        }
        if place == 0 && position > 0 {
            out.push_str("ล้าน");
        }
    }
    out
}

pub(crate) fn bahttext(value: f64) -> String {
    let negative = value < 0.0;
    let satang_total = (value.abs() * 100.0).round() as u64;
    let (baht, satang) = (satang_total / 100, satang_total % 100);
    let mut out = String::new();
    if negative && satang_total > 0 {
        out.push_str("ลบ");
    }
    if baht > 0 || satang == 0 {
        out.push_str(&thai_number(baht));
        out.push_str("บาท");
    }
    if satang == 0 {
        out.push_str("ถ้วน");
    } else {
        out.push_str(&thai_number(satang));
        out.push_str("สตางค์");
    }
    out
}

/// Half-width katakana, in code point order from U+FF61, as full-width.
const KATAKANA: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// DBCS: half-width ASCII and katakana as full-width characters.
pub(crate) fn dbcs(text: &str) -> String {
    let katakana: Vec<char> = KATAKANA.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(text.len());
    for c in text.chars() {
        let cp = c as u32;
        match cp {
            0x20 => out.push('\u{3000}'),
            0x21..=0x7E => out.push(char::from_u32(cp - 0x21 + 0xFF01).unwrap_or(c)),
            0xFF61..=0xFF9F => {
                let full = katakana[(cp - 0xFF61) as usize];
                // A voiced or semi-voiced mark joins the character before it.
                let previous = out.last().copied().map(|p| p as u32);
                let joined = match (full, previous) {
                    ('゛', Some(0x30A6)) => Some(0x30F4),
                    // カ…チ, ツ, テ, ト and ハ…ホ take a voiced mark.
                    ('゛', Some(p))
                        if ((0x30AB..=0x30C1).contains(&p) && p % 2 == 1)
                            || matches!(p, 0x30C4 | 0x30C6 | 0x30C8)
                            || ((0x30CF..=0x30DB).contains(&p) && (p - 0x30CF) % 3 == 0) =>
                    {
                        Some(p + 1)
                    }
                    ('゜', Some(p)) if (0x30CF..=0x30DB).contains(&p) && (p - 0x30CF) % 3 == 0 => {
                        Some(p + 2)
                    }
                    _ => None,
                };
                match joined.and_then(char::from_u32) {
                    Some(j) => *out.last_mut().unwrap_or(&mut ' ') = j,
                    None => out.push(full),
                }
            }
            _ => out.push(c),
        }
    }
    out.into_iter().collect()
}

pub(crate) fn encode_url(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// EUROCONVERT's currencies: units per euro and calculation precision.
fn euro_rate(code: &str) -> Option<(f64, i32)> {
    Some(match code {
        "BEF" | "LUF" => (40.3399, 0),
        "DEM" => (1.95583, 2),
        "ESP" => (166.386, 0),
        "FRF" => (6.55957, 2),
        "IEP" => (0.787564, 2),
        "ITL" => (1936.27, 0),
        "NLG" => (2.20371, 2),
        "ATS" => (13.7603, 2),
        "PTE" => (200.482, 1),
        "FIM" => (5.94573, 2),
        "GRD" => (340.75, 0),
        "SIT" => (239.64, 2),
        "EUR" => (1.0, 2),
        _ => return None,
    })
}

fn round_to(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round() / f
}

pub(crate) fn euroconvert(
    number: f64,
    source: &str,
    target: &str,
    full_precision: bool,
    triangulation: Option<i32>,
) -> Option<f64> {
    let (from, _) = euro_rate(source)?;
    let (to, precision) = euro_rate(target)?;
    if source == target {
        return Some(number);
    }
    let mut euros = number / from;
    if let (Some(digits), false, false) = (triangulation, source == "EUR", target == "EUR") {
        euros = round_to(euros, digits);
    }
    let result = euros * to;
    Some(if full_precision {
        result
    } else {
        round_to(result, precision)
    })
}

impl Model<'_> {
    pub(crate) fn fn_bahttext(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        match self.get_number(&args[0], cell) {
            Ok(n) => CalcResult::String(bahttext(n)),
            Err(e) => e,
        }
    }

    pub(crate) fn fn_dbcs(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        match self.get_string(&args[0], cell) {
            Ok(s) => CalcResult::String(dbcs(&s)),
            Err(e) => e,
        }
    }

    /// PHONETIC(reference): the reading of the top-left cell. Folio keeps no
    /// phonetic guides, so this is the cell's text.
    pub(crate) fn fn_phonetic(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        match self.evaluate_node_with_reference(&args[0], cell) {
            CalcResult::Range { left, .. } => match self.evaluate_cell(left) {
                CalcResult::String(s) => CalcResult::String(s),
                error @ CalcResult::Error { .. } => error,
                _ => CalcResult::String(String::new()),
            },
            error @ CalcResult::Error { .. } => error,
            _ => CalcResult::new_error(Error::NA, cell, "PHONETIC needs a reference".to_string()),
        }
    }

    pub(crate) fn fn_encodeurl(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        match self.get_string(&args[0], cell) {
            Ok(s) => CalcResult::String(encode_url(&s)),
            Err(e) => e,
        }
    }

    /// HYPERLINK(link_location, [friendly_name]): the cell shows the friendly
    /// name, or the link when there is none.
    pub(crate) fn fn_hyperlink(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let link = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(e) => return e,
        };
        if let Some(friendly) = args.get(1).filter(|a| !matches!(a, Node::EmptyArgKind)) {
            return match self.evaluate_node_in_context(friendly, cell) {
                CalcResult::EmptyCell | CalcResult::EmptyArg => CalcResult::Number(0.0),
                CalcResult::Range { .. } | CalcResult::Array(_) | CalcResult::Lambda(_) => {
                    CalcResult::new_error(Error::VALUE, cell, "Expected a value".to_string())
                }
                value => value,
            };
        }
        CalcResult::String(link)
    }

    pub(crate) fn fn_euroconvert(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(3..=5).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let number = match self.get_number(&args[0], cell) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let mut codes = vec![];
        for arg in &args[1..3] {
            match self.get_string(arg, cell) {
                Ok(s) => codes.push(s.trim().to_uppercase()),
                Err(e) => return e,
            }
        }
        let full = match args.get(3).filter(|a| !matches!(a, Node::EmptyArgKind)) {
            Some(a) => match self.get_boolean(a, cell) {
                Ok(b) => b,
                Err(e) => return e,
            },
            None => false,
        };
        let triangulation = match args.get(4).filter(|a| !matches!(a, Node::EmptyArgKind)) {
            Some(a) => match self.get_number(a, cell) {
                Ok(n) if n.trunc() >= 3.0 => Some(n.trunc() as i32),
                Ok(_) => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Triangulation precision must be at least 3".to_string(),
                    )
                }
                Err(e) => return e,
            },
            None => None,
        };
        match euroconvert(number, &codes[0], &codes[1], full, triangulation) {
            Some(x) => CalcResult::Number(x),
            None => CalcResult::new_error(Error::VALUE, cell, "Unknown currency code".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baht_words() {
        assert_eq!(bahttext(1234.0), "หนึ่งพันสองร้อยสามสิบสี่บาทถ้วน");
        assert_eq!(bahttext(11.0), "สิบเอ็ดบาทถ้วน");
        assert_eq!(bahttext(21.25), "ยี่สิบเอ็ดบาทยี่สิบห้าสตางค์");
        assert_eq!(bahttext(101.0), "หนึ่งร้อยเอ็ดบาทถ้วน");
        assert_eq!(bahttext(1.0), "หนึ่งบาทถ้วน");
        assert_eq!(bahttext(0.0), "ศูนย์บาทถ้วน");
        assert_eq!(bahttext(0.5), "ห้าสิบสตางค์");
        assert_eq!(bahttext(-2.0), "ลบสองบาทถ้วน");
        assert_eq!(bahttext(2_000_000.0), "สองล้านบาทถ้วน");
        assert_eq!(bahttext(12_000_010.0), "สิบสองล้านสิบบาทถ้วน");
    }

    #[test]
    fn full_width() {
        assert_eq!(dbcs("Ab 1!"), "Ａｂ\u{3000}１！");
        assert_eq!(dbcs("ｶﾞｷﾞｸﾞﾊﾟﾎﾞｳﾞｱ"), "ガギグパボヴア");
        assert_eq!(dbcs("ﾃｽﾄ"), "テスト");
        assert_eq!(dbcs("日本"), "日本");
    }

    #[test]
    fn urls_and_euros() {
        assert_eq!(encode_url("a b/c?x=ü"), "a%20b%2Fc%3Fx%3D%C3%BC");
        assert_eq!(euroconvert(1.0, "EUR", "DEM", false, None), Some(1.96));
        assert_eq!(euroconvert(1.2, "DEM", "EUR", false, None), Some(0.61));
        assert_eq!(euroconvert(1.0, "FRF", "DEM", false, Some(3)), Some(0.3));
        assert!(
            (euroconvert(1.0, "FRF", "DEM", true, None).unwrap() - 1.95583 / 6.55957).abs() < 1e-12
        );
        assert_eq!(euroconvert(1.0, "XXX", "DEM", false, None), None);
    }
}
