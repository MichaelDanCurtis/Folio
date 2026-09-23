//! SpreadsheetML ST_Xstring is distinct from XML escaping. Decode exactly
//! once; a protected underscore must not start a second decoding pass.

fn unit(bytes: &[u8]) -> Option<u16> {
    if bytes.len() < 7 || bytes[0] != b'_' || bytes[1] != b'x' || bytes[6] != b'_' {
        return None;
    }
    let mut value = 0;
    for b in &bytes[2..6] {
        value = value * 16 + (*b as char).to_digit(16)? as u16;
    }
    Some(value)
}

pub fn decode(text: &str) -> String {
    decode_positions(text, |_, _| {})
}

/// Positions that retain an unambiguous character boundary when upgrading an
/// old workspace whose editor displayed the XML escape tokens themselves.
pub fn boundaries(text: &str) -> std::collections::BTreeMap<usize, usize> {
    let mut result = std::collections::BTreeMap::new();
    decode_positions(text, |old, new| {
        result.insert(old, new);
    });
    result
}

fn decode_positions(text: &str, mut position: impl FnMut(usize, usize)) -> String {
    let mut result = String::with_capacity(text.len());
    let mut at = 0;
    let (mut old_units, mut new_units) = (0, 0);
    position(0, 0);
    while at < text.len() {
        if let Some(high) = unit(&text.as_bytes()[at..]) {
            if (0xd800..=0xdbff).contains(&high) {
                if let Some(low) =
                    unit(&text.as_bytes()[at + 7..]).filter(|low| (0xdc00..=0xdfff).contains(low))
                {
                    let scalar = 0x10000 + (((high as u32 - 0xd800) << 10) | (low as u32 - 0xdc00));
                    result.push(char::from_u32(scalar).unwrap());
                    at += 14;
                    old_units += 14;
                    new_units += 2;
                    position(old_units, new_units);
                    continue;
                }
            }
            if let Some(c) = char::from_u32(high as u32) {
                result.push(c);
                new_units += c.len_utf16();
            } else {
                // Rust and the UI require valid Unicode. Retain malformed
                // surrogate tokens verbatim instead of losing their bytes.
                result.push_str(&text[at..at + 7]);
                // An unpaired surrogate remains seven literal characters.
                for delta in 1..7 {
                    position(old_units + delta, new_units + delta);
                }
                new_units += 7;
            }
            at += 7;
            old_units += 7;
        } else {
            let c = text[at..].chars().next().unwrap();
            result.push(c);
            at += c.len_utf8();
            old_units += c.len_utf16();
            new_units += c.len_utf16();
        }
        position(old_units, new_units);
    }
    result
}

/// Encode logical text, then escape XML delimiters. Cell text keeps LF and TAB;
/// CR and XML-prohibited code points use ST_Xstring tokens. Attribute callers
/// additionally protect LF/TAB against XML whitespace normalization.
pub fn xml(text: &str, attribute: bool) -> String {
    let mut encoded = String::with_capacity(text.len());
    for (at, c) in text.char_indices() {
        if c == '_' && unit(&text.as_bytes()[at..]).is_some() {
            encoded.push_str("_x005F_");
        } else if c == '\r'
            || (c < ' ' && c != '\n' && c != '\t')
            || matches!(c, '\u{fffe}' | '\u{ffff}')
            || (attribute && matches!(c, '\n' | '\t'))
        {
            encoded.push_str(&format!("_x{:04X}_", c as u32));
        } else {
            encoded.push(c);
        }
    }
    crate::office::esc(&encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_decode_once_and_surrogate_pairs_become_one_scalar() {
        assert_eq!(decode("A_x000D_B_x0009_C_x000A_D"), "A\rB\tC\nD");
        assert_eq!(decode("_x005F_x0041_"), "_x0041_");
        assert_eq!(decode("_x005f__x0041_"), "_A");
        assert_eq!(decode("_xD83C__xDF0A_"), "🌊");
        assert_eq!(decode("_x0041__x0062__x00e9_"), "Abé");
        assert_eq!(decode("_X0041_ _xZZZZ_ _x000_"), "_X0041_ _xZZZZ_ _x000_");
        assert_eq!(decode("_xD800_X_xDC00_"), "_xD800_X_xDC00_");
    }
    #[test]
    fn xml_roundtrips_literal_tokens_controls_unicode_and_attribute_whitespace() {
        for attribute in [false, true] {
            for text in [
                "a\0b\u{1}c\r\n\t",
                "_x0041_ _x005F_x0042_",
                "&<>\"'",
                "🌊é\u{fffe}\u{ffff}",
                "_X000D_",
                "_xD800_",
            ] {
                let encoded = xml(text, attribute);
                let fragment = if attribute {
                    format!("<t v=\"{encoded}\"/>")
                } else {
                    format!("<t>{encoded}</t>")
                };
                let doc = crate::office::parse(&fragment).unwrap();
                let stored = if attribute {
                    doc.root_element().attribute("v").unwrap()
                } else {
                    doc.root_element().text().unwrap()
                };
                assert_eq!(decode(stored), text, "{fragment}");
            }
        }
        assert_eq!(xml("_x0041_", false), "_x005F_x0041_");
        assert_eq!(xml("\r\n\t", false), "_x000D_\n\t");
        assert_eq!(xml("\r\n\t", true), "_x000D__x000A__x0009_");
    }
}
