//! Tolerant XLSX reading (X17.12): normalizes what Excel accepts but the
//! rest of the reader assumes, before import.
use crate::formula::address;
use crate::office::{parse, replace_ranges, Package};

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Adds the `r` addresses OOXML lets writers leave out: a row without one
/// follows the previous row, and a cell without one follows the previous
/// cell in its row.
pub fn with_cell_addresses(mut pkg: Package) -> Result<Package, String> {
    let parts: Vec<String> = pkg
        .entries
        .keys()
        .filter(|name| name.starts_with("xl/worksheets/") && name.ends_with(".xml"))
        .cloned()
        .collect();
    for name in parts {
        let Ok(xml) = pkg.text(&name) else { continue };
        if !xml.contains("<c") && !xml.contains(":c") {
            continue;
        }
        let doc = parse(xml)?;
        let mut edits = vec![];
        let mut row_index = 0usize;
        for row in doc.descendants().filter(|n| n.has_tag_name((S, "row"))) {
            row_index = match row.attribute("r").and_then(|r| r.parse::<usize>().ok()) {
                Some(r) if r > 0 => r,
                _ => {
                    edits.push((after_tag(xml, row), format!(" r=\"{}\"", row_index + 1)));
                    row_index + 1
                }
            };
            let mut column = 0usize;
            for cell in row.children().filter(|n| n.has_tag_name((S, "c"))) {
                column = match cell
                    .attribute("r")
                    .and_then(|a| crate::formula::parse_address(a).ok())
                {
                    Some((_, c)) => c + 1,
                    None => {
                        edits.push((
                            after_tag(xml, cell),
                            format!(" r=\"{}\"", address(row_index - 1, column)),
                        ));
                        column + 1
                    }
                };
            }
        }
        if !edits.is_empty() {
            let updated = replace_ranges(
                xml,
                edits.into_iter().map(|(at, text)| (at..at, text)).collect(),
            );
            pkg.set(&name, updated);
        }
    }
    Ok(pkg)
}

/// Strict Open XML (and Excel 2007 beta) workbooks, read as Transitional:
/// `http://purl.oclc.org/ooxml/{a}/{b}` names the same thing as
/// `http://schemas.openxmlformats.org/{a}/2006/{b}`. Returns the package and
/// whether anything was converted; the file is saved as Transitional.
pub fn as_transitional(mut pkg: Package) -> (Package, bool) {
    const STRICT: &str = "http://purl.oclc.org/ooxml/";
    const BETA: &str = "http://schemas.microsoft.com/office/excel/2006/2";
    let mut converted = false;
    let names: Vec<String> = pkg.entries.keys().cloned().collect();
    for name in names {
        if !(name.ends_with(".xml") || name.ends_with(".rels")) {
            continue;
        }
        let Ok(xml) = pkg.text(&name) else { continue };
        if !xml.contains(STRICT) && !xml.contains(BETA) {
            continue;
        }
        let mut out = String::with_capacity(xml.len());
        let mut rest = xml;
        while let Some(at) = rest.find(STRICT) {
            out.push_str(&rest[..at]);
            let tail = &rest[at + STRICT.len()..];
            // `{a}/{b}`, up to the closing quote.
            let end = tail.find(['"', '\'']).unwrap_or(tail.len());
            let path = &tail[..end];
            match path.split_once('/') {
                Some((a, b)) => {
                    out.push_str(&format!("http://schemas.openxmlformats.org/{a}/2006/{b}"))
                }
                None => out.push_str(&rest[at..at + STRICT.len() + end]),
            }
            rest = &tail[end..];
        }
        out.push_str(rest);
        let out = out
            .replace(
                BETA,
                "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
            )
            .replace(" conformance=\"strict\"", "");
        pkg.set(&name, out);
        converted = true;
    }
    // Saving an unchanged workbook writes the Transitional form too.
    if converted {
        if let Ok(bytes) = pkg.bytes() {
            pkg.original = bytes;
        }
    }
    (pkg, converted)
}

/// The position just after an element's tag name, where an attribute can go.
fn after_tag(xml: &str, node: roxmltree::Node) -> usize {
    let start = node.range().start;
    let tag = &xml[start..];
    start
        + tag
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_namespaces_read_as_transitional() {
        let mut pkg = Package::default();
        pkg.set(
            "xl/workbook.xml",
            r#"<workbook xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main" xmlns:r="http://purl.oclc.org/ooxml/officeDocument/relationships" conformance="strict"/>"#.into(),
        );
        pkg.set(
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://purl.oclc.org/ooxml/officeDocument/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.into(),
        );
        let (pkg, converted) = as_transitional(pkg);
        assert!(converted);
        assert_eq!(
            pkg.text("xl/workbook.xml").unwrap(),
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>"#
        );
        assert!(pkg
            .text("xl/_rels/workbook.xml.rels")
            .unwrap()
            .contains(r#"Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet""#));
    }

    #[test]
    fn missing_addresses_follow_their_neighbours() {
        let mut pkg = Package::default();
        pkg.set(
            "xl/worksheets/sheet1.xml",
            concat!(
                r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
                r#"<row><c t="inlineStr"><is><t>a</t></is></c><c><v>1</v></c></row>"#,
                r#"<row r="4"><c r="C4"><v>2</v></c><c/></row>"#,
                r#"<row><c><v>3</v></c></row>"#,
                r#"</sheetData></worksheet>"#
            )
            .to_string(),
        );
        let pkg = with_cell_addresses(pkg).unwrap();
        let xml = pkg.text("xl/worksheets/sheet1.xml").unwrap();
        for expected in [
            r#"<row r="1"><c r="A1" t="inlineStr">"#,
            r#"<c r="B1"><v>1</v>"#,
            r#"<c r="D4"/>"#,
            r#"<row r="5"><c r="A5"><v>3</v>"#,
        ] {
            assert!(xml.contains(expected), "{expected}: {xml}");
        }
    }
}
