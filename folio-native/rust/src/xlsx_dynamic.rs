//! Dynamic-array formulas in exported worksheets (X09.02), written the way
//! Excel 365 writes them: the anchor is an array formula over the spilled
//! range, marked dynamic by cell metadata (`cm`, the XLDAPR properties in
//! `xl/metadata.xml`), and every spilled cell holds its cached value.
//! Without the metadata Excel opens the formula as a legacy one
//! (`=@SEQUENCE(3,3)`, one value).
use crate::formula::{address, parse_address};
use crate::office::*;
use crate::tables::Area;
use crate::xlsx::{append_root, cached_result, fragment_inner};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const XDA: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray";
const METADATA: &str = "xl/metadata.xml";
const METADATA_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sheetMetadata";
const METADATA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetMetadata+xml";

/// One worksheet's dynamic-array formulas.
#[derive(Default)]
pub(crate) struct Dynamic {
    /// Anchor -> the range it spills into (its own cell when it can't spill).
    anchors: HashMap<String, String>,
    /// The cells formulas spill into, other than their anchors.
    pub spilled: HashSet<String>,
    /// The cell-metadata index (`cm`) that marks a formula dynamic.
    pub cm: String,
}

impl Dynamic {
    /// `spills` is the calculator's anchor -> range map for this sheet.
    pub fn new(sheet: &Value, spills: &Value, calc: &Value, errors: &Value) -> Self {
        let cells = &sheet["cells"];
        let formula = |a: &str| cells[a].as_str().is_some_and(|v| v.starts_with('='));
        let mut out = Self::default();
        for (a, range) in spills.as_object().into_iter().flatten() {
            let Some(area) = range.as_str().and_then(|r| Area::parse(r).ok()) else {
                continue;
            };
            if !formula(a) {
                continue;
            }
            for r in area.top..=area.bottom {
                for c in area.left..=area.right {
                    let at = address(r, c);
                    if at != *a && cells.get(&at).is_none() {
                        out.spilled.insert(at);
                    }
                }
            }
            out.anchors.insert(a.clone(), reference(area));
        }
        // A formula blocked from spilling is still a dynamic-array formula;
        // written as a legacy one, Excel would show its first value.
        for (a, value) in calc.as_object().into_iter().flatten() {
            if errors[a] == true && value == "#SPILL!" && formula(a) {
                out.anchors.entry(a.clone()).or_insert_with(|| a.clone());
            }
        }
        out
    }
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }
    /// The anchor's spilled range and cell-metadata index.
    pub fn formula(&self, a: &str) -> Option<(&str, &str)> {
        self.anchors
            .get(a)
            .map(|range| (range.as_str(), self.cm.as_str()))
    }
}

fn reference(area: Area) -> String {
    if area.top == area.bottom && area.left == area.right {
        address(area.top, area.left)
    } else {
        area.text()
    }
}

/// A spilled cell: its cached value only, as Excel writes it.
pub(crate) fn spilled_xml(
    a: &str,
    style: Option<&str>,
    calculated: &Value,
    is_error: bool,
) -> String {
    let style = style
        .map(|s| format!(" s=\"{}\"", esc(s)))
        .unwrap_or_default();
    let start = format!("<c xmlns=\"{S}\" r=\"{}\"{style}", esc(a));
    match cached_result(calculated, is_error) {
        Some((t, v)) => format!("{start}{t}><v>{v}</v></c>"),
        None => format!("{start}/>"),
    }
}

/// The areas array formulas marked dynamic in `xml` filled when it was
/// saved, except those of formulas Folio keeps without calculating (which
/// keep their cached values). Cells in them that nothing spills into now
/// must lose their old values.
pub(crate) fn released(
    xml: &roxmltree::Document,
    sheet: &Value,
    before: &Value,
    calc: &Value,
    errors: &Value,
    dynamic: &Dynamic,
) -> Vec<Area> {
    xml.descendants()
        .filter(|n| n.has_tag_name((S, "c")) && attr(*n, "cm").is_some())
        .filter_map(|c| {
            let f = child(c, "f").filter(|f| attr(*f, "t") == Some("array"))?;
            let a = attr(c, "r")?;
            let calculated = cached_result(&calc[a], errors[a] == true).is_some();
            if dynamic.formula(a).is_none()
                && !calculated
                && sheet["cells"][a] == before["cells"][a]
            {
                return None;
            }
            Area::parse(attr(f, "ref")?).ok()
        })
        .collect()
}

/// Whether a cell `released` covers.
pub(crate) fn in_released(areas: &[Area], a: &str) -> bool {
    parse_address(a).is_ok_and(|(r, c)| areas.iter().any(|area| area.contains(r, c)))
}

const TYPE: &str = "<metadataType name=\"XLDAPR\" minSupportedVersion=\"120000\" copy=\"1\" pasteAll=\"1\" pasteValues=\"1\" merge=\"1\" splitFirst=\"1\" rowColShift=\"1\" clearFormats=\"1\" clearComments=\"1\" assign=\"1\" coerce=\"1\" cellMeta=\"1\"/>";

fn block() -> String {
    format!("<bk xmlns=\"{S}\"><extLst><ext uri=\"{{bdbb8cdc-fa1e-496e-a857-3c3f30c029c3}}\"><xda:dynamicArrayProperties xmlns:xda=\"{XDA}\" fDynamic=\"1\" fCollapsed=\"0\"/></ext></extLst></bk>")
}

/// The `cm` index of the dynamic-array cell metadata, adding the metadata
/// part (with its relationship and content type) or the missing entries to
/// an existing one.
pub(crate) fn metadata_index(p: &mut Package) -> Result<String, String> {
    if !p.entries.contains_key(METADATA) {
        p.set(
            METADATA,
            format!(
                "{XML_HEAD}<metadata xmlns=\"{S}\"><metadataTypes count=\"1\">{TYPE}</metadataTypes><futureMetadata name=\"XLDAPR\" count=\"1\">{}</futureMetadata><cellMetadata count=\"1\"><bk><rc t=\"1\" v=\"0\"/></bk></cellMetadata></metadata>",
                block()
            ),
        );
        link(p)?;
        return Ok("1".into());
    }
    let xml = p.text(METADATA)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let types_node = root.children().find(|n| named(*n, "metadataTypes"));
    let types = list(types_node, "metadataType");
    let future_node = root
        .children()
        .find(|n| named(*n, "futureMetadata") && attr(*n, "name") == Some("XLDAPR"));
    let blocks = list(future_node, "bk");
    let cells_node = root.children().find(|n| named(*n, "cellMetadata"));
    let cell_blocks = list(cells_node, "bk");

    let type_index = types
        .iter()
        .position(|n| attr(*n, "name") == Some("XLDAPR"))
        .map(|i| i + 1);
    let block_index = blocks.iter().position(|bk| {
        bk.descendants().any(|n| {
            n.has_tag_name((XDA, "dynamicArrayProperties"))
                && matches!(attr(n, "fDynamic"), Some("1" | "true"))
        })
    });
    if let (Some(t), Some(v)) = (type_index, block_index) {
        let (t, v) = (t.to_string(), v.to_string());
        if let Some(i) = cell_blocks.iter().position(|bk| {
            bk.children()
                .any(|rc| named(rc, "rc") && attr(rc, "t") == Some(&t) && attr(rc, "v") == Some(&v))
        }) {
            return Ok((i + 1).to_string());
        }
    }

    // Add what is missing, keeping the schema's element order.
    let t = type_index.unwrap_or(types.len() + 1);
    let v = block_index.unwrap_or(blocks.len());
    let cm = cell_blocks.len() + 1;
    let grown = |node: roxmltree::Node, count: usize, extra: &str| -> Result<String, String> {
        let inner = node.children().map(|n| raw(&xml, n)).collect::<String>();
        let opened =
            crate::xlsx_tables::attributes(&xml, node, &[("count", Some(count.to_string()))]);
        fragment_inner(&opened, &(inner + extra))
    };
    let new_types = (type_index.is_none() && types_node.is_none())
        .then(|| format!("<metadataTypes xmlns=\"{S}\" count=\"1\">{TYPE}</metadataTypes>"));
    let new_future = (block_index.is_none() && future_node.is_none()).then(|| {
        format!(
            "<futureMetadata xmlns=\"{S}\" name=\"XLDAPR\" count=\"1\">{}</futureMetadata>",
            block()
        )
    });
    let cell = format!("<bk xmlns=\"{S}\"><rc t=\"{t}\" v=\"{v}\"/></bk>");
    let new_cells = cells_node
        .is_none()
        .then(|| format!("<cellMetadata xmlns=\"{S}\" count=\"1\">{cell}</cellMetadata>"));
    let mut content = new_types.unwrap_or_default();
    let mut future_pending = new_future;
    let mut cells_pending = new_cells;
    for n in root.children() {
        if named(n, "cellMetadata") || named(n, "valueMetadata") || named(n, "extLst") {
            content += &future_pending.take().unwrap_or_default();
        }
        if named(n, "valueMetadata") || named(n, "extLst") {
            content += &cells_pending.take().unwrap_or_default();
        }
        content += &if Some(n) == types_node && type_index.is_none() {
            grown(
                n,
                types.len() + 1,
                &TYPE.replace("<metadataType ", &format!("<metadataType xmlns=\"{S}\" ")),
            )?
        } else if Some(n) == future_node && block_index.is_none() {
            grown(n, blocks.len() + 1, &block())?
        } else if Some(n) == cells_node {
            grown(n, cm, &cell)?
        } else {
            raw(&xml, n).to_string()
        };
    }
    content += &future_pending.unwrap_or_default();
    content += &cells_pending.unwrap_or_default();
    let next = replace_ranges(
        &xml,
        vec![(root.range(), fragment_inner(raw(&xml, root), &content)?)],
    );
    p.set(METADATA, next);
    link(p)?;
    Ok(cm.to_string())
}

fn named(n: roxmltree::Node, name: &str) -> bool {
    n.is_element() && n.tag_name().name() == name
}
fn list<'a, 'i>(
    parent: Option<roxmltree::Node<'a, 'i>>,
    name: &str,
) -> Vec<roxmltree::Node<'a, 'i>> {
    parent
        .into_iter()
        .flat_map(|p| p.children())
        .filter(|n| named(*n, name))
        .collect()
}

/// The workbook relationship and content type of the metadata part.
fn link(p: &mut Package) -> Result<(), String> {
    let rels_path = "xl/_rels/workbook.xml.rels";
    let rels = p.text(rels_path)?.to_string();
    let d = parse(&rels)?;
    if !d
        .descendants()
        .any(|n| attr(n, "Type") == Some(METADATA_REL))
    {
        let used: HashSet<_> = d.descendants().filter_map(|n| attr(n, "Id")).collect();
        let id = (1..)
            .map(|n| format!("folioMetadata{n}"))
            .find(|id| !used.contains(id.as_str()))
            .unwrap();
        append_root(
            p,
            rels_path,
            &format!("<Relationship xmlns=\"{REL_NS}\" Id=\"{id}\" Type=\"{METADATA_REL}\" Target=\"metadata.xml\"/>"),
        )?;
    }
    let types = p.text("[Content_Types].xml")?.to_string();
    let d = parse(&types)?;
    if !d
        .descendants()
        .any(|n| attr(n, "PartName") == Some("/xl/metadata.xml"))
    {
        append_root(
            p,
            "[Content_Types].xml",
            &format!("<Override xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\" PartName=\"/xl/metadata.xml\" ContentType=\"{METADATA_TYPE}\"/>"),
        )?;
    }
    Ok(())
}
