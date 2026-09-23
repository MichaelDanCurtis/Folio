//! Section page-number properties are separate from linked story content.
use crate::{docx_page_setup as pages, office::*};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub fn read(section: Option<roxmltree::Node>) -> Value {
    let n = section.and_then(|s| child(s, "pgNumType"));
    json!({"format":n.and_then(|n|n.attribute((W,"fmt"))).unwrap_or("decimal"),
 "start":n.and_then(|n|n.attribute((W,"start"))).and_then(|v|v.parse::<i64>().ok()),
 "chapterStyle":n.and_then(|n|n.attribute((W,"chapStyle"))).and_then(|v|v.parse::<i64>().ok()),
 "chapterSeparator":n.and_then(|n|n.attribute((W,"chapSep"))).unwrap_or("hyphen")})
}
pub fn patch(section: &str, value: &Value) -> Result<String, String> {
    let d = parse(section)?;
    let root = d.root_element();
    let before = read(Some(root));
    if before == *value {
        return Ok(section.into());
    }
    let format = value["format"]
        .as_str()
        .ok_or("Choose a page number format")?;
    if before["format"] != format
        && ![
            "decimal",
            "upperRoman",
            "lowerRoman",
            "upperLetter",
            "lowerLetter",
            "decimalZero",
            "numberInDash",
            "ordinal",
            "cardinalText",
            "ordinalText",
            "hex",
            "chicago",
            "none",
        ]
        .contains(&format)
    {
        return Err("Unsupported page number format".into());
    }
    let mut set = vec![];
    let mut remove = vec![];
    if before["format"] != format {
        set.push(("fmt", format.to_string()));
    }
    for (key, attribute, max) in [
        ("start", "start", i32::MAX as i64),
        ("chapterStyle", "chapStyle", 9),
    ] {
        if value[key] == before[key] {
            continue;
        }
        if value[key].is_null() {
            remove.push(attribute);
        } else {
            let n = value[key]
                .as_i64()
                .filter(|n| *n >= if key == "start" { 0 } else { 1 } && *n <= max)
                .ok_or(format!("Invalid {key}"))?;
            set.push((attribute, n.to_string()));
        }
    }
    if value["chapterSeparator"] != before["chapterSeparator"]
        || (!value["chapterStyle"].is_null() && value["chapterStyle"] != before["chapterStyle"])
    {
        let sep = value["chapterSeparator"]
            .as_str()
            .filter(|s| ["hyphen", "period", "colon", "emDash", "enDash"].contains(s))
            .ok_or("Choose a chapter separator")?;
        set.push(("chapSep", sep.into()));
    }
    if value["chapterStyle"].is_null() && before["chapterStyle"] != value["chapterStyle"] {
        remove.push("chapSep");
        set.retain(|(k, _)| *k != "chapSep");
    }
    let original = child(root, "pgNumType").map(|n| scoped_record(section, n));
    let leaf = pages::patch_leaf(original.as_deref(), "pgNumType", &set, &remove)?;
    pages::set_child(section, "pgNumType", &leaf)
}
