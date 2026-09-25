//! Compatibility checks against files saved by Word and Excel themselves
//! (docs/status/office-corpus.tsv; fetch them with
//! scripts/verification/fetch_office_corpus.py). Opt-in, since the files
//! aren't committed:
//!
//!     cargo test --release office_corpus -- --ignored --nocapture
//!
//! For each file it records:
//! - whether import succeeds, the error if not, or the panic location;
//! - export and reimport: whether the projected content survives unchanged;
//! - XLSX: each formula's recomputed value against Excel's cached result.
//!   That is a differential calculation check against real Excel output;
//!   volatile and external-reference formulas are skipped.
//! - DOCX: the document's words against the words in Word's own XML.
//!
//! The summary goes to stdout, and the per-file results to
//! build/office-corpus-report.json.
use crate::workspace::Core;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn corpus_dir() -> PathBuf {
    std::env::var("FOLIO_OFFICE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/office-corpus"))
}

fn manifest() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/status/office-corpus.tsv");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let cells: Vec<&str> = l.split('\t').collect();
            (cells[0].to_string(), format!("{} {}", cells[2], cells[3]))
        })
        .collect()
}

/// Dispatches, turning a panic into its message.
fn call(core: &mut Core, request: Value) -> Result<Value, String> {
    match catch_unwind(AssertUnwindSafe(|| core.dispatch(request))) {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "PANIC: {}",
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
    }
}

/// Comparable form of a cell value: numbers as f64, booleans as 1/0 (Excel
/// caches booleans that way), everything else as text.
enum Comparable {
    Number(f64),
    Text(String),
}
fn comparable(v: &Value) -> Comparable {
    match v {
        Value::Number(n) => Comparable::Number(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => Comparable::Number(if *b { 1.0 } else { 0.0 }),
        Value::Null => Comparable::Text(String::new()),
        Value::String(s) => s
            .parse::<f64>()
            .map(Comparable::Number)
            .unwrap_or_else(|_| Comparable::Text(s.clone())),
        other => Comparable::Text(other.to_string()),
    }
}
fn same(excel: &Value, folio: &Value) -> bool {
    match (comparable(excel), comparable(folio)) {
        (Comparable::Number(a), Comparable::Number(b)) => {
            a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
        }
        (Comparable::Text(a), Comparable::Text(b)) => a == b,
        _ => false,
    }
}

const VOLATILE: [&str; 9] = [
    "NOW(",
    "TODAY(",
    "RAND(",
    "RANDBETWEEN(",
    "RANDARRAY(",
    "INFO(",
    "CELL(",
    "OFFSET(",
    "INDIRECT(",
];

/// Formula cells whose recomputed value differs from Excel's cache, and the
/// number compared.
fn formula_differences(core: &mut Core) -> (usize, Vec<String>) {
    let snapshot = core.snapshot();
    let mut compared = 0;
    let mut differences = vec![];
    for sheet in core.workspace["sheets"].as_array().into_iter().flatten() {
        let id = sheet["id"].as_str().unwrap_or("");
        let empty = Map::new();
        let cached = sheet["cached"].as_object().unwrap_or(&empty);
        for (address, formula) in sheet["cells"].as_object().unwrap_or(&empty) {
            let Some(formula) = formula.as_str().filter(|f| f.starts_with('=')) else {
                continue;
            };
            let upper = formula.to_uppercase();
            if VOLATILE.iter().any(|v| upper.contains(v)) || formula.contains('[') {
                continue;
            }
            let Some(excel) = cached.get(address) else {
                continue;
            };
            let folio = &snapshot["calculated"][id][address];
            compared += 1;
            if !same(excel, folio) {
                differences.push(format!(
                    "{}!{address} {formula} excel={excel} folio={folio}",
                    sheet["name"].as_str().unwrap_or("?")
                ));
            }
        }
    }
    (compared, differences)
}

/// Words in the body of a Word document's XML: w:t text, not deleted text
/// or field codes, and not text boxes.
fn word_words(path: &Path) -> Option<Vec<String>> {
    let package = crate::office::Package::read(path.to_str()?).ok()?;
    let xml = package.text("word/document.xml").ok()?;
    let doc = roxmltree::Document::parse(xml).ok()?;
    let mut text = String::new();
    for node in doc.descendants() {
        let in_box = node
            .ancestors()
            .any(|a| a.tag_name().name() == "txbxContent" || a.tag_name().name() == "del");
        if in_box {
            continue;
        }
        match node.tag_name().name() {
            "t" => text.push_str(node.text().unwrap_or("")),
            "tab" | "br" | "p" => text.push(' '),
            _ => {}
        }
    }
    Some(text.split_whitespace().map(String::from).collect())
}

/// The share of `expected` words also present in `actual` (a multiset match).
fn word_overlap(expected: &[String], actual: &[String]) -> f64 {
    if expected.is_empty() {
        return if actual.is_empty() { 1.0 } else { 0.0 };
    }
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for w in actual {
        *counts.entry(w).or_default() += 1;
    }
    let mut found = 0;
    for w in expected {
        if let Some(c) = counts.get_mut(w.as_str()).filter(|c| **c > 0) {
            *c -= 1;
            found += 1;
        }
    }
    found as f64 / expected.len().max(actual.len()) as f64
}

fn content(core: &Core, kind: &str) -> Value {
    if kind == "docx" {
        json!(crate::workspace::document_text(&core.workspace["document"]))
    } else {
        json!(core.workspace["sheets"]
            .as_array()
            .map(|s| s.iter().map(|s| s["cells"].clone()).collect::<Vec<_>>()))
    }
}

#[test]
#[ignore = "needs build/office-corpus; see scripts/verification/fetch_office_corpus.py"]
fn office_corpus() {
    let dir = corpus_dir();
    let out = std::env::temp_dir().join(format!("folio-office-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let mut results = vec![];
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    let (mut formulas, mut formula_differences_total) = (0, 0);
    let mut files_with_differences = 0;
    for (name, application) in manifest() {
        let path = dir.join(&name);
        assert!(path.exists(), "{name} missing: run fetch_office_corpus.py");
        let kind = if name.ends_with(".docx") {
            "docx"
        } else {
            "xlsx"
        };
        let mut core = Core::new();
        let started = Instant::now();
        let imported = call(&mut core, json!({"op":"importOffice","path":path}));
        let import_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut result = json!({"file":name,"application":application,"importMs":import_ms});
        let outcome = match &imported {
            Err(e) if e.starts_with("PANIC") => format!("{kind}: panic"),
            Err(e) => format!(
                "{kind}: rejected: {}",
                e.chars().take(80).collect::<String>()
            ),
            Ok(_) => format!("{kind}: imported"),
        };
        *outcomes.entry(outcome.clone()).or_default() += 1;
        result["outcome"] = json!(outcome);
        if let Err(e) = &imported {
            result["error"] = json!(e);
            results.push(result);
            continue;
        }
        let before = content(&core, kind);
        if kind == "xlsx" {
            let (compared, differences) = formula_differences(&mut core);
            formulas += compared;
            formula_differences_total += differences.len();
            if !differences.is_empty() {
                files_with_differences += 1;
            }
            result["formulas"] = json!(compared);
            result["formulaDifferences"] = json!(differences);
        } else if let Some(expected) = word_words(&path) {
            let actual: Vec<String> = crate::workspace::document_text(&core.workspace["document"])
                .split_whitespace()
                .map(String::from)
                .collect();
            result["words"] = json!(expected.len());
            result["wordOverlap"] = json!(word_overlap(&expected, &actual));
        }
        // Export, then reimport the export: the projection must survive.
        let exported = out.join(&name);
        let round_trip = call(
            &mut core,
            json!({"op":"exportOffice","format":kind,"path":exported}),
        )
        .and_then(|_| {
            let mut again = Core::new();
            call(&mut again, json!({"op":"importOffice","path":exported}))?;
            Ok(content(&again, kind))
        });
        let trip = match round_trip {
            Ok(after) if after == before => "stable".to_string(),
            Ok(_) => "changed".to_string(),
            Err(e) => format!("failed: {}", e.chars().take(80).collect::<String>()),
        };
        *outcomes
            .entry(format!(
                "{kind}: round trip {}",
                trip.split(':').next().unwrap()
            ))
            .or_default() += 1;
        result["roundTrip"] = json!(trip);
        results.push(result);
    }
    let report = Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/office-corpus-report.json");
    std::fs::write(&report, serde_json::to_string_pretty(&results).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    for (outcome, n) in &outcomes {
        println!("{n:5} {outcome}");
    }
    println!(
        "formulas compared with Excel's cached results: {formulas}, differing: \
         {formula_differences_total} (in {files_with_differences} files)"
    );
    let overlaps: Vec<f64> = results
        .iter()
        .filter_map(|r| r["wordOverlap"].as_f64())
        .collect();
    let low = overlaps.iter().filter(|o| **o < 0.99).count();
    println!(
        "documents with text compared: {}, below 99% word overlap: {low}",
        overlaps.len()
    );
    println!("report: {}", report.display());
    let panics: Vec<&Value> = results
        .iter()
        .filter(|r| r["outcome"].as_str().is_some_and(|o| o.ends_with("panic")))
        .collect();
    assert!(panics.is_empty(), "panics: {panics:#?}");
}
