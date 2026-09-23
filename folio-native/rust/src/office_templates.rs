use crate::office::{parse, replace_ranges, Package};

const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const DOC: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const DOT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml";
const XLS: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const XLT: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml";

pub fn format(path: &str) -> Result<&str, String> {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "docx" | "DOCX" => Ok("docx"),
        "dotx" | "DOTX" => Ok("dotx"),
        "xlsx" | "XLSX" => Ok("xlsx"),
        "xltx" | "XLTX" => Ok("xltx"),
        _ => Err("Choose a DOCX, XLSX, DOTX or XLTX file".into()),
    }
}
pub fn document_format(format: &str) -> Result<&str, String> {
    match format {
        "docx" | "dotx" => Ok("docx"),
        "xlsx" | "xltx" => Ok("xlsx"),
        _ => Err("Unsupported Office format".into()),
    }
}

/// Change only the main-part content type. Unknown relationships and package
/// parts stay byte-for-byte intact. Never relabel macro-bearing content as a
/// macro-free document/template.
pub fn convert(mut package: Package, format: &str) -> Result<Package, String> {
    let (part, regular, template, target) = match format {
        "docx" => ("/word/document.xml", DOC, DOT, DOC),
        "dotx" => ("/word/document.xml", DOC, DOT, DOT),
        "xlsx" => ("/xl/workbook.xml", XLS, XLT, XLS),
        "xltx" => ("/xl/workbook.xml", XLS, XLT, XLT),
        _ => return Err("Unsupported Office format".into()),
    };
    let xml = package.text("[Content_Types].xml")?;
    let dom = parse(xml)?;
    if package
        .entries
        .keys()
        .any(|p| p.to_ascii_lowercase().ends_with("vbaproject.bin"))
        || dom.descendants().flat_map(|n| n.attributes()).any(|a| {
            a.name() == "ContentType" && {
                let value = a.value().to_ascii_lowercase();
                value.contains("macroenabled")
                    || value.contains("vbaproject")
                    || value.contains("macrosheet")
            }
        })
    {
        return Err("Macro-enabled Office templates are not supported yet. The original file was not changed.".into());
    }
    let matches: Vec<_> = dom
        .descendants()
        .filter(|n| n.has_tag_name((CT, "Override")) && n.attribute("PartName") == Some(part))
        .collect();
    if matches.len() != 1 {
        return Err("Missing or ambiguous Office main content type".into());
    }
    let content_type = matches[0]
        .attributes()
        .find(|a| a.name() == "ContentType" && a.namespace().is_none())
        .ok_or("Missing Office content type")?;
    if content_type.value() != regular && content_type.value() != template {
        return Err("The Office package type does not match this document or workbook".into());
    }
    if content_type.value() != target {
        let changed = replace_ranges(xml, vec![(content_type.range_value(), target.into())]);
        package.set("[Content_Types].xml", changed);
        package.original = package.bytes()?;
    }
    Ok(package)
}
