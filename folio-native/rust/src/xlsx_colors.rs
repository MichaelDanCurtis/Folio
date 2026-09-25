//! SpreadsheetML colours as `#RRGGBB`: `rgb`, `indexed` (the workbook's
//! palette or Excel's default one) and `theme` with `tint`, resolved against
//! the workbook theme.
use crate::office::{parse, Package};

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Excel's default indexed palette (entries 0–63).
const DEFAULT_PALETTE: [&str; 64] = [
    "000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "000000",
    "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "800000", "008000",
    "000080", "808000", "800080", "008080", "C0C0C0", "808080", "9999FF", "993366", "FFFFCC",
    "CCFFFF", "660066", "FF8080", "0066CC", "CCCCFF", "000080", "FF00FF", "FFFF00", "00FFFF",
    "800080", "800000", "008080", "0000FF", "00CCFF", "CCFFFF", "CCFFCC", "FFFF99", "99CCFF",
    "FF99CC", "CC99FF", "FFCC99", "3366FF", "33CCCC", "99CC00", "FFCC00", "FF9900", "FF6600",
    "666699", "969696", "003366", "339966", "003300", "333300", "993300", "993366", "333399",
    "333333",
];

/// The colours a workbook's cell formats can refer to.
#[derive(Default)]
pub struct Colors {
    palette: Vec<String>,
    /// Theme colours in SpreadsheetML index order: lt1, dk1, lt2, dk2,
    /// accent1–6, hlink, folHlink.
    theme: Vec<String>,
}

impl Colors {
    pub fn read(pkg: &Package) -> Self {
        let palette = pkg
            .text("xl/styles.xml")
            .ok()
            .and_then(|xml| parse(xml).ok())
            .and_then(|d| {
                d.descendants()
                    .find(|n| n.has_tag_name((S, "indexedColors")))
                    .map(|n| {
                        n.children()
                            .filter(|n| n.has_tag_name((S, "rgbColor")))
                            .filter_map(|n| n.attribute("rgb").and_then(hex))
                            .collect()
                    })
            })
            .unwrap_or_default();
        let theme = theme_path(pkg)
            .and_then(|path| pkg.text(&path).ok())
            .and_then(|xml| parse(xml).ok())
            .map(|d| {
                let Some(scheme) = d.descendants().find(|n| n.has_tag_name((A, "clrScheme")))
                else {
                    return vec![];
                };
                let colour = |name: &str| {
                    scheme
                        .children()
                        .find(|n| n.has_tag_name((A, name)))
                        .and_then(|n| n.children().find(|n| n.is_element()))
                        .and_then(|n| {
                            if n.has_tag_name((A, "sysClr")) {
                                n.attribute("lastClr")
                            } else {
                                n.attribute("val")
                            }
                        })
                        .and_then(hex)
                        .unwrap_or_else(|| "000000".into())
                };
                // Excel swaps the first two pairs relative to the scheme.
                [
                    "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4",
                    "accent5", "accent6", "hlink", "folHlink",
                ]
                .iter()
                .map(|name| colour(name))
                .collect()
            })
            .unwrap_or_default();
        Self { palette, theme }
    }

    /// A `<color>`-like element's colour, or None for automatic, missing or
    /// unknown colours.
    pub fn resolve(&self, node: roxmltree::Node) -> Option<String> {
        if matches!(node.attribute("auto"), Some("1" | "true")) {
            return None;
        }
        let base = if let Some(rgb) = node.attribute("rgb") {
            hex(rgb)?
        } else if let Some(i) = node
            .attribute("indexed")
            .and_then(|v| v.parse::<usize>().ok())
        {
            if let Some(c) = self.palette.get(i) {
                c.clone()
            } else {
                // 64 and 65 are the system foreground and background.
                DEFAULT_PALETTE.get(i)?.to_string()
            }
        } else if let Some(i) = node
            .attribute("theme")
            .and_then(|v| v.parse::<usize>().ok())
        {
            self.theme.get(i)?.clone()
        } else {
            return None;
        };
        let tint = node
            .attribute("tint")
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|t| t.is_finite() && *t != 0.0)
            .map(|t| t.clamp(-1.0, 1.0));
        Some(format!(
            "#{}",
            match tint {
                Some(t) => apply_tint(&base, t),
                None => base,
            }
        ))
    }
}

fn theme_path(pkg: &Package) -> Option<String> {
    let rels = parse(pkg.text("xl/_rels/workbook.xml.rels").ok()?).ok()?;
    let target = rels
        .descendants()
        .find(|n| {
            n.attribute("Type")
                .is_some_and(|t| t.ends_with("/relationships/theme"))
        })?
        .attribute("Target")?;
    Some(if let Some(absolute) = target.strip_prefix('/') {
        absolute.to_string()
    } else {
        crate::xlsx::resolve(target).ok()?
    })
}

/// `AARRGGBB` or `RRGGBB` as upper-case `RRGGBB`.
fn hex(value: &str) -> Option<String> {
    let v = match value.len() {
        8 => &value[2..],
        6 => value,
        _ => return None,
    };
    v.bytes()
        .all(|b| b.is_ascii_hexdigit())
        .then(|| v.to_ascii_uppercase())
}

/// Excel's tint: lighten (positive) or darken (negative) the luminance in
/// HLS space.
fn apply_tint(rgb: &str, tint: f64) -> String {
    let channel = |i: usize| u8::from_str_radix(&rgb[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
    let (r, g, b) = (channel(0), channel(2), channel(4));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if max == min {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if max == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h / 6.0, s)
    };
    let l = if tint < 0.0 {
        l * (1.0 + tint)
    } else {
        l * (1.0 - tint) + tint
    };
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let hue = |t: f64| {
            let t = t.rem_euclid(1.0);
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        (hue(h + 1.0 / 3.0), hue(h), hue(h - 1.0 / 3.0))
    };
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("{:02X}{:02X}{:02X}", byte(r), byte(g), byte(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tints_match_excel() {
        // Excel 2007 palette swatches: Accent 1 (#4F81BD) Darker 25%,
        // Accent 2 (#C0504D) Lighter 60%, White Darker 15%, Black Lighter
        // 50%.
        assert_eq!(apply_tint("4F81BD", -0.249977111117893), "376092");
        assert_eq!(apply_tint("C0504D", 0.3999755851924192), "D99694");
        assert_eq!(apply_tint("FFFFFF", -0.1499984740745262), "D9D9D9");
        assert_eq!(apply_tint("000000", 0.499984740745262), "7F7F7F");
    }
}
