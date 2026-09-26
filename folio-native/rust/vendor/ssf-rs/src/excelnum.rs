//! Folio: Excel's number sections, rendered from placeholders rather than
//! `ssf`'s list of recognized shapes. Handles digit placeholders (`0 # ?`),
//! grouping and scaling commas, percent, fractions and scientific notation
//! with literal text anywhere, as Excel's TEXT does. Dates and General stay
//! with `eval_fmt`.

#[derive(Debug, Clone, PartialEq)]
enum T {
    Lit(String),
    Ph(char),
    Dot,
    Comma,
    Pct,
    Exp { plus: bool, letter: char },
    Slash,
    Digits(String),
}

/// Splits a number section into tokens. None when it isn't a plain number
/// section (text placeholder, General, elapsed time).
fn tokens(section: &str) -> Option<Vec<T>> {
    let f: Vec<char> = section.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut seen_dot = false;
    let mut seen_slash = false;
    let mut seen_exp = false;
    while i < f.len() {
        let c = f[i];
        match c {
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < f.len() && f[i] != '"' {
                    s.push(f[i]);
                    i += 1;
                }
                out.push(T::Lit(s));
            }
            '\\' => {
                i += 1;
                if let Some(&x) = f.get(i) {
                    out.push(T::Lit(x.to_string()));
                }
            }
            '_' => {
                i += 1;
                out.push(T::Lit(" ".into()));
            }
            '*' => i += 1,
            '[' => {
                let mut s = String::new();
                i += 1;
                while i < f.len() && f[i] != ']' {
                    s.push(f[i]);
                    i += 1;
                }
                let lower = s.to_lowercase();
                if lower.chars().all(|c| "hms".contains(c)) && !lower.is_empty() {
                    return None;
                }
                if let Some(cur) = s.strip_prefix('$') {
                    let sym = cur.split('-').next().unwrap_or("");
                    out.push(T::Lit(sym.to_string()));
                }
            }
            '0' | '#' | '?' => {
                if seen_slash
                    && matches!(out.last(), Some(T::Digits(_)))
                    && c == '0'
                {
                    if let Some(T::Digits(d)) = out.last_mut() {
                        d.push('0');
                    }
                } else {
                    out.push(T::Ph(c));
                }
            }
            '1'..='9' if seen_slash && !out[out.iter().rposition(|t| *t == T::Slash)?..].iter().any(|t| matches!(t, T::Ph(_))) => {
                if let Some(T::Digits(d)) = out.last_mut() {
                    d.push(c);
                } else {
                    out.push(T::Digits(c.to_string()));
                }
            }
            '.' if !seen_dot && !seen_exp && !seen_slash => {
                seen_dot = true;
                out.push(T::Dot);
            }
            ',' => out.push(T::Comma),
            '%' => out.push(T::Pct),
            'E' | 'e' if !seen_exp && matches!(f.get(i + 1), Some('+') | Some('-')) => {
                seen_exp = true;
                out.push(T::Exp {
                    plus: f[i + 1] == '+',
                    letter: c,
                });
                i += 1;
            }
            '/' if !seen_slash && !seen_exp => {
                seen_slash = true;
                out.push(T::Slash);
            }
            '@' => return None,
            'G' | 'g' if section[i..].to_lowercase().starts_with("general") => return None,
            _ => out.push(T::Lit(c.to_string())),
        }
        i += 1;
    }
    Some(out)
}

/// `x` rounded half away from zero at `nf` decimals, from its shortest
/// decimal form as Excel does (0.035 shows as 0.04), as integer and
/// fraction digit strings.
fn round_digits(x: f64, nf: usize) -> (String, String) {
    // Excel works to 15 significant digits.
    let s = format!("{:.14e}", x.abs());
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    // digits d0.d1d2... × 10^exp
    let point = exp + 1; // digits before the decimal point
    let mut all: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let mut point = point;
    if point < 0 {
        let mut v = vec![0u8; (-point) as usize];
        v.extend(all);
        all = v;
        point = 0;
    }
    let keep = point as usize + nf;
    if all.len() < keep + 1 {
        all.resize(keep + 1, 0);
    }
    let round_up = all[keep] >= 5;
    all.truncate(keep);
    if round_up {
        let mut k = keep;
        loop {
            if k == 0 {
                all.insert(0, 1);
                point += 1;
                break;
            }
            k -= 1;
            if all[k] == 9 {
                all[k] = 0;
            } else {
                all[k] += 1;
                break;
            }
        }
    }
    let p = point as usize;
    let int: String = all[..p].iter().map(|d| (d + b'0') as char).collect();
    let frac: String = all[p..].iter().map(|d| (d + b'0') as char).collect();
    let int = int.trim_start_matches('0').to_string();
    (int, frac)
}

/// Integer digits into placeholders, right to left: the leftmost takes any
/// extra digits; unfilled `0` shows 0, `?` a space, `#` nothing. Returns one
/// string per placeholder.
fn place_int(digits: &str, phs: &[char]) -> Vec<String> {
    let d: Vec<char> = digits.chars().collect();
    let mut out = vec![String::new(); phs.len()];
    let mut j = d.len();
    for k in (0..phs.len()).rev() {
        if j > 0 {
            j -= 1;
            out[k] = d[j].to_string();
        } else {
            out[k] = match phs[k] {
                '0' => "0".into(),
                '?' => " ".into(),
                _ => String::new(),
            };
        }
    }
    if j > 0 && !phs.is_empty() {
        out[0] = format!("{}{}", d[..j].iter().collect::<String>(), out[0]);
    }
    out
}

/// Fraction digits into placeholders, left to right; trailing zeros drop for
/// `#` and become spaces for `?`.
fn place_frac(digits: &str, phs: &[char]) -> Vec<String> {
    let d: Vec<char> = digits.chars().collect();
    let mut out: Vec<String> = phs
        .iter()
        .enumerate()
        .map(|(k, _)| d.get(k).copied().unwrap_or('0').to_string())
        .collect();
    for k in (0..phs.len()).rev() {
        if out[k] != "0" {
            break;
        }
        match phs[k] {
            '#' => out[k] = String::new(),
            '?' => out[k] = " ".into(),
            _ => break,
        }
    }
    out
}

/// Thousands separators over the integer part: a `,` between digits, a space
/// where the digit to its left is padding.
fn group(int: &str) -> String {
    let chars: Vec<char> = int.chars().collect();
    let mut out = Vec::new();
    for (k, c) in chars.iter().rev().enumerate() {
        if k > 0 && k % 3 == 0 {
            out.push(if *c == ' ' { ' ' } else { ',' });
        }
        out.push(*c);
    }
    out.iter().rev().collect()
}

/// Integer placeholders joined, with grouping, when no literal sits between
/// them; otherwise each placeholder's text in place.
fn render_int(tokens: &[T], digits: &str, grouping: bool, out: &mut [String]) {
    let idx: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| matches!(t, T::Ph(_)))
        .map(|(i, _)| i)
        .collect();
    let phs: Vec<char> = idx
        .iter()
        .map(|&i| match tokens[i] {
            T::Ph(c) => c,
            _ => unreachable!(),
        })
        .collect();
    let placed = place_int(digits, &phs);
    if grouping {
        let joined: String = placed.concat();
        if let Some(&first) = idx.first() {
            out[first] = group(&joined);
            for &i in &idx[1..] {
                out[i] = String::new();
            }
        }
        return;
    }
    for (k, &i) in idx.iter().enumerate() {
        out[i] = placed[k].clone();
    }
}

fn is_ph(t: &T) -> bool {
    matches!(t, T::Ph(_))
}

/// Renders `value` (its sign shown as a leading `-`) with a number section.
/// None when the section isn't one this renderer handles.
pub fn render(section: &str, value: f64) -> Option<String> {
    let toks = tokens(section)?;
    if !toks.iter().any(is_ph) {
        return None;
    }
    let neg = value < 0.0;
    let x = value.abs();
    let body = if let Some(e) = toks.iter().position(|t| matches!(t, T::Exp { .. })) {
        scientific(&toks, e, x)
    } else if let Some(s) = fraction_slash(&toks) {
        fraction(&toks, s, x)?
    } else {
        decimal(&toks, x)
    };
    Some(if neg && body.chars().any(|c| c.is_ascii_digit() && c != '0') {
        format!("-{body}")
    } else if neg {
        // Excel keeps the sign of a value that rounds to zero.
        format!("-{body}")
    } else {
        body
    })
}

fn fraction_slash(toks: &[T]) -> Option<usize> {
    let s = toks.iter().position(|t| *t == T::Slash)?;
    let before = toks[..s].iter().any(is_ph);
    let after = toks[s + 1..]
        .iter()
        .any(|t| is_ph(t) || matches!(t, T::Digits(_)));
    (before && after).then_some(s)
}

fn decimal(toks: &[T], x: f64) -> String {
    let dot = toks.iter().position(|t| *t == T::Dot);
    let int_end = dot.unwrap_or(toks.len());
    let last_int_ph = toks[..int_end].iter().rposition(is_ph);
    let first_ph = toks.iter().position(is_ph);
    let mut scale = 0i32;
    let mut grouping = false;
    for (i, t) in toks.iter().enumerate() {
        match t {
            T::Pct => scale += 2,
            T::Comma => {
                let ph_before = first_ph.is_some_and(|f| f < i);
                if !ph_before {
                    continue;
                }
                if i < int_end && last_int_ph.is_some_and(|l| l > i) {
                    grouping = true;
                } else if i < int_end || !toks[i + 1..].iter().any(is_ph) {
                    // Right of the last integer digit, or of all digits.
                    scale -= 3;
                }
            }
            _ => {}
        }
    }
    let x = x * 10f64.powi(scale);
    // A comma before any digit placeholder is shown as itself.
    let mut toks = toks.to_vec();
    for t in toks[..first_ph.unwrap_or(0)].iter_mut() {
        if *t == T::Comma {
            *t = T::Lit(",".into());
        }
    }
    let toks = &toks[..];
    let frac_phs: Vec<char> = dot
        .map(|d| {
            toks[d + 1..]
                .iter()
                .filter_map(|t| match t {
                    T::Ph(c) => Some(*c),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let (int, frac) = round_digits(x, frac_phs.len());
    let mut out = vec![String::new(); toks.len()];
    render_int(&toks[..int_end], &int, grouping, &mut out[..int_end]);
    if let Some(d) = dot {
        let placed = place_frac(&frac, &frac_phs);
        let mut k = 0;
        for i in d + 1..toks.len() {
            if is_ph(&toks[i]) {
                out[i] = placed[k].clone();
                k += 1;
            }
        }
    }
    join(toks, out)
}

fn join(toks: &[T], mut out: Vec<String>) -> String {
    for (i, t) in toks.iter().enumerate() {
        match t {
            T::Lit(s) => out[i] = s.clone(),
            T::Dot => out[i] = ".".into(),
            T::Pct => out[i] = "%".into(),
            T::Slash => out[i] = "/".into(),
            _ => {}
        }
    }
    out.concat()
}

fn scientific(toks: &[T], e: usize, x: f64) -> String {
    let (plus, letter) = match toks[e] {
        T::Exp { plus, letter } => (plus, letter),
        _ => unreachable!(),
    };
    let mant = &toks[..e];
    let dot = mant.iter().position(|t| *t == T::Dot);
    let int_end = dot.unwrap_or(mant.len());
    let n = mant[..int_end].iter().filter(|t| is_ph(t)).count().max(1) as i32;
    let last_int_ph = mant[..int_end].iter().rposition(is_ph);
    let grouping = mant[..int_end]
        .iter()
        .enumerate()
        .any(|(i, t)| *t == T::Comma && last_int_ph.is_some_and(|l| l > i) && mant[..i].iter().any(is_ph));
    let frac_phs: Vec<char> = dot
        .map(|d| {
            mant[d + 1..]
                .iter()
                .filter_map(|t| match t {
                    T::Ph(c) => Some(*c),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let mut exp = if x == 0.0 {
        0
    } else {
        let e10 = x.log10().floor() as i32;
        e10.div_euclid(n) * n
    };
    let (mut int, mut frac) = round_digits(x / 10f64.powi(exp), frac_phs.len());
    if int.len() as i32 > n {
        exp += n;
        (int, frac) = round_digits(x / 10f64.powi(exp), frac_phs.len());
    }
    if x == 0.0 {
        // Zero fills every integer placeholder with 0.
        int = "0".repeat(n as usize);
    }
    let mut out = vec![String::new(); toks.len()];
    render_int(&mant[..int_end], &int, grouping, &mut out[..int_end]);
    if let Some(d) = dot {
        let placed = place_frac(&frac, &frac_phs);
        let mut k = 0;
        for i in d + 1..e {
            if is_ph(&toks[i]) {
                out[i] = placed[k].clone();
                k += 1;
            }
        }
    }
    out[e] = letter.to_string();
    // The exponent's digits, with its sign just before them.
    let exp_idx: Vec<usize> = (e + 1..toks.len()).filter(|&i| is_ph(&toks[i])).collect();
    let phs: Vec<char> = exp_idx
        .iter()
        .map(|&i| match toks[i] {
            T::Ph(c) => c,
            _ => unreachable!(),
        })
        .collect();
    let digits = exp.abs().to_string();
    let placed = place_int(&digits, &phs);
    for (k, &i) in exp_idx.iter().enumerate() {
        out[i] = placed[k].clone();
    }
    let sign = if exp < 0 { "-" } else if plus { "+" } else { "" };
    if let Some(&first) = exp_idx.first() {
        out[first] = format!("{sign}{}", out[first]);
    }
    let mut toks = toks.to_vec();
    for t in toks.iter_mut() {
        if matches!(t, T::Pct) {
            *t = T::Lit("%".into());
        }
    }
    let mut s = String::new();
    for (i, t) in toks.iter().enumerate() {
        match t {
            T::Lit(l) => s.push_str(l),
            T::Dot if i < e => s.push('.'),
            T::Dot => s.push('.'),
            T::Slash => s.push('/'),
            T::Comma => {}
            _ => s.push_str(&out[i]),
        }
    }
    s
}

/// The best fraction for `x` with a denominator of at most `max`.
fn approximate(x: f64, max: u64) -> (u64, u64) {
    let (mut p0, mut q0, mut p1, mut q1) = (0u64, 1u64, 1u64, 0u64);
    let mut v = x;
    for _ in 0..64 {
        let a = v.floor();
        let a_u = a as u64;
        let q2 = a_u.saturating_mul(q1).saturating_add(q0);
        if q2 > max {
            // The best semiconvergent within the limit.
            let k = (max - q0) / q1.max(1);
            let (pk, qk) = (p0 + k * p1, q0 + k * q1);
            let err_k = (x - pk as f64 / qk as f64).abs();
            let err_1 = (x - p1 as f64 / q1.max(1) as f64).abs();
            return if q1 != 0 && err_1 <= err_k { (p1, q1) } else { (pk, qk) };
        }
        let p2 = a_u.saturating_mul(p1).saturating_add(p0);
        (p0, q0, p1, q1) = (p1, q1, p2, q2);
        let r = v - a;
        if r < 1e-12 {
            break;
        }
        v = 1.0 / r;
    }
    (p1, q1.max(1))
}

fn fraction(toks: &[T], s: usize, x: f64) -> Option<String> {
    // Numerator: the placeholder run nearest the slash.
    let num_end = toks[..s].iter().rposition(is_ph)? + 1;
    let mut num_start = num_end - 1;
    while num_start > 0 && is_ph(&toks[num_start - 1]) {
        num_start -= 1;
    }
    let int_end = toks[..num_start].iter().rposition(is_ph).map(|i| i + 1);
    let den_end = toks[s + 1..]
        .iter()
        .rposition(|t| is_ph(t) || matches!(t, T::Digits(_)))
        .map(|i| s + 2 + i)?;
    let fixed = match toks[s + 1..den_end].iter().find_map(|t| match t {
        T::Digits(d) => Some(d),
        _ => None,
    }) {
        // An oversized denominator is left to the checked error path.
        Some(d) => Some(d.parse::<u64>().ok().filter(|d| *d > 0 && *d < 1 << 53)?),
        None => None,
    };
    let den_phs: Vec<char> = toks[s + 1..den_end]
        .iter()
        .filter_map(|t| match t {
            T::Ph(c) => Some(*c),
            _ => None,
        })
        .collect();
    let num_phs: Vec<char> = toks[num_start..num_end]
        .iter()
        .filter_map(|t| match t {
            T::Ph(c) => Some(*c),
            _ => None,
        })
        .collect();
    let ph_of = |r: std::ops::Range<usize>| -> Vec<char> {
        toks[r]
            .iter()
            .filter_map(|t| match t {
                T::Ph(c) => Some(*c),
                _ => None,
            })
            .collect()
    };
    let int_phs = int_end.map(|e| ph_of(0..e)).unwrap_or_default();
    let (mut ip, fpart) = if int_end.is_some() {
        (x.trunc(), x - x.trunc())
    } else {
        (0.0, x)
    };
    let (mut num, den) = match fixed {
        Some(d) => ((fpart * d as f64).round() as u64, d),
        None => {
            let max = 10u64.pow(den_phs.len().min(9) as u32) - 1;
            approximate(fpart, max.max(1))
        }
    };
    if int_end.is_some() && num == den && den != 0 {
        ip += 1.0;
        num = 0;
    }
    let mut out = vec![String::new(); toks.len()];
    let any_q = int_phs.contains(&'?') || num_phs.contains(&'?') || den_phs.contains(&'?');
    let num_zero_shows = num_phs.contains(&'0') || int_end.is_none();
    // A zero value shows its 0 in `0` and `?` integer placeholders.
    let shows_zero = x == 0.0 && (int_phs.contains(&'0') || int_phs.contains(&'?'));
    let ip_digits = if ip != 0.0 {
        format!("{}", ip as u64)
    } else if shows_zero {
        "0".into()
    } else {
        String::new()
    };

    if let Some(ie) = int_end {
        if num == 0 && !num_zero_shows {
            // A whole number: the integer, and the fraction's width as spaces
            // when it has `?` placeholders.
            let digits = if ip == 0.0 { "0".to_string() } else { ip_digits.clone() };
            render_int(&toks[..ie], &digits, false, &mut out[..ie]);
            let mut s_out = join_range(toks, &out, 0, ie);
            let width: usize = (ie..den_end).map(|i| token_width(&toks[i])).sum();
            if any_q {
                s_out.push_str(&" ".repeat(width));
            }
            s_out.push_str(&join_range(toks, &vec![String::new(); toks.len()], den_end, toks.len()));
            return Some(s_out);
        }
        render_int(&toks[..ie], &ip_digits, false, &mut out[..ie]);
        if ip == 0.0 && !int_phs.contains(&'0') && !shows_zero {
            // The separator between an empty integer and the numerator.
            let blank = int_phs.contains(&'?') || num_phs.contains(&'?');
            for i in ie..num_start {
                if let T::Lit(l) = &toks[i] {
                    out[i] = if blank { " ".repeat(l.chars().count()) } else { DROPPED.into() };
                }
            }
        }
    }
    let placed = place_int(&num.to_string(), &num_phs);
    let mut k = 0;
    for i in num_start..num_end {
        if is_ph(&toks[i]) {
            out[i] = placed[k].clone();
            k += 1;
        }
    }
    let den_s = den.to_string();
    let den_idx: Vec<usize> = (s + 1..den_end).filter(|&i| is_ph(&toks[i])).collect();
    if fixed.is_some() {
        for i in s + 1..den_end {
            if let T::Digits(d) = &toks[i] {
                out[i] = d.clone();
            }
        }
    } else {
        // Denominators pad with `0` on the left and `?` on the right.
        let d: Vec<char> = den_s.chars().collect();
        let n = den_idx.len();
        if d.len() >= n {
            for (k, &i) in den_idx.iter().enumerate() {
                out[i] = if k == 0 {
                    d[..d.len() - n + 1].iter().collect()
                } else {
                    d[d.len() - n + k].to_string()
                };
            }
        } else {
            let zeros = den_phs.iter().filter(|c| **c == '0').count();
            let lead = zeros.min(n - d.len());
            for (k, &i) in den_idx.iter().enumerate() {
                out[i] = if k < lead {
                    "0".into()
                } else if k - lead < d.len() {
                    d[k - lead].to_string()
                } else if den_phs[k] == '?' {
                    " ".into()
                } else {
                    String::new()
                };
            }
        }
    }
    let _ = &mut ip;
    Some(join_range(toks, &out, 0, toks.len()))
}

fn token_width(t: &T) -> usize {
    match t {
        T::Lit(l) => l.chars().count(),
        T::Ph(_) | T::Dot | T::Pct | T::Slash => 1,
        T::Digits(d) => d.len(),
        T::Comma | T::Exp { .. } => 0,
    }
}

/// Marks a literal that is left out.
const DROPPED: &str = "\u{0}";

fn join_range(toks: &[T], out: &[String], from: usize, to: usize) -> String {
    let mut s = String::new();
    for i in from..to {
        match &toks[i] {
            _ if out[i] == DROPPED => {}
            T::Lit(l) if out[i].is_empty() => s.push_str(l),
            T::Dot => s.push('.'),
            T::Pct => s.push('%'),
            T::Slash => s.push('/'),
            _ => s.push_str(&out[i]),
        }
    }
    s
}
