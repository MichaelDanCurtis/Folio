//! Mutated and truncated files must produce errors, never panics or hangs.
//! The default run covers a fixed seed set; `fuzz_long` takes
//! `FOLIO_FUZZ_CASES` / `FOLIO_FUZZ_SEED` for longer searches:
//! `cargo test --release fuzz_long -- --ignored --nocapture`.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};

/// How long one file operation may take before it counts as a hang. Debug
/// builds are several times slower and share the machine with the rest of
/// the suite, so they get more room; the limit exists to catch runaway work
/// (minutes), not to measure speed.
pub(crate) fn slow_limit() -> Duration {
    Duration::from_secs(if cfg!(debug_assertions) { 60 } else { 10 })
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

thread_local! {
    static FUZZING: RefCell<bool> = const { RefCell::new(false) };
    static PANIC_AT: RefCell<Option<String>> = const { RefCell::new(None) };
    /// How each open ended: "ok" or its error message, for the long run.
    static OUTCOMES: RefCell<BTreeMap<String, usize>> = const { RefCell::new(BTreeMap::new()) };
    /// The slowest sessions: seconds, fixture and mutation.
    static SLOWEST: RefCell<Vec<(f64, String)>> = const { RefCell::new(Vec::new()) };
}

/// The first frame of Folio's own code in a backtrace: its `src/file:line`
/// when the build has line tables, else its function name.
fn own_frame(trace: &str) -> Option<String> {
    let mut lines = trace.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        let Some((_, name)) = line.split_once(": ") else {
            continue;
        };
        let at = lines.peek().and_then(|l| l.strip_prefix("at "));
        let at = at.map(|a| {
            a.trim_start_matches(env!("CARGO_MANIFEST_DIR"))
                .trim_start_matches(['.', '/'])
                .to_string()
        });
        let own = name.trim_start_matches('<').starts_with("folio_core::")
            && !name.contains("fuzz_tests");
        if own {
            return Some(match at {
                Some(at) if at.starts_with("src/") => at
                    .rsplit_once(':')
                    .map_or(at.clone(), |(f, _)| f.to_string()),
                _ => name.to_string(),
            });
        }
    }
    None
}

/// Records where a panic happened on fuzzing threads instead of printing it.
fn install_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if FUZZING.with(|f| *f.borrow()) {
                let mut at = info
                    .location()
                    .map(|l| format!("{}:{}", l.file(), l.line()))
                    .unwrap_or_else(|| "unknown".into());
                // A panic inside a dependency (a serde_json index, say) is
                // reported at the first frame in Folio's own code.
                if !at.starts_with("src/") {
                    if let Some(own) =
                        own_frame(&std::backtrace::Backtrace::force_capture().to_string())
                    {
                        at = own;
                    }
                }
                let message = info
                    .payload()
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| info.payload().downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                PANIC_AT.with(|p| *p.borrow_mut() = Some(format!("{at}: {message}")));
            } else {
                previous(info);
            }
        }));
    });
}

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("docx" | "xlsx")
            )
        })
        .collect();
    files.sort();
    files
}

fn unzip(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let mut parts = vec![];
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).ok()?;
        let mut data = vec![];
        file.read_to_end(&mut data).ok()?;
        parts.push((file.name().to_string(), data));
    }
    Some(parts)
}

fn rezip(parts: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = zip::ZipWriter::new(Cursor::new(vec![]));
    for (name, data) in parts {
        out.start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

const VALUES: [&str; 11] = [
    "",
    "-1",
    "0",
    "4294967296",
    "99999999999999999999",
    "NaN",
    "XFD1048576",
    "A1:XFD1048576",
    "🙂",
    "1:1048576",
    "=A1",
];

/// Byte ranges of every `name="value"` attribute value in `xml`.
fn attribute_values(xml: &str) -> Vec<(usize, usize)> {
    let b = xml.as_bytes();
    let mut spans = vec![];
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'=' && b[i + 1] == b'"' {
            if let Some(end) = xml[i + 2..].find('"') {
                spans.push((i + 2, i + 2 + end));
                i += 2 + end;
            }
        }
        i += 1;
    }
    spans
}

/// Byte ranges of elements: self-closing tags, or a start tag through the
/// first matching end tag.
fn elements(xml: &str) -> Vec<(usize, usize)> {
    let mut spans = vec![];
    for (start, _) in xml.match_indices('<') {
        let rest = &xml[start + 1..];
        if rest.starts_with(['/', '?', '!']) {
            continue;
        }
        let Some(close) = rest.find('>') else {
            continue;
        };
        let tag = &rest[..close];
        if tag.ends_with('/') {
            spans.push((start, start + close + 2));
            continue;
        }
        let name: String = tag.chars().take_while(|c| !c.is_whitespace()).collect();
        let end_tag = format!("</{name}>");
        if let Some(end) = xml[start..].find(&end_tag) {
            spans.push((start, start + end + end_tag.len()));
        }
    }
    spans
}

/// Attributes that hold cell ranges, indices or counts.
const RANGE_ATTRIBUTES: [&str; 14] = [
    "ref",
    "sqref",
    "r",
    "spans",
    "min",
    "max",
    "count",
    "topLeftCell",
    "xSplit",
    "ySplit",
    "colId",
    "gridSpan",
    "w:val",
    "localSheetId",
];
const RANGES: [&str; 6] = [
    "A1:XFD1048576",
    "XFD1048576",
    "1:1048576",
    "A:XFD",
    "1048576",
    "16384",
];

/// Mutates XML text; returns a description.
fn mutate_xml(r: &mut Lcg, xml: &mut String) -> String {
    match r.below(8) {
        6 | 7 => {
            // A huge range or index on an attribute that holds one.
            let spans: Vec<_> = attribute_values(xml)
                .into_iter()
                .filter(|(a, _)| {
                    let name = xml[..a - 2]
                        .rsplit(char::is_whitespace)
                        .next()
                        .unwrap_or("");
                    RANGE_ATTRIBUTES.contains(&name)
                })
                .collect();
            if spans.is_empty() {
                return "no range attributes".into();
            }
            let (a, b) = spans[r.below(spans.len())];
            let value = RANGES[r.below(RANGES.len())];
            let old = xml[a..b].to_string();
            xml.replace_range(a..b, value);
            format!("range attribute {old:?} -> {value:?}")
        }
        0 => {
            let at = r.below(xml.len());
            let at = (0..=at)
                .rev()
                .find(|i| xml.is_char_boundary(*i))
                .unwrap_or(0);
            xml.truncate(at);
            format!("truncate xml at {at}")
        }
        1 | 2 => {
            let spans = attribute_values(xml);
            if spans.is_empty() {
                return "no attributes".into();
            }
            let (a, b) = spans[r.below(spans.len())];
            let value = if r.below(8) == 0 {
                "A".repeat(5000)
            } else {
                VALUES[r.below(VALUES.len())].to_string()
            };
            let old = xml[a..b].to_string();
            xml.replace_range(a..b, &value);
            format!(
                "attribute {old:?} -> {:?}",
                value.chars().take(20).collect::<String>()
            )
        }
        3 => {
            let spans = elements(xml);
            if spans.is_empty() {
                return "no elements".into();
            }
            let (a, b) = spans[r.below(spans.len())];
            let removed: String = xml[a..b].chars().take(40).collect();
            xml.replace_range(a..b, "");
            format!("delete element {removed:?}")
        }
        4 => {
            let spans = elements(xml);
            if spans.is_empty() {
                return "no elements".into();
            }
            let (a, b) = spans[r.below(spans.len())];
            let copy = xml[a..b].to_string();
            let times = 1 + r.below(3) * 50;
            xml.insert_str(b, &copy.repeat(times));
            format!(
                "duplicate element x{times} {:?}",
                copy.chars().take(40).collect::<String>()
            )
        }
        _ => {
            // Remove an attribute: name="value".
            let spans = attribute_values(xml);
            if spans.is_empty() {
                return "no attributes".into();
            }
            let (a, b) = spans[r.below(spans.len())];
            let start = xml[..a - 2]
                .rfind(char::is_whitespace)
                .map_or(a - 2, |i| i + 1);
            let removed = xml[start..=b].to_string();
            xml.replace_range(start..=b, "");
            format!("remove attribute {removed:?}")
        }
    }
}

/// A mutated copy of `bytes` (an Office package) and a description.
fn mutate_package(r: &mut Lcg, bytes: &[u8]) -> (Vec<u8>, String) {
    let Some(mut parts) = unzip(bytes) else {
        return (bytes.to_vec(), "unreadable fixture".into());
    };
    match r.below(10) {
        0 => {
            let at = r.below(bytes.len());
            (bytes[..at].to_vec(), format!("truncate package at {at}"))
        }
        1 => {
            let mut out = bytes.to_vec();
            for _ in 0..1 + r.below(8) {
                let at = r.below(out.len());
                out[at] ^= 1 << r.below(8);
            }
            (out, "flip bytes".into())
        }
        2 => {
            let i = r.below(parts.len());
            let (name, _) = parts.remove(i);
            (rezip(&parts), format!("delete part {name}"))
        }
        3 => {
            let i = r.below(parts.len());
            parts[i].1.clear();
            let name = parts[i].0.clone();
            (rezip(&parts), format!("empty part {name}"))
        }
        _ => {
            // XML mutation, biased to worksheet/document/style parts.
            let xml_parts: Vec<usize> = (0..parts.len())
                .filter(|i| parts[*i].0.ends_with(".xml") || parts[*i].0.ends_with(".rels"))
                .collect();
            if xml_parts.is_empty() {
                return (bytes.to_vec(), "no xml parts".into());
            }
            // Mostly the parts the importers read closely.
            let content: Vec<usize> = xml_parts
                .iter()
                .copied()
                .filter(|i| {
                    let n = &parts[*i].0;
                    ![
                        "theme",
                        "docProps",
                        "customXml",
                        "fontTable",
                        "webSettings",
                        "printerSettings",
                    ]
                    .iter()
                    .any(|skip| n.contains(skip))
                })
                .collect();
            let pool = if content.is_empty() || r.below(4) == 0 {
                &xml_parts
            } else {
                &content
            };
            let i = pool[r.below(pool.len())];
            let Ok(mut xml) = String::from_utf8(parts[i].1.clone()) else {
                return (bytes.to_vec(), "non-utf8 part".into());
            };
            let what = (0..1 + r.below(3))
                .map(|_| mutate_xml(r, &mut xml))
                .collect::<Vec<_>>()
                .join("; ");
            let name = parts[i].0.clone();
            parts[i].1 = xml.into_bytes();
            (rezip(&parts), format!("{name}: {what}"))
        }
    }
}

/// Mutates a saved `.folio` workspace's JSON; returns a description.
fn mutate_folio(r: &mut Lcg, w: &mut Value) -> String {
    // Walk to a random node, usually well below the top level where the
    // workspace validator doesn't look.
    let depth = 2 + r.below(6);
    let mut path = vec![];
    let mut node = &mut *w;
    for _ in 0..depth {
        let keys: Vec<Value> = match &*node {
            Value::Object(o) if !o.is_empty() => o.keys().map(|k| json!(k)).collect(),
            Value::Array(a) if !a.is_empty() => (0..a.len()).map(|i| json!(i)).collect(),
            _ => break,
        };
        let key = keys[r.below(keys.len())].clone();
        let leaf = match &key {
            Value::String(k) => !node[k.as_str()].is_object() && !node[k.as_str()].is_array(),
            Value::Number(i) => {
                let i = i.as_u64().unwrap() as usize;
                !node[i].is_object() && !node[i].is_array()
            }
            _ => unreachable!(),
        };
        // Stop at the parent of a leaf half the time, to delete or retype it.
        if leaf && r.below(2) == 0 {
            if let (Value::Object(o), Value::String(k)) = (&mut *node, &key) {
                if r.below(3) == 0 {
                    o.remove(k);
                    return format!("delete {path:?}.{k}");
                }
            }
        }
        path.push(key.clone());
        node = match key {
            Value::String(k) => node.get_mut(k.as_str()).unwrap(),
            Value::Number(i) => node.get_mut(i.as_u64().unwrap() as usize).unwrap(),
            _ => unreachable!(),
        };
    }
    let replacement = match r.below(10) {
        0 => Value::Null,
        1 => json!(-1),
        2 => json!("garbage"),
        3 => json!([]),
        4 => json!({}),
        5 => json!(1e300),
        6 => json!(4294967296u64),
        7 => json!(VALUES[r.below(VALUES.len())]),
        8 => json!(true),
        _ => json!("A".repeat(5000)),
    };
    *node = replacement.clone();
    let shown: String = replacement.to_string().chars().take(20).collect();
    format!("set {path:?} to {shown}")
}

struct Case {
    fixture: String,
    mutation: String,
    op: String,
    panic: String,
}

/// Runs `request` on `core`, returning the panic location if it panicked.
fn run(core: &mut Core, request: Value) -> Result<Result<Value, String>, String> {
    PANIC_AT.with(|p| *p.borrow_mut() = None);
    FUZZING.with(|f| *f.borrow_mut() = true);
    let result = catch_unwind(AssertUnwindSafe(|| core.dispatch(request)));
    FUZZING.with(|f| *f.borrow_mut() = false);
    result.map_err(|_| PANIC_AT.with(|p| p.borrow_mut().take().unwrap_or_default()))
}

/// The engine caches the workspace's serialized length for the cell fast
/// path; every op that changes the workspace must keep it right or clear it.
fn stale_size(core: &Core) -> Option<String> {
    let tracked = core.serialized_size?;
    let actual = core.workspace.to_string().len();
    (tracked != actual).then(|| format!("stale tracked size: {tracked} vs {actual}"))
}

/// Opens `path` as `op`, then runs a short editing session if that succeeded.
/// Returns the first panic, and the elapsed time.
fn session(dir: &Path, path: &Path, open: &str) -> (Option<(String, String)>, Duration) {
    let started = Instant::now();
    let mut core = Core::new();
    let path = path.to_str().unwrap();
    let mut requests = vec![json!({"op":open,"path":path})];
    let opened = match run(&mut core, requests[0].clone()) {
        Err(panic) => return (Some((open.into(), panic)), started.elapsed()),
        Ok(r) => r,
    };
    if let Some(stale) = stale_size(&core) {
        return (Some((open.into(), stale)), started.elapsed());
    }
    let outcome = match &opened {
        Ok(_) => format!("{open}: ok"),
        Err(e) => format!("{open}: {}", e.chars().take(90).collect::<String>()),
    };
    OUTCOMES.with(|o| *o.borrow_mut().entry(outcome).or_default() += 1);
    if opened.is_err() || open == "inspectTemplate" || open == "inspectWorkspace" {
        return (None, started.elapsed());
    }
    let sheet = core.workspace["sheets"][0]["id"].clone();
    let folio = dir.join("session.folio");
    requests = vec![
        json!({"op":"get"}),
        json!({"op":"exportOffice","format":"docx","path":dir.join("out.docx")}),
        json!({"op":"exportOffice","format":"xlsx","path":dir.join("out.xlsx")}),
        json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"=1+1"}]}),
        json!({"op":"undo"}),
        json!({"op":"copyCells","sheetId":sheet,"range":"A1:D6"}),
        json!({"op":"pasteCells","sheetId":sheet,"range":"F8","token":"$token"}),
        json!({"op":"inspectScenarioSource","sheetId":sheet,"path":path,
            "workspaceId":"$id","baseRevision":"$revision"}),
        json!({"op":"save","path":folio}),
        json!({"op":"load","path":folio}),
    ];
    let mut token = Value::Null;
    for mut request in requests {
        let op = request["op"].as_str().unwrap().to_string();
        for (field, value) in [
            ("token", token.clone()),
            ("workspaceId", core.workspace["id"].clone()),
            ("baseRevision", core.workspace["revision"].clone()),
        ] {
            if request[field].as_str().is_some_and(|v| v.starts_with('$')) {
                request[field] = value;
            }
        }
        match run(&mut core, request) {
            Err(panic) => return (Some((op, panic)), started.elapsed()),
            Ok(Ok(result)) if op == "copyCells" => token = result["token"].clone(),
            Ok(_) => {}
        }
        if let Some(stale) = stale_size(&core) {
            return (Some((op, stale)), started.elapsed());
        }
    }
    (None, started.elapsed())
}

/// Runs `cases` mutations starting at `seed`; returns failures by location.
fn fuzz(seed: u64, cases: usize) -> BTreeMap<String, Vec<Case>> {
    install_hook();
    let dir = std::env::temp_dir().join(format!("folio-fuzz-{seed}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let corpus: Vec<(String, Vec<u8>)> = fixtures()
        .into_iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    // A saved workspace that embeds one Word and one Excel package.
    let folio = {
        let mut c = Core::new();
        let docx = corpus.iter().find(|(n, _)| n == "source.docx").unwrap();
        let xlsx = corpus.iter().find(|(n, _)| n == "source.xlsx").unwrap();
        for (name, bytes) in [docx, xlsx] {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            c.dispatch(json!({"op":"importOffice","path":path}))
                .unwrap();
        }
        let path = dir.join("corpus.folio");
        c.dispatch(json!({"op":"save","path":path})).unwrap();
        serde_json::from_str::<Value>(&std::fs::read_to_string(&path).unwrap()).unwrap()
    };
    let mut failures: BTreeMap<String, Vec<Case>> = BTreeMap::new();
    let mut r = Lcg(seed);
    for case in 0..cases {
        let folio_case = r.below(4) == 0;
        let pick = r.below(corpus.len());
        let (fixture, path, opens, mutation) = if folio_case {
            let mut w = folio.clone();
            let mutation = mutate_folio(&mut r, &mut w);
            let path = dir.join(format!("case-{case}.folio"));
            std::fs::write(&path, w.to_string()).unwrap();
            (
                "corpus.folio".to_string(),
                path,
                vec!["load", "recoverWorkspace"],
                mutation,
            )
        } else {
            let (name, bytes) = &corpus[pick];
            let (mutated, mutation) = mutate_package(&mut r, bytes);
            let template = r.below(4) == 0;
            let ext = match (name.ends_with(".docx"), template) {
                (true, false) => "docx",
                (true, true) => "dotx",
                (false, false) => "xlsx",
                (false, true) => "xltx",
            };
            let path = dir.join(format!("case-{case}.{ext}"));
            std::fs::write(&path, mutated).unwrap();
            let opens = if template {
                vec!["inspectTemplate", "newFromTemplate"]
            } else {
                vec!["importOffice"]
            };
            (name.clone(), path, opens, mutation)
        };
        for open in opens {
            let (panic, elapsed) = session(&dir, &path, open);
            SLOWEST.with(|slow| {
                let mut slow = slow.borrow_mut();
                slow.push((
                    elapsed.as_secs_f64(),
                    format!("{fixture} {open} [{mutation}]"),
                ));
                slow.sort_by(|a, b| b.0.total_cmp(&a.0));
                slow.truncate(5);
            });
            let failure = panic.map(|(op, panic)| (op, panic)).or_else(|| {
                (elapsed > slow_limit()).then(|| {
                    (
                        open.to_string(),
                        format!("slow: {:.1}s", elapsed.as_secs_f64()),
                    )
                })
            });
            if let Some((op, panic)) = failure {
                let key = panic.split(": ").next().unwrap_or("").to_string();
                failures.entry(key).or_default().push(Case {
                    fixture: fixture.clone(),
                    mutation: mutation.clone(),
                    op,
                    panic,
                });
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    failures
}

fn report(failures: &BTreeMap<String, Vec<Case>>) -> String {
    let mut out = format!("{} distinct failure sites\n", failures.len());
    for (site, cases) in failures {
        let c = &cases[0];
        out += &format!(
            "- {site} ({} cases), e.g. {} [{}] during {}: {}\n",
            cases.len(),
            c.fixture,
            c.mutation,
            c.op,
            c.panic
        );
    }
    out
}

#[test]
fn mutated_files_never_panic() {
    let failures = fuzz(1, 120);
    assert!(failures.is_empty(), "{}", report(&failures));
}

#[test]
#[ignore = "long fuzz run; FOLIO_FUZZ_CASES / FOLIO_FUZZ_SEED"]
fn fuzz_long() {
    let cases = std::env::var("FOLIO_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000);
    let seed = std::env::var("FOLIO_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    let failures = fuzz(seed, cases);
    OUTCOMES.with(|o| {
        let mut o: Vec<_> = o.borrow().iter().map(|(k, v)| (*v, k.clone())).collect();
        o.sort_by(|a, b| b.cmp(a));
        for (n, k) in o.iter().take(40) {
            println!("{n:6} {k}");
        }
    });
    SLOWEST.with(|slow| {
        for (seconds, case) in slow.borrow().iter() {
            println!("slow {seconds:.2}s {case}");
        }
    });
    println!("{}", report(&failures));
    assert!(failures.is_empty());
}

/// A zip whose one part inflates to `real` bytes but declares `declared`.
fn lying_zip(real: usize, declared: u32) -> Vec<u8> {
    let mut bytes = rezip(&[("[Content_Types].xml".into(), vec![b' '; real])]);
    // Uncompressed size: local header offset 22, central directory offset 24.
    bytes[22..26].copy_from_slice(&declared.to_le_bytes());
    let central = bytes
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .unwrap();
    bytes[central + 24..central + 28].copy_from_slice(&declared.to_le_bytes());
    bytes
}

#[test]
fn a_part_that_inflates_past_its_declared_size_is_rejected() {
    let bytes = lying_zip(70 * 1024 * 1024, 1000);
    assert!(bytes.len() < 1024 * 1024);
    let error = crate::office::Package::from_bytes(bytes).err().unwrap();
    assert!(error.contains("larger than it declares"), "{error}");
    // Understating a small part is caught the same way.
    let error = crate::office::Package::from_bytes(lying_zip(5000, 10))
        .err()
        .unwrap();
    assert!(error.contains("larger than it declares"), "{error}");
}

/// Full-sheet ranges in every place a workbook can carry one. Each must
/// import, edit, export and reload quickly, not loop over 17 billion cells.
#[test]
fn full_sheet_ranges_stay_fast() {
    install_hook();
    const ALL: &str = "A1:XFD1048576";
    let sheet: [(&str, &str, String); 11] = [
        (
            "dimension",
            r#"<dimension ref="A1:AA4"/>"#,
            format!(r#"<dimension ref="{ALL}"/>"#),
        ),
        (
            "autoFilter",
            r#"<autoFilter ref="A1:C4"/>"#,
            format!(r#"<autoFilter ref="{ALL}"/>"#),
        ),
        (
            "mergeCell",
            r#"<mergeCell ref="E1:F1"/>"#,
            format!(r#"<mergeCell ref="{ALL}"/>"#),
        ),
        (
            "selection",
            r#"activeCell="A1" sqref="A1""#,
            format!(r#"activeCell="A1" sqref="{ALL}""#),
        ),
        (
            "pane",
            r#"topLeftCell="B2""#,
            r#"topLeftCell="XFD1048576""#.into(),
        ),
        (
            "split",
            r#"xSplit="1" ySplit="1""#,
            r#"xSplit="16384" ySplit="1048576""#.into(),
        ),
        (
            "row",
            r#"<row r="1">"#,
            r#"<row r="1048576" spans="1:16384">"#.into(),
        ),
        (
            "cols",
            "<sheetData>",
            r#"<cols><col min="1" max="16384" width="20" customWidth="1"/></cols><sheetData>"#
                .into(),
        ),
        (
            "row heights",
            "<sheetData>",
            r#"<sheetData><row r="1048576" ht="30" customHeight="1" hidden="1"/>"#.into(),
        ),
        (
            "conditional format",
            "</mergeCells>",
            format!(
                r#"</mergeCells><conditionalFormatting sqref="{ALL}"><cfRule type="expression" priority="1"><formula>A1&gt;0</formula></cfRule><cfRule type="colorScale" priority="2"><colorScale><cfvo type="min"/><cfvo type="max"/><color rgb="FFFF0000"/><color rgb="FF00FF00"/></colorScale></cfRule></conditionalFormatting>"#
            ),
        ),
        (
            "data validation",
            "</mergeCells>",
            format!(
                r#"</mergeCells><dataValidations count="1"><dataValidation type="whole" operator="between" sqref="{ALL}"><formula1>1</formula1><formula2>9</formula2></dataValidation></dataValidations>"#
            ),
        ),
    ];
    let workbook: [(&str, &str, String); 2] = [
        ("print area", "</definedNames>", r#"<definedName name="_xlnm.Print_Area" localSheetId="0">'Budget'!$A$1:$XFD$1048576</definedName></definedNames>"#.into()),
        ("filter database", "'Budget'!$A$1:$C$4", "'Budget'!$A$1:$XFD$1048576".into()),
    ];
    let source =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.xlsx"))
            .unwrap();
    let dir = std::env::temp_dir().join(format!("folio-ranges-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut failures = vec![];
    for (part, (name, from, to)) in sheet
        .iter()
        .map(|c| ("xl/worksheets/sheet1.xml", c))
        .chain(workbook.iter().map(|c| ("xl/workbook.xml", c)))
    {
        let mut parts = unzip(&source).unwrap();
        let entry = parts.iter_mut().find(|(n, _)| n == part).unwrap();
        let xml = String::from_utf8(entry.1.clone()).unwrap();
        assert!(xml.contains(from), "{name}: fixture changed");
        entry.1 = xml.replacen(from, to, 1).into_bytes();
        let path = dir.join(format!("{}.xlsx", name.replace(' ', "-")));
        std::fs::write(&path, rezip(&parts)).unwrap();
        let (panic, elapsed) = session(&dir, &path, "importOffice");
        if let Some((op, panic)) = panic {
            failures.push(format!("{name}: {op}: {panic}"));
        } else if elapsed > slow_limit() {
            failures.push(format!("{name}: {:.1}s", elapsed.as_secs_f64()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Deeply nested XML must be an error or import, never overflow the stack.
/// Nesting just inside the XML limit runs a full session (or stops at a
/// readable error, such as the document tree's own depth limit); deeper
/// nesting is refused before parsing.
#[test]
fn deeply_nested_xml_does_not_overflow_the_stack() {
    install_hook();
    let source =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.docx"))
            .unwrap();
    let dir = std::env::temp_dir().join(format!("folio-nesting-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Opening tags, their levels, and the matching close.
    let cases = [
        (
            "content controls",
            "<w:sdt><w:sdtContent>",
            2,
            "</w:sdtContent></w:sdt>",
        ),
        (
            "tables",
            "<w:tbl><w:tr><w:tc>",
            3,
            "<w:p/></w:tc></w:tr></w:tbl>",
        ),
        ("smart tags", "<w:smartTag>", 1, "</w:smartTag>"),
        (
            "insertions",
            "<w:ins w:id=\"1\" w:author=\"a\">",
            1,
            "</w:ins>",
        ),
    ];
    let mut failures = vec![];
    for (name, open, levels, close) in cases {
        // document > body > [nesting] > p > r > t is 5 levels besides the nesting.
        let fits = (crate::office::MAX_XML_DEPTH - 5) / levels;
        for (depth, may_open) in [(fits, true), (fits + 1, false), (100_000, false)] {
            let mut parts = unzip(&source).unwrap();
            let entry = parts
                .iter_mut()
                .find(|(n, _)| n == "word/document.xml")
                .unwrap();
            let xml = String::from_utf8(entry.1.clone()).unwrap();
            let paragraph = if name == "smart tags" || name == "insertions" {
                "<w:r><w:t>deep</w:t></w:r>"
            } else {
                "<w:p><w:r><w:t>deep</w:t></w:r></w:p>"
            };
            let (open, close) = if paragraph.starts_with("<w:r>") {
                (
                    format!("<w:p>{}", open.repeat(depth)),
                    format!("{}</w:p>", close.repeat(depth)),
                )
            } else {
                (open.repeat(depth), close.repeat(depth))
            };
            entry.1 = xml
                .replacen("<w:body>", &format!("<w:body>{open}{paragraph}{close}"), 1)
                .into_bytes();
            let path = dir.join(format!("{}-{depth}.docx", name.replace(' ', "-")));
            std::fs::write(&path, rezip(&parts)).unwrap();
            // The app calls the engine on worker threads with the default
            // 2 MB stack, in release. Debug frames are several times larger.
            let stack = if cfg!(debug_assertions) { 16 } else { 2 } << 20;
            let run_dir = dir.clone();
            let (panic, opened) = std::thread::Builder::new()
                .stack_size(stack)
                .spawn(move || {
                    install_hook();
                    let (panic, _) = session(&run_dir, &path, "importOffice");
                    let opened = Core::new()
                        .dispatch(json!({"op":"importOffice","path":path}))
                        .map(drop);
                    (panic, opened)
                })
                .unwrap()
                .join()
                .unwrap();
            if let Some((op, panic)) = panic {
                failures.push(format!("{name} x{depth}: {op}: {panic}"));
            } else if opened.is_ok() && !may_open {
                failures.push(format!("{name} x{depth}: opened past the nesting limit"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Regression: `newFromTemplate` set the title and revision after the import
/// had recorded the workspace's serialized size, leaving it stale; the next
/// cell edit then tracked the wrong size.
#[test]
fn a_new_file_from_a_template_keeps_its_tracked_size() {
    install_hook();
    let dir = std::env::temp_dir().join(format!("folio-template-size-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (fixture, template) in [
        ("source.docx", "Report.dotx"),
        ("source.xlsx", "Budget.xltx"),
    ] {
        let path = dir.join(template);
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../test/fixtures")
                .join(fixture),
            &path,
        )
        .unwrap();
        let (failure, _) = session(&dir, &path, "newFromTemplate");
        assert_eq!(failure, None, "{template}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Regression: loading a `.folio` hydrated its Office state before validating
/// it, so a damaged baseline or report panicked on a JSON index.
#[test]
fn damaged_office_state_in_a_saved_workspace_is_an_error() {
    install_hook();
    let dir = std::env::temp_dir().join(format!("folio-damaged-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    let mut core = Core::new();
    for fixture in ["source.docx", "source.xlsx"] {
        core.dispatch(json!({"op":"importOffice","path":fixtures.join(fixture)}))
            .unwrap();
    }
    let saved = dir.join("saved.folio");
    core.dispatch(json!({"op":"save","path":saved})).unwrap();
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&saved).unwrap()).unwrap();
    let damage: [(&str, Value); 6] = [
        ("/officeDocument/baseline", json!("AAAA")),
        ("/officeDocument/baseline/attrs", json!(7)),
        ("/officeWorkbook/report", json!(4294967296u64)),
        ("/officeWorkbook/report/warnings", json!({})),
        (
            "/officeWorkbook/baseline/0/styleDefaults",
            json!(4294967296u64),
        ),
        ("/sheets/0/cached", json!("garbage")),
    ];
    for (pointer, value) in damage {
        let mut w = saved.clone();
        *w.pointer_mut(pointer)
            .unwrap_or_else(|| panic!("{pointer}: fixture changed")) = value;
        let path = dir.join("damaged.folio");
        std::fs::write(&path, w.to_string()).unwrap();
        for op in ["load", "recoverWorkspace", "inspectWorkspace"] {
            let mut core = Core::new();
            let result = run(&mut core, json!({"op":op,"path":path}));
            assert!(matches!(result, Ok(Err(_))), "{pointer} {op}: {result:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A table whose range covers whole columns (or the whole sheet) must not
/// expand its calculated column or totals into a million cells.
#[test]
fn full_column_tables_stay_fast() {
    install_hook();
    let source =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/tables.xlsx"))
            .unwrap();
    let dir = std::env::temp_dir().join(format!("folio-table-ranges-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut failures = vec![];
    for range in ["A1:D1048576", "A1:XFD1048576", "A1048575:D1048576"] {
        let mut parts = unzip(&source).unwrap();
        let entry = parts
            .iter_mut()
            .find(|(n, _)| n == "xl/tables/table1.xml")
            .unwrap();
        let xml = String::from_utf8(entry.1.clone()).unwrap();
        assert!(xml.contains(r#"ref="A1:D5""#), "fixture changed");
        entry.1 = xml
            .replacen(r#"ref="A1:D5""#, &format!(r#"ref="{range}""#), 1)
            .into_bytes();
        let path = dir.join(format!("{}.xlsx", range.replace(':', "-")));
        std::fs::write(&path, rezip(&parts)).unwrap();
        let (panic, elapsed) = session(&dir, &path, "importOffice");
        if let Some((op, panic)) = panic {
            failures.push(format!("{range}: {op}: {panic}"));
        } else if elapsed > slow_limit() {
            failures.push(format!("{range}: {:.1}s", elapsed.as_secs_f64()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "{failures:#?}");
}
