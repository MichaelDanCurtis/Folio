pub const ROWS: usize = 1_048_576;
pub const COLS: usize = 16_384;
pub fn address(row: usize, col: usize) -> String {
    let mut column = col + 1;
    let mut letters = Vec::new();
    while column > 0 {
        column -= 1;
        letters.push((b'A' + (column % 26) as u8) as char);
        column /= 26;
    }
    format!(
        "{}{}",
        letters.into_iter().rev().collect::<String>(),
        row + 1
    )
}
pub fn parse_address(a: &str) -> Result<(usize, usize), String> {
    let n = a.bytes().take_while(|b| b.is_ascii_uppercase()).count();
    if n == 0 || n > 3 || n == a.len() {
        return Err("#REF!".into());
    }
    let col = a[..n]
        .bytes()
        .fold(0usize, |c, b| c * 26 + (b - b'A' + 1) as usize);
    if !a[n..].bytes().all(|b| b.is_ascii_digit()) || a[n..].starts_with('0') {
        return Err("#REF!".into());
    }
    let row = a[n..].parse::<usize>().map_err(|_| "#REF!")?;
    if row == 0 || row > ROWS || col > COLS {
        return Err("#REF!".into());
    }
    Ok((row - 1, col - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excel_bounds() {
        assert_eq!(address(1048575, 16383), "XFD1048576");
        assert_eq!(parse_address("AA10").unwrap(), (9, 26));
        assert!(parse_address("XFE1").is_err());
        assert!(parse_address("A0").is_err());
    }
}
