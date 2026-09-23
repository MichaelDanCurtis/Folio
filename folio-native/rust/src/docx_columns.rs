//! Section columns, retaining source attributes and unrelated section content.
use crate::{docx_page_setup as pages, office::*};
use roxmltree::Node;
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn flag(n: Option<Node>, name: &str, fallback: bool) -> bool {
    n.and_then(|n| n.attribute((W, name)))
        .map(|v| !matches!(v, "0" | "false" | "off"))
        .unwrap_or(fallback)
}
pub fn read(section: Option<Node>) -> Value {
    let cols = section.and_then(|n| child(n, "cols"));
    let equal = flag(cols, "equalWidth", true);
    let definitions = cols.map(|n| n.children().filter(|n| n.has_tag_name((W, "col")))
        .map(|n| json!({"width":pages::value(Some(n),"w",0),"space":pages::value(Some(n),"space",0)}))
        .collect::<Vec<_>>()).unwrap_or_default();
    let count = if equal {
        pages::value(cols, "num", 1)
    } else {
        definitions.len().max(1) as i64
    };
    json!({"count":count,"equalWidth":equal,"space":pages::value(cols,"space",720),
        "separator":flag(cols,"sep",false),"rtl":section.and_then(|n|child(n,"bidi")).is_some_and(|n|flag(Some(n),"val",true)),
        "columns":if equal {vec![]} else {definitions}})
}
pub fn validate(columns: &Value, geometry: &Value, gutter_top: bool) -> Result<(), String> {
    let object = columns.as_object().ok_or("Column settings are missing")?;
    if object.keys().any(|k| {
        ![
            "count",
            "equalWidth",
            "space",
            "separator",
            "rtl",
            "columns",
        ]
        .contains(&k.as_str())
    }) {
        return Err("Choose supported column settings".into());
    }
    let count = columns["count"]
        .as_i64()
        .filter(|n| (1..=45).contains(n))
        .ok_or("Choose between 1 and 45 columns")?;
    for key in ["equalWidth", "separator", "rtl"] {
        if !columns[key].is_boolean() {
            return Err(format!("Choose a valid {key} setting"));
        }
    }
    let measurement = |value: &Value, minimum| {
        value
            .as_i64()
            .filter(|n| (minimum..=31680).contains(n))
            .ok_or_else(|| "Column widths and spacing must be valid whole measurements".to_string())
    };
    let space = measurement(&columns["space"], 0)?;
    let definitions = columns["columns"]
        .as_array()
        .ok_or("Column width definitions are missing")?;
    let available = geometry["width"].as_i64().unwrap_or(0)
        - geometry["left"].as_i64().unwrap_or(0)
        - geometry["right"].as_i64().unwrap_or(0)
        - if gutter_top {
            0
        } else {
            geometry["gutter"].as_i64().unwrap_or(0)
        };
    let needed = if columns["equalWidth"] == true {
        if !definitions.is_empty() {
            return Err("Equal-width columns do not need individual widths".into());
        }
        (count - 1) * space + count
    } else {
        if definitions.len() != count as usize {
            return Err("Enter one width for each column".into());
        }
        let mut total = 0;
        for (i, definition) in definitions.iter().enumerate() {
            if definition
                .as_object()
                .is_none_or(|o| o.keys().any(|k| !["width", "space"].contains(&k.as_str())))
            {
                return Err("Choose supported column measurements".into());
            }
            total += measurement(&definition["width"], 1)?;
            let gap = measurement(&definition["space"], 0)?;
            if i + 1 < definitions.len() {
                total += gap;
            }
        }
        total
    };
    if needed > available {
        return Err("Column widths and spacing must fit between the page margins".into());
    }
    Ok(())
}
pub fn patch(section: &str, settings: &Value) -> Result<String, String> {
    let d = parse(section)?;
    let root = d.root_element();
    let source = child(root, "cols").map(|n| scoped_record(section, n));
    let mut cols = pages::patch_leaf(
        source.as_deref(),
        "cols",
        &[
            ("num", settings["count"].to_string()),
            (
                "equalWidth",
                if settings["equalWidth"] == true {
                    "1"
                } else {
                    "0"
                }
                .into(),
            ),
            ("space", settings["space"].to_string()),
            (
                "sep",
                if settings["separator"] == true {
                    "1"
                } else {
                    "0"
                }
                .into(),
            ),
        ],
        &[],
    )?;
    if settings["equalWidth"] == false {
        let d = parse(&cols)?;
        let root = d.root_element();
        let old = root
            .children()
            .filter(|n| n.has_tag_name((W, "col")))
            .collect::<Vec<_>>();
        let mut children = String::new();
        for (i, definition) in settings["columns"].as_array().unwrap().iter().enumerate() {
            let original = old.get(i).map(|n| scoped_record(&cols, *n));
            children.push_str(&pages::patch_leaf(
                original.as_deref(),
                "col",
                &[
                    ("w", definition["width"].to_string()),
                    ("space", definition["space"].to_string()),
                ],
                &[],
            )?);
        }
        for n in root.children().filter(|n| !n.has_tag_name((W, "col"))) {
            children.push_str(raw(&cols, n));
        }
        cols = replace_inner(&cols, root, &children);
    }
    let out = pages::set_child(section, "cols", &cols)?;
    let bidi = child(root, "bidi").map(|n| scoped_record(section, n));
    let bidi = pages::patch_leaf(
        bidi.as_deref(),
        "bidi",
        &[(
            "val",
            if settings["rtl"] == true { "1" } else { "0" }.into(),
        )],
        &[],
    )?;
    pages::set_child(&out, "bidi", &bidi)
}
