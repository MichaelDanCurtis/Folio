//! The file compatibility contract (ROADMAP §2; W16.09, X17.09, D02.17): one
//! table of features with their state for each capability, what the open
//! file contains, and what exporting it will lose or invalidate.
use crate::office::{parse, Package};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Verified,
    Partial,
    Preserved,
    Unsupported,
    NotApplicable,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Verified => "Verified",
            State::Partial => "Partial",
            State::Preserved => "Preserved only",
            State::Unsupported => "Unsupported",
            State::NotApplicable => "—",
        }
    }
}

use State::{NotApplicable as NA, Partial as P, Preserved as K, Unsupported as U, Verified as V};

pub struct Feature {
    pub id: &'static str,
    pub format: &'static str,
    pub name: &'static str,
    /// Open, display, edit, calculate, save, round trip, print, automation.
    pub states: [State; 8],
    /// What exporting an edited file does with it.
    pub on_save: &'static str,
    #[cfg_attr(not(test), allow(dead_code))]
    pub evidence: &'static str,
    count: fn(&Package) -> usize,
}

pub const CAPABILITIES: [&str; 8] = [
    "Open",
    "Display",
    "Edit",
    "Calculate",
    "Save",
    "Round trip",
    "Print",
    "Automation",
];

/// Elements named `local` in parts whose names start with `prefix`.
fn elements(pkg: &Package, prefix: &str, local: &str) -> usize {
    elements_where(pkg, prefix, local, |_| true)
}

fn elements_where(
    pkg: &Package,
    prefix: &str,
    local: &str,
    keep: fn(roxmltree::Node) -> bool,
) -> usize {
    pkg.entries
        .keys()
        .filter(|k| k.starts_with(prefix) && k.ends_with(".xml"))
        .filter_map(|k| pkg.text(k).ok())
        .filter(|xml| xml.contains(local))
        .filter_map(|xml| {
            parse(xml).ok().map(|d| {
                d.descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == local && keep(*n))
                    .count()
            })
        })
        .sum()
}

fn parts(pkg: &Package, keep: impl Fn(&str) -> bool) -> usize {
    pkg.entries.keys().filter(|k| keep(k)).count()
}

fn attr<'a>(n: roxmltree::Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local)
        .map(|a| a.value())
}

/// Parts Folio knows, by prefix; anything else is an unknown extension part.
const KNOWN_PARTS: [&str; 46] = [
    "[Content_Types].xml",
    "_rels/",
    "docProps/",
    "customXml/",
    "_xmlsignatures/",
    "word/_rels/",
    "word/document.xml",
    "word/styles.xml",
    "word/stylesWithEffects.xml",
    "word/settings.xml",
    "word/webSettings.xml",
    "word/fontTable.xml",
    "word/theme/",
    "word/numbering.xml",
    "word/media/",
    "word/header",
    "word/footer",
    "word/footnotes.xml",
    "word/endnotes.xml",
    "word/comments",
    "word/people.xml",
    "word/charts/",
    "word/diagrams/",
    "word/embeddings/",
    "word/glossary/",
    "word/vbaProject.bin",
    "word/vbaData.xml",
    "word/fonts/",
    "word/customizations.xml",
    "xl/_rels/",
    "xl/workbook.xml",
    "xl/styles.xml",
    "xl/sharedStrings.xml",
    "xl/theme/",
    "xl/worksheets/",
    "xl/tables/",
    "xl/comments",
    "xl/threadedComments/",
    "xl/persons/",
    "xl/drawings/",
    "xl/charts/",
    "xl/media/",
    "xl/pivot",
    "xl/calcChain.xml",
    "xl/printerSettings/",
    "xl/vbaProject.bin",
];
const KNOWN_XLSX_EXTRA: [&str; 9] = [
    "xl/externalLinks/",
    "xl/connections.xml",
    "xl/queryTables/",
    "xl/slicers/",
    "xl/slicerCaches/",
    "xl/metadata.xml",
    "xl/richData/",
    "xl/embeddings/",
    "xl/ctrlProps/",
];

fn unknown_part(name: &str) -> bool {
    !name.ends_with('/')
        && !KNOWN_PARTS
            .iter()
            .chain(KNOWN_XLSX_EXTRA.iter())
            .any(|p| name.starts_with(p))
}

pub fn features() -> Vec<Feature> {
    vec![
        // ── Word ──────────────────────────────────────────────────────────
        Feature {
            id: "word.text",
            format: "docx",
            name: "Text and character formatting",
            states: [V, V, V, NA, V, V, U, P],
            on_save: "Written from Folio's edits; formatting Folio doesn't edit is lost in edited paragraphs",
            evidence: "W01, W16.01; corpus round trip",
            count: |p| elements(p, "word/document.xml", "p"),
        },
        Feature {
            id: "word.styles",
            format: "docx",
            name: "Styles",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W02",
            count: |p| elements(p, "word/styles.xml", "style"),
        },
        Feature {
            id: "word.lists",
            format: "docx",
            name: "Numbered and bulleted lists",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W03",
            count: |p| elements(p, "word/numbering.xml", "num"),
        },
        Feature {
            id: "word.tables",
            format: "docx",
            name: "Tables",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W07",
            count: |p| elements(p, "word/document.xml", "tbl"),
        },
        Feature {
            id: "word.pictures",
            format: "docx",
            name: "Inline pictures",
            states: [V, V, P, NA, V, V, U, NA],
            on_save: "Kept; new pictures added",
            evidence: "W08.01–W08.02; corpus pictures",
            count: |p| elements(p, "word/document.xml", "inline"),
        },
        Feature {
            id: "word.floating",
            format: "docx",
            name: "Floating pictures and shapes",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W08.04, W08.08",
            count: |p| elements(p, "word/document.xml", "anchor"),
        },
        Feature {
            id: "word.fields",
            format: "docx",
            name: "Fields",
            states: [V, P, P, NA, K, V, U, NA],
            on_save: "Page fields written; other fields kept unchanged",
            evidence: "W10",
            count: |p| {
                elements(p, "word/document.xml", "fldSimple")
                    + elements_where(p, "word/document.xml", "fldChar", |n| {
                        attr(n, "fldCharType") == Some("begin")
                    })
            },
        },
        Feature {
            id: "word.comments",
            format: "docx",
            name: "Comments",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W13.01",
            count: |p| elements(p, "word/comments.xml", "comment"),
        },
        Feature {
            id: "word.revisions",
            format: "docx",
            name: "Tracked changes",
            states: [V, V, P, NA, V, V, U, NA],
            on_save: "Kept; Folio's tracked edits recorded",
            evidence: "W13.03–W13.06",
            count: |p| {
                ["ins", "del", "moveFrom", "moveTo"]
                    .iter()
                    .map(|t| elements(p, "word/document.xml", t))
                    .sum()
            },
        },
        Feature {
            id: "word.headers",
            format: "docx",
            name: "Headers and footers",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W06",
            count: |p| {
                parts(p, |k| {
                    (k.starts_with("word/header") || k.starts_with("word/footer"))
                        && k.ends_with(".xml")
                })
            },
        },
        Feature {
            id: "word.notes",
            format: "docx",
            name: "Footnotes and endnotes",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W11.01",
            count: |p| {
                let note = |n: roxmltree::Node| {
                    attr(n, "type").is_none_or(|t| t == "normal")
                };
                elements_where(p, "word/footnotes.xml", "footnote", note)
                    + elements_where(p, "word/endnotes.xml", "endnote", note)
            },
        },
        Feature {
            id: "word.equations",
            format: "docx",
            name: "Equations",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W09.01",
            count: |p| elements(p, "word/document.xml", "oMath"),
        },
        Feature {
            id: "word.controls",
            format: "docx",
            name: "Content controls",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W09.05",
            count: |p| elements(p, "word/document.xml", "sdt"),
        },
        Feature {
            id: "word.charts",
            format: "docx",
            name: "Charts and SmartArt",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W08.06–W08.07",
            count: |p| {
                parts(p, |k| {
                    (k.starts_with("word/charts/chart") || k.starts_with("word/diagrams/data"))
                        && k.ends_with(".xml")
                })
            },
        },
        Feature {
            id: "word.objects",
            format: "docx",
            name: "Embedded objects",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "W09.02",
            count: |p| parts(p, |k| k.starts_with("word/embeddings/")),
        },
        Feature {
            id: "word.bookmarks",
            format: "docx",
            name: "Bookmarks and hyperlinks",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W04.07, W01.14",
            count: |p| {
                elements(p, "word/document.xml", "bookmarkStart")
                    + elements(p, "word/document.xml", "hyperlink")
            },
        },
        Feature {
            id: "word.sections",
            format: "docx",
            name: "Sections, page setup and columns",
            states: [V, P, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "W05",
            count: |p| elements(p, "word/document.xml", "sectPr"),
        },
        // ── Excel ─────────────────────────────────────────────────────────
        Feature {
            id: "excel.cells",
            format: "xlsx",
            name: "Cells, values and formulas",
            states: [V, V, V, P, V, V, U, P],
            on_save: "Written from Folio's edits with recalculated values",
            evidence: "X01, X08; corpus: 99.87% of formulas match Excel",
            count: |p| elements(p, "xl/worksheets/", "c"),
        },
        Feature {
            id: "excel.styles",
            format: "xlsx",
            name: "Cell styles and number formats",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X04",
            count: |p| elements(p, "xl/styles.xml", "xf"),
        },
        Feature {
            id: "excel.merged",
            format: "xlsx",
            name: "Merged cells",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X04",
            count: |p| elements(p, "xl/worksheets/", "mergeCell"),
        },
        Feature {
            id: "excel.tables",
            format: "xlsx",
            name: "Tables",
            states: [V, V, V, V, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X06",
            count: |p| parts(p, |k| k.starts_with("xl/tables/") && k.ends_with(".xml")),
        },
        Feature {
            id: "excel.conditional",
            format: "xlsx",
            name: "Conditional formatting",
            states: [V, V, V, V, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X07",
            count: |p| elements(p, "xl/worksheets/", "conditionalFormatting"),
        },
        Feature {
            id: "excel.validation",
            format: "xlsx",
            name: "Data validation",
            states: [V, V, V, V, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X10",
            count: |p| elements(p, "xl/worksheets/", "dataValidation"),
        },
        Feature {
            id: "excel.comments",
            format: "xlsx",
            name: "Comments and notes",
            states: [V, V, V, NA, V, V, U, NA],
            on_save: "Kept, with Folio's changes",
            evidence: "X15",
            count: |p| {
                elements(p, "xl/comments", "comment")
                    + elements(p, "xl/threadedComments/", "threadedComment")
            },
        },
        Feature {
            id: "excel.names",
            format: "xlsx",
            name: "Defined names",
            states: [V, NA, P, V, V, V, NA, NA],
            on_save: "Kept",
            evidence: "X09.08",
            count: |p| elements(p, "xl/workbook.xml", "definedName"),
        },
        Feature {
            id: "excel.charts",
            format: "xlsx",
            name: "Charts",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "X11",
            count: |p| parts(p, |k| k.starts_with("xl/charts/chart") && k.ends_with(".xml")),
        },
        Feature {
            id: "excel.pivots",
            format: "xlsx",
            name: "PivotTables",
            states: [V, K, K, K, K, K, U, NA],
            on_save: "Kept unchanged; not refreshed",
            evidence: "X12",
            count: |p| parts(p, |k| k.starts_with("xl/pivotTables/") && k.ends_with(".xml")),
        },
        Feature {
            id: "excel.sparklines",
            format: "xlsx",
            name: "Sparklines",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "X11",
            count: |p| elements(p, "xl/worksheets/", "sparklineGroup"),
        },
        Feature {
            id: "excel.slicers",
            format: "xlsx",
            name: "Slicers",
            states: [V, K, K, NA, K, K, U, NA],
            on_save: "Kept unchanged",
            evidence: "X12",
            count: |p| parts(p, |k| k.starts_with("xl/slicers/") && k.ends_with(".xml")),
        },
        Feature {
            id: "excel.links",
            format: "xlsx",
            name: "Links to other workbooks",
            states: [V, K, U, U, K, K, U, NA],
            on_save: "Kept unchanged; values not refreshed",
            evidence: "X13",
            count: |p| parts(p, |k| k.starts_with("xl/externalLinks/externalLink")),
        },
        Feature {
            id: "excel.connections",
            format: "xlsx",
            name: "Data connections and queries",
            states: [V, K, U, U, K, K, U, NA],
            on_save: "Kept unchanged; not refreshed",
            evidence: "X13",
            count: |p| {
                elements(p, "xl/connections.xml", "connection")
                    + parts(p, |k| k.starts_with("xl/queryTables/"))
            },
        },
        // ── Both ──────────────────────────────────────────────────────────
        Feature {
            id: "any.macros",
            format: "both",
            name: "Macros",
            states: [K, NA, U, NA, U, U, NA, U],
            on_save: "Refused: Folio doesn't write files that contain macros",
            evidence: "D06",
            count: |p| parts(p, |k| k.to_ascii_lowercase().ends_with("vbaproject.bin")),
        },
        Feature {
            id: "any.signatures",
            format: "both",
            name: "Digital signatures",
            states: [V, NA, U, NA, U, K, NA, NA],
            on_save: "Removed when the file has changed, as a signature no longer matches it; kept on an unchanged export",
            evidence: "D06",
            count: |p| parts(p, |k| k.starts_with("_xmlsignatures/sig")),
        },
        Feature {
            id: "any.protection",
            format: "both",
            name: "Protection",
            states: [V, NA, P, NA, K, K, NA, NA],
            on_save: "Kept unchanged; not enforced in Folio",
            evidence: "D06, W15, X16",
            count: |p| {
                elements(p, "word/settings.xml", "documentProtection")
                    + elements(p, "xl/workbook.xml", "workbookProtection")
                    + elements(p, "xl/worksheets/", "sheetProtection")
            },
        },
        Feature {
            id: "any.customxml",
            format: "both",
            name: "Custom XML data",
            states: [V, NA, NA, NA, K, K, NA, NA],
            on_save: "Kept unchanged",
            evidence: "W08.08",
            count: |p| parts(p, |k| k.starts_with("customXml/item") && !k.contains("Props")),
        },
        Feature {
            id: "any.unknown",
            format: "both",
            name: "Other package parts",
            states: [V, K, K, NA, K, K, NA, NA],
            on_save: "Kept unchanged",
            evidence: "W08.08",
            count: |p| parts(p, unknown_part),
        },
    ]
}

impl Feature {
    /// The one state the report groups the feature under: unsupported when
    /// saving can't keep it, otherwise how far Folio edits it.
    pub fn overall(&self) -> State {
        match (self.states[4], self.states[2]) {
            (U, _) => U,
            (_, V) => V,
            (_, P) => P,
            _ => K,
        }
    }
}

fn applies(feature: &Feature, format: &str) -> bool {
    feature.format == format || feature.format == "both"
}

/// Character formatting Folio edits; other run properties are lost when a
/// paragraph's text is rewritten.
const KEPT_RUN_PROPERTIES: [&str; 11] = [
    "b",
    "i",
    "u",
    "strike",
    "color",
    "sz",
    "rFonts",
    "shd",
    "highlight",
    "szCs",
    "rPrChange",
];

fn run_property_label(name: &str) -> &'static str {
    match name {
        "smallCaps" => "small caps",
        "caps" => "all caps",
        "spacing" => "character spacing",
        "kern" => "kerning",
        "w" => "character scale",
        "position" => "raised or lowered text",
        "vertAlign" => "superscript and subscript",
        "rStyle" => "character styles",
        "dstrike" => "double strikethrough",
        "vanish" | "specVanish" | "webHidden" => "hidden text",
        "emboss" | "imprint" | "outline" | "shadow" | "effect" => "text effects",
        "lang" | "noProof" => "language and proofing settings",
        "bCs" | "iCs" | "cs" | "rtl" => "right-to-left and complex-script formatting",
        "em" | "eastAsianLayout" | "fitText" => "East Asian layout",
        "bdr" => "character borders",
        _ => "other character formatting",
    }
}

/// Paragraphs whose text changed from the source and the character
/// formatting that exporting them will drop.
fn lost_run_formatting(
    pkg: &Package,
    document: &Value,
    baseline: &Value,
) -> Vec<(String, BTreeSet<&'static str>)> {
    let Ok(xml) = pkg.text("word/document.xml") else {
        return vec![];
    };
    let Ok(dom) = parse(xml) else { return vec![] };
    let Some(body) = dom.descendants().find(|n| n.tag_name().name() == "body") else {
        return vec![];
    };
    let mut sources: HashMap<String, &str> = HashMap::new();
    for (i, n) in body
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() != "sectPr")
        .enumerate()
    {
        crate::docx_tables::source_map(xml, n, format!("word-{i}"), &mut sources);
    }
    fn paragraphs<'a>(n: &'a Value, out: &mut Vec<&'a Value>) {
        if n["attrs"]["officeId"].is_string() && n["type"] != "table" {
            out.push(n);
        }
        for c in n["content"].as_array().into_iter().flatten() {
            paragraphs(c, out);
        }
    }
    let (mut now, mut before) = (vec![], vec![]);
    paragraphs(document, &mut now);
    paragraphs(baseline, &mut before);
    let before: HashMap<&str, &Value> = before
        .into_iter()
        .filter_map(|n| Some((n["attrs"]["officeId"].as_str()?, n)))
        .collect();
    let mut out = vec![];
    for n in now {
        let id = n["attrs"]["officeId"].as_str().unwrap();
        let Some(old) = before.get(id) else { continue };
        if old["content"] == n["content"] {
            continue;
        }
        let Some(source) = sources.get(id) else {
            continue;
        };
        let wrapped = format!(
            "<x xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{source}</x>"
        );
        let Ok(d) = parse(&wrapped) else { continue };
        let labels: BTreeSet<&'static str> = d
            .descendants()
            .filter(|e| e.is_element() && e.tag_name().name() == "rPr")
            // Run properties, not a paragraph mark's.
            .filter(|e| e.parent().is_some_and(|p| p.tag_name().name() == "r"))
            .flat_map(|rpr| rpr.children().filter(|c| c.is_element()))
            .map(|c| c.tag_name().name())
            .filter(|name| !KEPT_RUN_PROPERTIES.contains(name))
            .map(run_property_label)
            .collect();
        if !labels.is_empty() {
            out.push((id.to_string(), labels));
        }
    }
    out
}

/// What the open file contains, feature by feature, and what exporting it
/// now will lose or invalidate.
pub fn report(
    format: &str,
    pkg: Option<&Package>,
    document: Option<&Value>,
    baseline: Option<&Value>,
    changed: bool,
) -> Value {
    let Some(pkg) = pkg else {
        return json!({"format": format, "features": [], "impacts": [], "created": true});
    };
    let mut features = vec![];
    let mut impacts = vec![];
    for f in features_for(format) {
        let count = (f.count)(pkg);
        if count == 0 {
            continue;
        }
        let states: BTreeMap<&str, &str> = CAPABILITIES
            .iter()
            .zip(f.states.iter())
            .map(|(c, s)| (*c, s.label()))
            .collect();
        features.push(json!({
            "id": f.id,
            "name": f.name,
            "count": count,
            "state": f.overall().label(),
            "states": states,
            "onSave": f.on_save,
        }));
        if changed && f.id == "any.signatures" {
            impacts.push(json!({
                "id": f.id,
                "effect": "removed",
                "message": format!(
                    "The {} will be removed, because the file has changed since it was signed.",
                    if count == 1 { "digital signature".to_string() } else { format!("{count} digital signatures") }
                ),
            }));
        }
    }
    if let (Some(document), Some(baseline), "docx") = (document, baseline, format) {
        let lost = lost_run_formatting(pkg, document, baseline);
        if !lost.is_empty() {
            let kinds: BTreeSet<&str> = lost.iter().flat_map(|(_, l)| l.iter().copied()).collect();
            impacts.push(json!({
                "id": "word.text",
                "effect": "changed",
                "message": format!(
                    "{} edited {} will lose formatting Folio doesn't edit: {}.",
                    lost.len(),
                    if lost.len() == 1 { "paragraph" } else { "paragraphs" },
                    kinds.into_iter().collect::<Vec<_>>().join(", ")
                ),
                "paragraphs": lost.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
            }));
        }
    }
    json!({"format": format, "features": features, "impacts": impacts})
}

pub fn features_for(format: &str) -> impl Iterator<Item = Feature> + '_ {
    features().into_iter().filter(move |f| applies(f, format))
}

/// The matrix as Markdown, for docs/status/COMPATIBILITY.md.
#[cfg_attr(not(test), allow(dead_code))]
pub fn matrix_markdown() -> String {
    let mut out = String::new();
    for (format, title) in [
        ("docx", "Word documents (DOCX)"),
        ("xlsx", "Excel workbooks (XLSX)"),
    ] {
        out.push_str(&format!(
            "## {title}\n\n| Feature | {} | Saving an edited file | Evidence |\n|---|{}---|---|\n",
            CAPABILITIES.join(" | "),
            "---|".repeat(CAPABILITIES.len())
        ));
        for f in features_for(format) {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                f.name,
                f.states
                    .iter()
                    .map(|s| s.label())
                    .collect::<Vec<_>>()
                    .join(" | "),
                f.on_save,
                f.evidence
            ));
        }
        out.push('\n');
    }
    out
}

const SIGNATURE_ORIGIN: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/origin";

/// Removes a package's digital signatures: the signature parts, the
/// relationship to their origin and their content types.
pub fn remove_signatures(pkg: &mut Package) -> Result<(), String> {
    let signed: Vec<String> = pkg
        .entries
        .keys()
        .filter(|k| k.starts_with("_xmlsignatures/"))
        .cloned()
        .collect();
    if signed.is_empty() {
        return Ok(());
    }
    for part in &signed {
        pkg.entries.remove(part);
    }
    let without = |xml: &str, keep: &dyn Fn(roxmltree::Node) -> bool| -> Result<String, String> {
        let d = parse(xml)?;
        let mut ranges: Vec<_> = d
            .root_element()
            .children()
            .filter(|n| n.is_element() && !keep(*n))
            .map(|n| n.range())
            .collect();
        ranges.reverse();
        let mut out = xml.to_string();
        for r in ranges {
            out.replace_range(r, "");
        }
        Ok(out)
    };
    if let Ok(rels) = pkg.text("_rels/.rels").map(String::from) {
        let next = without(&rels, &|n| n.attribute("Type") != Some(SIGNATURE_ORIGIN))?;
        pkg.set("_rels/.rels", next);
    }
    let types = pkg.text("[Content_Types].xml")?.to_string();
    let next = without(&types, &|n| {
        !n.attribute("PartName")
            .is_some_and(|p| p.starts_with("/_xmlsignatures/"))
    })?;
    pkg.set("[Content_Types].xml", next);
    pkg.original = pkg.bytes()?;
    Ok(())
}
