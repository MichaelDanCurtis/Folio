use crate::change_set::ChangeSet;
use crate::formula::{address, parse_address, COLS, ROWS};
use crate::json_patch::{Key, Op};
use crate::perf_stages::time as stage;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_SIZE: usize = 32_000_000;
#[derive(Default)]
pub struct Core {
    pub workspace: Value,
    history: crate::document_history::History,
    calculator: crate::calculation::WorkbookCalculator,
    packages: HashMap<String, Arc<crate::office::Package>>,
    clipboard: Option<Arc<crate::sheet_clipboard::Snapshot>>,
    package_digests: HashMap<String, (std::sync::Weak<crate::office::Package>, String)>,
    layout: Option<LayoutEdit>,
    layout_serial: u64,
    conditional_state: Value,
    conditional: Value,
    conditional_warnings: Value,
    /// The calculator (and its conditional-format state) that the next
    /// staged layout edit reuses, one step behind `calculator`.
    layout_bundle: Option<LayoutBundle>,
    /// Identifies the published state; replaced by every `sync` with a value
    /// no engine in this process has used, so it never repeats.
    epoch: u64,
    /// The base epoch a caller's request says it holds, if it accepts a delta.
    want_delta: Option<u64>,
    /// Workspace paths the next `sync` publishes, set by commits and undo.
    pending_paths: Option<Vec<Vec<crate::json_patch::Key>>>,
    /// What the last `sync` changed, when that can be sent as a delta.
    last_change: Option<LastChange>,
    file_states_cache: Option<(u64, Value)>,
    /// A staged engine whose responses nobody reads: `snapshot` returns null.
    quiet: bool,
    /// Cells a staged engine's calculator hasn't caught up on yet; the next
    /// sync applies them together with its own changes (one evaluation).
    catch_up: Option<Vec<(usize, String)>>,
    goal_seek: Option<crate::goal_seek::Session>,
    goal_seek_serial: u64,
    scenario_source: Option<crate::scenario_source::Source>,
    scenario_source_serial: u64,
    /// `workspace.to_string().len()`, kept current by in-place cell edits and
    /// cleared by every other change.
    serialized_size: Option<usize>,
    /// The last `document` and `officeDocument` that document normalization
    /// left unchanged, so cell edits can skip re-checking them.
    stable_document: Option<(Value, Value)>,
    #[cfg(test)]
    pub(crate) slow_cells: bool,
    /// Test-only: stage layout edits the original way (full sync, full
    /// validation and diff at commit).
    #[cfg(test)]
    pub(crate) slow_layout: bool,
}
/// The difference between two published states.
#[derive(Clone)]
struct LastChange {
    from: u64,
    to: u64,
    paths: Vec<Vec<crate::json_patch::Key>>,
    /// Cells whose calculated or displayed value may differ.
    cells: Vec<(usize, String)>,
    /// Conditional-format warnings may differ.
    warnings: bool,
}
fn next_epoch() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
struct LayoutBundle {
    calculator: crate::calculation::WorkbookCalculator,
    conditional: (Value, Value, Value),
    /// Cells whose input or published value may have changed since
    /// `calculator` last synced, or None if unknown (the next sync then
    /// compares every cell).
    lag: Option<Vec<(usize, String)>>,
}
struct LayoutEdit {
    token: u64,
    base_epoch: u64,
    /// The epoch the staged edit's recorded change starts from, when that
    /// change (plus any deferred catch-up) accounts for every difference from
    /// this engine's state; None when the calendars differ.
    staged_base: Option<u64>,
    /// Every cell whose input or published value may differ from this
    /// engine's state, when known.
    candidates: Option<Vec<(usize, String)>>,
    staged: Box<Core>,
    rows: crate::row_reflow::Targets,
    calculation_only: bool,
}
fn bind_rules(sheets: &mut Value, projection: &Value) {
    crate::data_validation::bind(sheets, projection);
    crate::conditional_format::bind(sheets, projection);
    if let Some(sheets) = sheets.as_array_mut() {
        for sheet in sheets {
            if let Some(source) = projection
                .as_array()
                .and_then(|all| all.iter().find(|s| s["name"] == sheet["name"]))
            {
                if let Some(ranges) = source.get("mergedRanges") {
                    sheet["mergedRanges"] = ranges.clone();
                } else {
                    sheet.as_object_mut().unwrap().remove("mergedRanges");
                }
            }
        }
    }
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key].as_str().ok_or(format!("Missing {key}"))
}
fn cells(sheet: &Value) -> HashMap<String, String> {
    sheet["cells"]
        .as_object()
        .map(|c| {
            c.iter()
                .map(|(a, v)| (a.clone(), v.as_str().unwrap_or("").into()))
                .collect()
        })
        .unwrap_or_default()
}
fn document_with_settings(document: &Value, previous: &Value) -> Result<Value, String> {
    let mut next = document.clone();
    if next.is_object() && next.get("attrs").is_none() {
        if let Some(attrs) = previous.get("attrs") {
            next["attrs"] = attrs.clone();
        }
    }
    if next["attrs"][crate::docx_styles::KEY] != previous["attrs"][crate::docx_styles::KEY] {
        return Err("Use Styles to change the document style catalog".into());
    }
    Ok(next)
}
pub(crate) fn document_validate(v: &Value, depth: usize, count: &mut usize) -> Result<(), String> {
    *count += 1;
    if depth > 40 || *count > 100000 {
        return Err("Document is too complex".into());
    }
    if !v.is_object()
        || !v["type"].is_string()
        || v.get("attrs")
            .is_some_and(|a| !a.is_null() && !a.is_object())
    {
        return Err("Invalid document node".into());
    }
    if v["type"] == "text" && !v["text"].is_string() {
        return Err("Invalid text node".into());
    }
    crate::docx_fields::validate(v)?;
    crate::docx_numbering::validate_node(v)?;
    crate::docx_tabs::validate_node(v)?;
    crate::docx_paragraph::validate(v)?;
    if v["type"] == "table" {
        crate::docx_tables::grid(v)?;
    }
    if let Some(children) = v.get("content") {
        for n in children.as_array().ok_or("Invalid document content")? {
            document_validate(n, depth + 1, count)?
        }
    }
    Ok(())
}
pub fn validate(w: &Value) -> Result<(), String> {
    if w.to_string().len() > MAX_SIZE {
        return Err("Workspace exceeds 32 MB".into());
    }
    if w["format"] != "folio"
        || w["version"] != 1
        || !w["id"].is_string()
        || !w["title"].is_string()
        || w["revision"].as_u64().is_none()
        || w["document"]["type"] != "doc"
    {
        return Err("Not a valid Folio workspace".into());
    }
    document_validate(&w["document"], 0, &mut 0)?;
    if let Some(settings) = w["officeDocument"].get("tracking") {
        crate::docx_tracking::validate_settings(settings)?;
    }
    crate::document_bookmarks::validate(&w["document"])?;
    crate::document_comments::validate(&w["document"])?;
    if let Some(c) = w["document"]["attrs"].get(crate::docx_styles::KEY) {
        crate::docx_styles::validate(c)?;
    }
    let sheets = w["sheets"].as_array().ok_or("Invalid sheets")?;
    if sheets.is_empty() || sheets.len() > 30 {
        return Err("A workspace needs 1–30 sheets".into());
    }
    let mut ids = HashSet::new();
    for s in sheets {
        crate::sheet_reviews::validate(s)?;
        crate::cell_watches::validate(s)?;
        crate::scenarios::validate(s)?;
        crate::data_validation::validate(s)?;
        crate::conditional_format::validate(s)?;
        let id = text(s, "id")?;
        text(s, "name")?;
        if !ids.insert(id) {
            return Err("Duplicate sheet id".into());
        }
        let c = s["cells"].as_object().ok_or("Invalid cells")?;
        if c.len() > 100000 {
            return Err("Too many populated cells".into());
        }
        for (a, v) in c {
            parse_address(a)?;
            if v.as_str().ok_or("Invalid cell value")?.len() > 10000 {
                return Err("Cell exceeds 10,000 bytes".into());
            }
        }
    }
    crate::tables::validate(&w["sheets"])
}
pub fn seed() -> Value {
    let mut v: Value = serde_json::from_str(include_str!("seed.json")).unwrap();
    v["id"] = json!(format!(
        "native-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    v
}
fn workspace_description(w: &Value) -> Value {
    json!({"workspaceId":w["id"], "title":w["title"], "revision":w["revision"],
        "document":document_text(&w["document"]).chars().take(2000).collect::<String>(),
        "sheets":w["sheets"].as_array().map(|s| s.iter().map(|s| s["name"].clone()).collect::<Vec<_>>()).unwrap_or_default()})
}

pub fn document_text(v: &Value) -> String {
    if let Some(s) = v["text"].as_str() {
        return s.into();
    }
    if v["type"] == "hardBreak" {
        return "\n".into();
    }
    let children = v["content"]
        .as_array()
        .map(|a| a.iter().map(document_text).collect::<Vec<_>>())
        .unwrap_or_default();
    let separator = if matches!(
        v["type"].as_str(),
        Some("doc" | "bulletList" | "orderedList" | "table" | "tableRow")
    ) {
        "\n"
    } else {
        ""
    };
    children.join(separator)
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn html(v: &Value) -> String {
    if v["type"] == "text" {
        let mut s = esc(v["text"].as_str().unwrap_or(""));
        if let Some(marks) = v["marks"].as_array() {
            for m in marks {
                let tag = match m["type"].as_str() {
                    Some("bold") => "strong",
                    Some("italic") => "em",
                    Some("underline") => "u",
                    Some("strike") => "s",
                    Some("code") => "code",
                    _ => "",
                };
                if !tag.is_empty() && m["attrs"]["value"] != false {
                    s = format!("<{tag}>{s}</{tag}>")
                }
                if m["type"] == "link" {
                    let href = m["attrs"]["href"].as_str().unwrap_or("");
                    if href.starts_with("https://")
                        || href.starts_with("http://")
                        || href.starts_with("mailto:")
                    {
                        s = format!("<a href=\"{}\">{s}</a>", esc(href));
                    }
                }
            }
        }
        return s;
    }
    let inside = v["content"]
        .as_array()
        .map(|a| a.iter().map(html).collect::<String>())
        .unwrap_or_default();
    let tag = match v["type"].as_str() {
        Some("doc") => return inside,
        Some("paragraph") => "p",
        Some("heading") => match v["attrs"]["level"].as_u64() {
            Some(1) => "h1",
            Some(2) => "h2",
            _ => "h3",
        },
        Some("bulletList") => "ul",
        Some("orderedList") => "ol",
        Some("listItem") => "li",
        Some("blockquote") => "blockquote",
        Some("codeBlock") => "pre",
        Some("table") => "table",
        Some("tableRow") => "tr",
        Some("tableCell") => "td",
        Some("tableHeader") => "th",
        Some("horizontalRule") => return "<hr>".into(),
        Some("hardBreak") => return "<br>".into(),
        _ => "div",
    };
    let spans = if matches!(v["type"].as_str(), Some("tableCell" | "tableHeader")) {
        format!(
            " colspan=\"{}\" rowspan=\"{}\"",
            v["attrs"]["colspan"].as_u64().unwrap_or(1),
            v["attrs"]["rowspan"].as_u64().unwrap_or(1)
        )
    } else {
        String::new()
    };
    format!("<{tag}{spans}>{inside}</{tag}>")
}
impl Core {
    fn import_office_package(
        &mut self,
        package: crate::office::Package,
        format: &str,
    ) -> Result<Value, String> {
        let mut n = self.workspace.clone();
        let id = format!(
            "package-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        if format == "docx" {
            let (doc, report) = crate::docx::import(&package)?;
            n["document"] = doc.clone();
            n["officeDocument"] =
                json!({"packageId":id,"sourceId":id,"baseline":doc,"report":report});
            n["officeDocument"]["recordingRequested"] =
                crate::docx_tracking::read_settings(&package)?["enabled"].clone();
        } else {
            let (sheets, report) = crate::xlsx::import(&package)?;
            n["sheets"] = sheets.clone();
            n["officeWorkbook"] =
                json!({"packageId":id,"sourceId":id,"baseline":sheets,"report":report});
        }
        validate(&n)?;
        self.packages.insert(id, Arc::new(package));
        self.calculator.invalidate();
        self.commit(n)
    }

    pub fn new() -> Self {
        let mut s = Self {
            workspace: seed(),
            ..Default::default()
        };
        s.sync(&ChangeSet::Structural);
        s
    }
    fn sync(&mut self, changes: &ChangeSet) {
        self.serialized_size = None;
        let from = self.epoch;
        self.epoch = next_epoch();
        let paths = self.pending_paths.take();
        let merged;
        let changes = match (self.catch_up.take(), changes) {
            (None, changes) => changes,
            (Some(mut cells), ChangeSet::Cells(own)) => {
                cells.extend(own.iter().cloned());
                merged = ChangeSet::Cells(cells);
                &merged
            }
            (Some(_), ChangeSet::Structural) => &ChangeSet::Structural,
        };
        let pkg = self.workspace["officeWorkbook"]["packageId"]
            .as_str()
            .and_then(|id| self.packages.get(id))
            .map(AsRef::as_ref);
        let (date1904, palette) = crate::number_format::settings(pkg);
        let hint = match changes {
            ChangeSet::Cells(cells) => Some(cells.as_slice()),
            ChangeSet::Structural => None,
        };
        stage("sync.calc", || {
            self.calculator
                .sync_changes(&self.workspace["sheets"], date1904, hint)
        });
        // Overlays depend on calculated values, which change only when the
        // calculator's generation or error state does, and on the rules.
        let conditional_state = stage(
            "sync.cf_state",
            || json!({"calculation":self.calculator.version(),"error":self.calculator.error,"rules":self.workspace["sheets"].as_array().unwrap().iter().map(|s|json!([s["id"],s["name"],s["conditionalFormats"],s["extendedConditionalFormats"],s["conditionalRightToLeft"]])).collect::<Vec<_>>()}),
        );
        let cf_started = std::time::Instant::now();
        // Cells whose overlay changed, or None when that can't be listed cheaply.
        let mut overlay_changes = Some(vec![]);
        let warnings_changed = conditional_state != self.conditional_state;
        if conditional_state != self.conditional_state {
            let mut overlays = serde_json::Map::new();
            let mut warnings = serde_json::Map::new();
            for (i, sheet) in self.workspace["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let id = sheet["id"].as_str().unwrap();
                // Without rules, evaluation reads no cells.
                let cells = if crate::conditional_format::rules(sheet).next().is_some() {
                    sheet["cells"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };
                match crate::conditional_format::evaluate(
                    &self.workspace["sheets"],
                    i,
                    &cells,
                    &mut self.calculator,
                ) {
                    Ok(result) => {
                        let styles = &result["styles"];
                        if let (Some(changes), Some(new), Some(old)) = (
                            overlay_changes.as_mut(),
                            styles.as_object(),
                            self.conditional.get(id).and_then(Value::as_object),
                        ) {
                            changes.extend(
                                new.iter()
                                    .filter(|(a, v)| old.get(*a) != Some(*v))
                                    .map(|(a, _)| (i, a.clone())),
                            );
                            changes.extend(
                                old.keys()
                                    .filter(|a| !new.contains_key(*a))
                                    .map(|a| (i, a.clone())),
                            );
                        } else {
                            overlay_changes = None;
                        }
                        overlays.insert(id.into(), styles.clone());
                        warnings.insert(id.into(), result["warnings"].clone());
                    }
                    Err(e) => {
                        overlay_changes = None;
                        warnings.insert(id.into(), json!([e]));
                    }
                }
            }
            self.conditional = Value::Object(overlays);
            self.conditional_warnings = Value::Object(warnings);
            self.conditional_state = conditional_state;
        }
        crate::perf_stages::record("sync.cf_eval", cf_started);
        // Only edited cells, recalculated cells and restyled cells can display
        // differently after a cell-value-only change.
        let display_cells = match (changes, &self.calculator.changed, overlay_changes) {
            (
                ChangeSet::Cells(edited),
                crate::calculation::Changed::Cells(values),
                Some(overlays),
            ) => {
                let mut cells: Vec<_> = edited
                    .iter()
                    .chain(values)
                    .chain(&overlays)
                    .cloned()
                    .collect();
                cells.sort();
                cells.dedup();
                Some(cells)
            }
            _ => None,
        };
        let delta_cells = display_cells.clone();
        stage("sync.display", || {
            let sheets = &self.workspace["sheets"];
            let calc = &mut self.calculator;
            let updated = display_cells.is_some_and(|cells| {
                calc.display.sync_cells(
                    sheets,
                    &calc.values,
                    &calc.cell_errors,
                    &self.conditional,
                    date1904,
                    &palette,
                    &cells,
                )
            });
            if !updated {
                calc.display.sync(
                    sheets,
                    &calc.values,
                    &calc.cell_errors,
                    calc.generation,
                    &self.conditional,
                    date1904,
                    palette,
                )
            }
        });
        if let Some(bundle) = &mut self.layout_bundle {
            match (&delta_cells, &mut bundle.lag) {
                (Some(cells), Some(lag)) if lag.len() + cells.len() <= 10_000 => {
                    lag.extend(cells.iter().cloned())
                }
                _ => bundle.lag = None,
            }
        }
        let unchanged = !warnings_changed
            && paths.as_ref().is_some_and(Vec::is_empty)
            && delta_cells.as_ref().is_some_and(Vec::is_empty);
        if unchanged {
            // Nothing published changed: keep the state's identity.
            self.epoch = from;
            return;
        }
        // Published as a delta only when both the workspace paths and the
        // affected cells are known.
        self.last_change = paths.zip(delta_cells).map(|(paths, cells)| LastChange {
            from,
            to: self.epoch,
            paths,
            cells,
            warnings: warnings_changed,
        });
    }
    #[cfg(test)]
    pub fn freeze_clock(&mut self, rfc3339: &str) {
        self.calculator.clock_override =
            Some(crate::calculation_clock::CalculationClock::fixed(rfc3339));
    }
    fn file_states(&mut self) -> Value {
        let mut packages = Vec::new();
        for key in ["officeDocument", "officeWorkbook"] {
            let id = self.workspace[key]["packageId"].as_str().unwrap_or("");
            let digest = self.packages.get(id).map(|package| {
                if let Some((weak, digest)) = self.package_digests.get(id) {
                    if weak.upgrade().is_some_and(|old| Arc::ptr_eq(&old, package)) {
                        return digest.clone();
                    }
                }
                let digest = crate::file_state::package_digest(package);
                self.package_digests
                    .insert(id.into(), (Arc::downgrade(package), digest.clone()));
                digest
            });
            packages.push(digest);
        }
        let doc = self.resolved_document();
        let docx = crate::file_state::digest(&(
            "docx-state-v1",
            doc.as_ref().unwrap_or(&self.workspace["document"]),
            doc.as_ref().err(),
            &self.workspace["officeDocument"]["baseline"],
            &packages[0],
        ));
        let xlsx = crate::file_state::digest(&(
            "xlsx-state-v1",
            &self.workspace["sheets"],
            &self.calculator.values,
            &self.calculator.cell_errors,
            &self.workspace["officeWorkbook"]["baseline"],
            &packages[1],
        ));
        let workspace = self
            .workspace
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, _)| key.as_str() != "revision")
            .collect::<std::collections::BTreeMap<_, _>>();
        let folio = crate::file_state::digest(&("folio-state-v1", workspace, packages));
        json!({"docx":docx,"xlsx":xlsx,"folio":folio})
    }
    pub fn snapshot(&mut self) -> Value {
        if self.quiet {
            return Value::Null;
        }
        if let Some(delta) = self.delta() {
            return delta;
        }
        let blank = self.is_blank();
        let file_states = stage("snapshot.file_states", || self.file_states());
        stage(
            "snapshot.build",
            || json!({"blankWorkspace":blank,"fileStates":file_states,"workspace":self.workspace,"calculated":self.calculator.values,"formatted":self.calculator.display.values,"conditionalWarnings":self.conditional_warnings,"date1904":self.calculator.display.date1904,"calculationError":self.calculator.error,"canUndo":self.history.can_undo(),"canRedo":self.history.can_redo(),"evaluatedCells":self.calculator.generation,"localCalendarDay":self.calculator.clock.map(|c|c.local_day),"localUtcOffset":self.calculator.clock.map(|c|c.offset_seconds),"rows":ROWS,"columns":COLS,"epoch":self.epoch}),
        )
    }
    /// The published state as changes from the state the caller holds, when
    /// the caller asked for that and the last sync recorded the difference.
    /// Omits `fileStates`, which callers fetch separately (`fileStates` op).
    fn delta(&self) -> Option<Value> {
        let change = self.last_change.as_ref()?;
        if self.want_delta != Some(change.from) || change.to != self.epoch {
            return None;
        }
        let path_json = |path: &[crate::json_patch::Key]| {
            Value::Array(
                path.iter()
                    .map(|k| match k {
                        crate::json_patch::Key::Field(f) => json!(f),
                        crate::json_patch::Key::Index(i) => json!(i),
                    })
                    .collect(),
            )
        };
        let mut seen = HashSet::new();
        let mut workspace = vec![json!({"path":["revision"],"value":self.workspace["revision"]})];
        for path in &change.paths {
            if !seen.insert(path) {
                continue;
            }
            workspace.push(match crate::json_patch::get(&self.workspace, path) {
                Some(value) => json!({"path":path_json(path),"value":value}),
                None => json!({"path":path_json(path)}),
            });
        }
        let mut calculated = serde_json::Map::new();
        let mut formatted = serde_json::Map::new();
        for (i, a) in &change.cells {
            let id = self.workspace["sheets"][*i]["id"].as_str()?;
            let value = |all: &Value| all[id].get(a).cloned().unwrap_or(Value::Null);
            calculated
                .entry(id)
                .or_insert_with(|| json!({}))
                .as_object_mut()?
                .insert(a.clone(), value(&self.calculator.values));
            formatted
                .entry(id)
                .or_insert_with(|| json!({}))
                .as_object_mut()?
                .insert(a.clone(), value(&self.calculator.display.values));
        }
        let mut set = json!({
            "canUndo":self.history.can_undo(),"canRedo":self.history.can_redo(),
            "evaluatedCells":self.calculator.generation,"calculationError":self.calculator.error,
            "date1904":self.calculator.display.date1904,
            "localCalendarDay":self.calculator.clock.map(|c|c.local_day),
            "localUtcOffset":self.calculator.clock.map(|c|c.offset_seconds),
            "blankWorkspace":self.is_blank(),"epoch":self.epoch,
        });
        if change.warnings {
            set["conditionalWarnings"] = self.conditional_warnings.clone();
        }
        Some(json!({"delta":{"base":change.from,"workspace":workspace,
            "calculated":calculated,"formatted":formatted,"set":set}}))
    }
    fn is_blank(&self) -> bool {
        self.workspace["title"] == "Untitled workspace"
            && self.workspace["document"] == json!({"type":"doc","content":[{"type":"paragraph"}]})
            && self.workspace["sheets"] == json!([{"id":"sheet-1","name":"Sheet 1","cells":{}}])
            && self.workspace.get("officeDocument").is_none()
            && self.workspace.get("officeWorkbook").is_none()
    }
    fn clipboard_package(&self) -> Result<Vec<u8>, String> {
        let pkg = self.workspace["officeWorkbook"]["packageId"]
            .as_str()
            .and_then(|id| self.packages.get(id))
            .map(AsRef::as_ref);
        crate::xlsx::export(
            &self.workspace["sheets"],
            &self.calculator.values,
            &self.calculator.cell_errors,
            pkg,
            self.workspace["officeWorkbook"].get("baseline"),
        )
    }
    fn resolved_document(&self) -> Result<Value, String> {
        let mut doc = self.workspace["document"].clone();
        fn walk(core: &Core, node: &mut Value) -> Result<(), String> {
            if node["type"] == "workbookBlock" {
                let a = node["attrs"].clone();
                let data = match core.read_range(
                    a["sheetId"].as_str().ok_or("Missing sheet")?,
                    a["range"].as_str().ok_or("Missing range")?,
                ) {
                    Ok(data) => data,
                    Err(_) => {
                        node["content"] = json!([{"type":"paragraph","content":[{"type":"text","text":"Workbook source unavailable: its worksheet or referenced cells were deleted. Restore the source or relink this block."}]}]);
                        return Ok(());
                    }
                };
                node["content"] = json!([{"type":"paragraph","content":[{"type":"text","text":format!("{} — {}!{}",a["title"].as_str().unwrap_or("Workbook data"),data["sheetName"].as_str().unwrap(),a["range"].as_str().unwrap())}]},{"type":"table","content":data["displayRows"].as_array().unwrap().iter().map(|r|json!({"type":"tableRow","content":r.as_array().unwrap().iter().map(|v|json!({"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":display_value(v)}]}]})).collect::<Vec<_>>()})).collect::<Vec<_>>()}]);
            }
            if let Some(children) = node.get_mut("content").and_then(Value::as_array_mut) {
                for n in children {
                    walk(core, n)?
                }
            }
            Ok(())
        }
        walk(self, &mut doc)?;
        Ok(doc)
    }
    fn read_range(&self, id: &str, range: &str) -> Result<Value, String> {
        let sheet = self.workspace["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .ok_or("Source worksheet no longer exists")?;
        let ends = range.to_uppercase();
        let ends: Vec<_> = ends.split(':').collect();
        if ends.len() > 2 {
            return Err("Use a cell or rectangular range such as A1:D9".into());
        }
        let (r1, c1) = parse_address(ends[0])?;
        let (r2, c2) = parse_address(ends.last().unwrap())?;
        if r2 < r1 || c2 < c1 || (r2 - r1 + 1) * (c2 - c1 + 1) > 2000 {
            return Err("Select a rectangular range of at most 2,000 cells".into());
        }
        let rows = (r1..=r2)
            .map(|r| {
                (c1..=c2)
                    .map(|c| self.calculator.values[id][address(r, c)].clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let display_rows = (r1..=r2)
            .map(|r| {
                (c1..=c2)
                    .map(|c| {
                        self.calculator.display.values[id][address(r, c)]["text"]
                            .as_str()
                            .unwrap_or("")
                            .to_string()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        Ok(
            json!({"sheetId":id,"sheetName":sheet["name"],"range":range,"revision":self.workspace["revision"],"rows":rows,"displayRows":display_rows}),
        )
    }
    fn materialize_document_styles(&self, next: &mut Value) -> Result<(), String> {
        document_validate(&next["document"], 0, &mut 0)?;
        crate::docx_paragraph::normalize_outline(
            &mut next["document"],
            &self.workspace["document"],
        );
        let native;
        let source = if let Some(pkg) = next["officeDocument"]["packageId"]
            .as_str()
            .and_then(|id| self.packages.get(id))
        {
            pkg.as_ref()
        } else {
            native = crate::docx::new_package();
            &native
        };
        let catalog = next["document"]["attrs"]
            .get(crate::docx_styles::KEY)
            .cloned()
            .unwrap_or(crate::docx_styles::read(source)?);
        fn has_outline(n: &Value) -> bool {
            n["attrs"]["paragraphFormat"]["direct"]
                .get("outlineLevel")
                .is_some()
                || n["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(has_outline)
        }
        if next["document"]["attrs"]
            .get(crate::docx_styles::KEY)
            .is_none()
            && has_outline(&next["document"])
        {
            // Native paragraphs gain a stable visual-style identity on the
            // first outline edit; later heading commands can resolve it.
            next["document"]["attrs"][crate::docx_styles::KEY] = catalog.clone();
        }
        if !crate::builtin_styles::needed(&next["document"], &catalog) {
            return Ok(());
        }
        let mut pkg = source.clone();
        crate::builtin_styles::materialize_document(&mut next["document"], &mut pkg)
    }
    fn commit(&mut self, next: Value) -> Result<Value, String> {
        self.commit_document(next, None)
    }
    fn commit_document(&mut self, mut next: Value, exact: Option<&Value>) -> Result<Value, String> {
        if self.workspace["officeDocument"]["packageId"] == next["officeDocument"]["packageId"]
            && self.workspace["document"]["attrs"].get(crate::docx_page_setup::KEY)
                != next["document"]["attrs"].get(crate::docx_page_setup::KEY)
        {
            return Err("Use Layout > Page setup to change document page settings".into());
        }
        if self.workspace["officeDocument"]["packageId"] == next["officeDocument"]["packageId"]
            && self.workspace["officeDocument"]["recordingRequested"] == true
            && self.workspace["officeDocument"].get("tracking").is_none()
            && self.workspace["document"] != next["document"]
        {
            return Err("This Word file requests Track Changes. Choose a review author in Recording settings before editing.".into());
        }
        let journal = crate::document_history::Journal::new(
            &self.workspace["document"],
            &next["document"],
            exact,
        )?;
        if self.workspace["officeDocument"]["packageId"] == next["officeDocument"]["packageId"]
            && self.workspace["officeDocument"].get("tracking").is_some()
        {
            if self.workspace["officeDocument"]["tracking"] != next["officeDocument"]["tracking"] {
                return Err("Use Track Changes settings to change recording mode".into());
            }
            if self.workspace["document"] != next["document"] {
                validate(&next)?;
                crate::docx_tracking::check_bindings(&next["document"])?;
                let exact = &journal
                    .as_ref()
                    .ok_or("Tracked text changes require an exact edit journal")?
                    .forward;
                let settings = &next["officeDocument"]["tracking"];
                let author = crate::docx_track_edit::Author {
                    name: settings["author"].as_str().unwrap().into(),
                    date: chrono::Utc::now().to_rfc3339(),
                    enabled: settings["enabled"].as_bool().unwrap(),
                    formatting: settings["formatting"].as_bool().unwrap(),
                };
                let source = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .ok_or("Tracked source package is missing")?;
                let updated = crate::docx_tracking::edit(
                    source,
                    &self.workspace["document"],
                    &next["document"],
                    exact,
                    &author,
                )?;
                let id = format!(
                    "package-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                next["officeDocument"]["baseline"] = next["document"].clone();
                crate::file_state::replace_package(&mut next, "officeDocument", &id);
                validate(&next)?;
                self.packages.insert(id, Arc::new(updated));
            }
        }
        if self.workspace["officeDocument"]["packageId"] == next["officeDocument"]["packageId"] {
            crate::docx_revisions::check_model(&self.workspace["document"], &next["document"])?;
            crate::docx_page_setup::check_model(&self.workspace["document"], &next["document"])?;
        }
        self.commit_journal(next, journal)
    }
    fn commit_journal(
        &mut self,
        mut next: Value,
        document: Option<crate::document_history::Journal>,
    ) -> Result<Value, String> {
        stage("validate", || validate(&next))?;
        next["revision"] = json!(self.workspace["revision"].as_u64().unwrap() + 1);
        let previous = std::mem::replace(&mut self.workspace, next);
        let back = stage("undo.diff", || {
            crate::json_patch::diff(&self.workspace, &previous).without_field("revision")
        });
        let changes = ChangeSet::from_patch(&back);
        self.pending_paths = Some(back.paths());
        stage("undo.push", || {
            self.history
                .record(crate::document_history::Entry::new(back, document))
        });
        stage("undo.drop", || drop(previous));
        self.sync(&changes);
        Ok(self.snapshot())
    }
    /// Applies plain cell edits in place: no workspace copy, no whole-workspace
    /// validation or diff. Returns `Ok(None)`, having changed nothing, when
    /// the edit needs the general path (tables, or document normalization
    /// that would change the document). Errors match the general path's.
    fn commit_cells(&mut self, changes: &Value) -> Result<Option<Value>, String> {
        match self.apply_cells(changes)? {
            Some((back, size)) => self.finish_cells(back, size).map(Some),
            None => Ok(None),
        }
    }
    /// The first half of `commit_cells`: edits the cells in place and returns
    /// the undo patch and the new serialized size, without committing.
    fn apply_cells(
        &mut self,
        changes: &Value,
    ) -> Result<Option<(crate::json_patch::Patch, usize)>, String> {
        #[cfg(test)]
        if self.slow_cells {
            return Ok(None);
        }
        let sheets = self.workspace["sheets"].as_array().ok_or("No sheets")?;
        if sheets
            .iter()
            .any(|s| crate::tables::list(s).next().is_some())
        {
            return Ok(None);
        }
        // Same checks, in the same order, as `proposed`.
        let changes = changes.as_array().ok_or("Invalid cell changes")?;
        if changes.len() > 10000 {
            return Err("Too many cell changes".into());
        }
        let mut edits = Vec::with_capacity(changes.len());
        for c in changes {
            let a = text(c, "address")?.to_uppercase();
            parse_address(&a)?;
            let value = text(c, "value")?;
            if value.len() > 10000 {
                return Err("Cell too long".into());
            }
            let id = text(c, "sheetId")?;
            let i = sheets
                .iter()
                .position(|s| s["id"] == id)
                .ok_or("Sheet does not exist")?;
            edits.push((i, a, value.to_string()));
        }
        // `proposed` re-normalizes the document on every edit; if that would
        // change it, the general path must commit the change.
        let cached = self
            .stable_document
            .as_ref()
            .is_some_and(|(document, office)| {
                *document == self.workspace["document"]
                    && *office == self.workspace["officeDocument"]
            });
        let document = cached
            || stage("cells.document", || -> Result<bool, String> {
                let mut next = json!({
                    "document": self.workspace["document"],
                    "officeDocument": self.workspace["officeDocument"],
                });
                self.materialize_document_styles(&mut next)?;
                crate::document_bookmarks::rebase(
                    &self.workspace["document"],
                    &mut next["document"],
                    None,
                )?;
                crate::document_comments::rebase(
                    &self.workspace["document"],
                    &mut next["document"],
                    None,
                )?;
                crate::docx_revisions::check_model(&self.workspace["document"], &next["document"])?;
                crate::docx_page_setup::check_model(
                    &self.workspace["document"],
                    &next["document"],
                )?;
                Ok(next["document"] == self.workspace["document"])
            })?;
        if document && !cached {
            self.stable_document = Some((
                self.workspace["document"].clone(),
                self.workspace["officeDocument"].clone(),
            ));
        }
        if !document {
            return Ok(None);
        }
        let size = match self.serialized_size {
            Some(size) => size,
            None => stage("cells.size", || self.workspace.to_string().len()),
        };
        self.serialized_size = Some(size);
        let json_len = |v: &Value| serde_json::to_string(v).map_or(0, |s| s.len());
        let mut grown: isize = 0;
        let mut back = Vec::with_capacity(edits.len());
        for (i, a, value) in edits {
            let cells = self.workspace["sheets"][i]["cells"]
                .as_object_mut()
                .ok_or("Invalid cells")?;
            let value = Value::String(value);
            let value_len = json_len(&value) as isize;
            let path = vec![
                crate::json_patch::Key::Field("sheets".into()),
                crate::json_patch::Key::Index(i),
                crate::json_patch::Key::Field("cells".into()),
                crate::json_patch::Key::Field(a.clone()),
            ];
            let key_len = json_len(&json!(a)) as isize;
            match cells.insert(a, value) {
                Some(old) => {
                    grown += value_len - json_len(&old) as isize;
                    back.push(crate::json_patch::Op::Set { path, value: old });
                }
                None => {
                    // `"key":value`, plus a comma unless the object was empty.
                    grown += key_len + 1 + value_len + (cells.len() > 1) as isize;
                    back.push(crate::json_patch::Op::Remove { path });
                }
            }
        }
        back.reverse();
        let back = crate::json_patch::Patch { ops: back };
        let size = (size as isize + grown) as usize;
        let too_many = back.ops.iter().any(|op| {
            let (Op::Set { path, .. } | Op::Remove { path }) = op;
            matches!(path[1], crate::json_patch::Key::Index(i)
                if self.workspace["sheets"][i]["cells"].as_object().is_some_and(|c| c.len() > 100000))
        });
        let error = if size > MAX_SIZE {
            Some("Workspace exceeds 32 MB")
        } else if too_many {
            Some("Too many populated cells")
        } else {
            None
        };
        if let Some(error) = error {
            crate::json_patch::apply_inverse(&mut self.workspace, back)?;
            return Err(error.into());
        }
        Ok(Some((back, size)))
    }
    fn finish_cells(
        &mut self,
        back: crate::json_patch::Patch,
        size: usize,
    ) -> Result<Value, String> {
        let revision = self.workspace["revision"].as_u64().unwrap();
        let size = size + (revision + 1).to_string().len() - revision.to_string().len();
        self.workspace["revision"] = json!(revision + 1);
        let changes = ChangeSet::from_patch(&back);
        self.pending_paths = Some(back.paths());
        let journal = crate::document_history::Journal {
            forward: json!([]),
            reverse: json!([]),
        };
        stage("undo.push", || {
            self.history
                .record(crate::document_history::Entry::new(back, Some(journal)))
        });
        self.sync(&changes);
        #[cfg(test)]
        assert_eq!(
            size,
            self.workspace.to_string().len(),
            "tracked workspace size"
        );
        self.serialized_size = Some(size);
        Ok(self.snapshot())
    }
    /// A staged engine's calculator and conditional-format state, for reuse
    /// by the next staged edit. Its workspace is `self.workspace`.
    fn into_bundle(self) -> LayoutBundle {
        LayoutBundle {
            calculator: self.calculator,
            conditional: (
                self.conditional,
                self.conditional_state,
                self.conditional_warnings,
            ),
            lag: None,
        }
    }
    /// Commits a staged layout edit from its undo patch: applies the measured
    /// row sizes, checks only what they can change (the size limit), and
    /// records one history entry, without re-validating or re-diffing the
    /// whole workspace. Same result as `commit_journal` on the staged
    /// workspace.
    fn commit_staged(
        &mut self,
        pending: LayoutEdit,
        entry: crate::document_history::Entry,
        sizes: &Value,
    ) -> Result<Value, String> {
        let LayoutEdit {
            rows,
            staged,
            staged_base,
            candidates,
            ..
        } = pending;
        let mut staged = *staged;
        // The staged edit's recorded change is relative to the state the
        // staged engine caught up to, which is this engine's published state
        // when both calculate on the same calendar day.
        let staged_change = staged
            .last_change
            .clone()
            .filter(|c| Some(c.from) == staged_base);
        // Row sizes: record each touched key's old value for the undo patch.
        let json_len = |v: &Value| serde_json::to_string(v).map_or(0, |s| s.len()) as isize;
        let mut row_back = vec![];
        let mut grown: isize = 0;
        let mut before = vec![];
        for (i, sheet) in staged.workspace["sheets"]
            .as_array()
            .ok_or("No sheets")?
            .iter()
            .enumerate()
        {
            if sheet["id"].as_str().is_some_and(|id| rows.contains_key(id)) {
                let fields = ["rowHeights", "rowHeightModes"].map(|f| sheet.get(f).cloned());
                before.push((i, fields));
            }
        }
        crate::row_reflow::apply(&mut staged.workspace, &rows, sizes)?;
        for (i, fields) in before {
            let sheet = &staged.workspace["sheets"][i];
            for (field, old) in ["rowHeights", "rowHeightModes"].into_iter().zip(fields) {
                let new = &sheet[field];
                let path = |key: Option<&str>| {
                    let mut p = vec![
                        crate::json_patch::Key::Field("sheets".into()),
                        crate::json_patch::Key::Index(i),
                        crate::json_patch::Key::Field(field.into()),
                    ];
                    p.extend(key.map(|k| crate::json_patch::Key::Field(k.into())));
                    p
                };
                match &old {
                    Some(old) if old.is_object() => {
                        grown += json_len(new) - json_len(old);
                        let old = old.as_object().unwrap();
                        for (key, value) in new.as_object().into_iter().flatten() {
                            match old.get(key) {
                                Some(v) if v == value => {}
                                Some(v) => row_back.push(Op::Set {
                                    path: path(Some(key)),
                                    value: v.clone(),
                                }),
                                None => row_back.push(Op::Remove {
                                    path: path(Some(key)),
                                }),
                            }
                        }
                    }
                    // `apply` replaced a missing or non-object field.
                    _ => {
                        grown += json_len(new)
                            // A new `"field":{…}` member, plus its comma.
                            - old.as_ref().map_or(-4 - field.len() as isize, json_len);
                        row_back.push(match old {
                            Some(v) => Op::Set {
                                path: path(None),
                                value: v,
                            },
                            None => Op::Remove { path: path(None) },
                        });
                    }
                }
            }
        }
        let size = match staged.serialized_size {
            Some(size) => (size as isize + grown) as usize,
            None => staged.workspace.to_string().len(),
        };
        if size > MAX_SIZE {
            return Err("Workspace exceeds 32 MB".into());
        }
        let mut back = row_back;
        back.extend(entry.patch.ops);
        let back = crate::json_patch::Patch { ops: back };
        self.packages = std::mem::take(&mut staged.packages);
        self.package_digests = std::mem::take(&mut staged.package_digests);
        let workspace =
            std::mem::replace(&mut self.workspace, std::mem::take(&mut staged.workspace));
        // Dropping a large workspace is linear; don't make the edit wait for it.
        std::thread::spawn(move || drop(workspace));
        let conditional = (
            std::mem::replace(
                &mut self.conditional,
                std::mem::take(&mut staged.conditional),
            ),
            std::mem::replace(
                &mut self.conditional_state,
                std::mem::take(&mut staged.conditional_state),
            ),
            std::mem::replace(
                &mut self.conditional_warnings,
                std::mem::take(&mut staged.conditional_warnings),
            ),
        );
        // The previous calculator now lags by exactly the staged changes.
        self.layout_bundle = Some(LayoutBundle {
            calculator: std::mem::replace(&mut self.calculator, staged.calculator),
            conditional,
            lag: candidates,
        });
        let changes = ChangeSet::from_patch(&back);
        self.pending_paths = Some(back.paths());
        self.history
            .record(crate::document_history::Entry::new(back, entry.document));
        self.sync(&changes);
        // The swapped-in calculator already holds the edit's results, so this
        // sync saw no value changes; the staged edit's own sync recorded them.
        match (&mut self.last_change, staged_change) {
            (Some(change), Some(staged)) => {
                change.cells.extend(staged.cells);
                change.cells.sort();
                change.cells.dedup();
                change.warnings = true;
            }
            (change, _) => *change = None,
        }
        self.serialized_size = Some(size);
        #[cfg(test)]
        assert_eq!(
            size,
            self.workspace.to_string().len(),
            "tracked workspace size"
        );
        Ok(self.snapshot())
    }
    #[cfg(test)]
    pub(crate) fn snapshot_for_test(&mut self) -> Value {
        self.snapshot()
    }
    #[cfg(test)]
    pub(crate) fn layout_token(&self) -> Value {
        json!(self.layout.as_ref().map(|p| p.token))
    }
    fn sheet_mut<'a>(w: &'a mut Value, id: &str) -> Result<&'a mut Value, String> {
        w["sheets"]
            .as_array_mut()
            .ok_or("No sheets")?
            .iter_mut()
            .find(|s| s["id"] == id)
            .ok_or("Sheet does not exist".into())
    }
    fn proposed(&self, p: &Value) -> Result<Value, String> {
        if p["baseRevision"] != self.workspace["revision"] {
            return Err("The workspace has changed. Ask for an updated proposal.".into());
        }
        if let Some(id) = p.get("_workspaceId") {
            if id != &self.workspace["id"] {
                return Err("This proposal belongs to a different workspace".into());
            }
        }
        text(p, "summary")?;
        let mut n = self.workspace.clone();
        for key in ["replaceDocument", "appendParagraphs"] {
            if let Some(a) = p.get(key) {
                let a = a.as_array().ok_or("Invalid paragraphs")?;
                if a.len() > 200 {
                    return Err("Too many paragraphs".into());
                }
                let mut nodes = vec![];
                for t in a {
                    let t = t.as_str().ok_or("Invalid paragraph")?;
                    if t.len() > 30000 {
                        return Err("Paragraph too long".into());
                    }
                    nodes.push(json!({"type":"paragraph","content":[{"type":"text","text":t}]}));
                }
                if key == "replaceDocument" {
                    if nodes.is_empty() {
                        nodes.push(json!({"type":"paragraph"}));
                    }
                    n["document"]["content"] = json!(nodes);
                } else {
                    n["document"]["content"]
                        .as_array_mut()
                        .ok_or("Invalid document")?
                        .extend(nodes);
                }
            }
        }
        if let Some(changes) = p.get("cells") {
            let changes = changes.as_array().ok_or("Invalid cell changes")?;
            if changes.len() > 10000 {
                return Err("Too many cell changes".into());
            }
            for c in changes {
                let a = text(c, "address")?.to_uppercase();
                parse_address(&a)?;
                let value = text(c, "value")?;
                if value.len() > 10000 {
                    return Err("Cell too long".into());
                }
                Self::sheet_mut(&mut n, text(c, "sheetId")?)?["cells"][&a] = json!(value);
            }
            crate::tables::after_cells(&self.workspace, &mut n, changes)?;
        }
        if let Some(document) = p.get("document") {
            n["document"] = document_with_settings(document, &n["document"])?;
        }
        if let Some(link) = p.get("linkedBlock") {
            self.read_range(text(link, "sheetId")?, text(link, "range")?)?;
            n["document"]["content"]
                .as_array_mut()
                .ok_or("Invalid document")?
                .push(json!({"type":"workbookBlock","attrs":link}));
        }
        self.materialize_document_styles(&mut n)?;
        if p.get("replaceDocument").is_some()
            && !crate::document_bookmarks::entries(&self.workspace["document"]).is_empty()
        {
            n["document"]["attrs"]["bookmarks"] = json!([]);
        } else {
            crate::document_bookmarks::rebase(
                &self.workspace["document"],
                &mut n["document"],
                p.get("textEdits"),
            )?;
        }
        if p.get("replaceDocument").is_some() {
            if self.workspace["document"]["attrs"]
                .get("comments")
                .is_some()
            {
                n["document"]["attrs"]["comments"] = json!([]);
            }
        } else {
            crate::document_comments::rebase(
                &self.workspace["document"],
                &mut n["document"],
                p.get("textEdits"),
            )?;
        }
        validate(&n)?;
        Ok(n)
    }
    fn scenario_scope(&self, r: &Value) -> Result<usize, String> {
        if r["workspaceId"] != self.workspace["id"]
            || r["baseRevision"] != self.workspace["revision"]
        {
            return Err("The workbook changed. Reopen Scenario Manager.".into());
        }
        crate::scenarios::sheet_index(&self.workspace, r)
    }
    fn scenario_next(&self, r: &Value) -> Result<Value, String> {
        let si = self.scenario_scope(r)?;
        let sheet = &self.workspace["sheets"][si];
        let pkg = self.workspace["officeWorkbook"]["packageId"]
            .as_str()
            .and_then(|id| self.packages.get(id))
            .map(AsRef::as_ref);
        crate::scenarios::check_sheet(sheet, pkg)?;
        let scenario = crate::scenarios::selected(sheet, r)?;
        let mut edits = vec![];
        for input in scenario["inputs"].as_array().unwrap() {
            if input["deleted"] == true {
                continue;
            }
            crate::goal_seek::check_scenario_input(
                &self.workspace,
                &self.calculator,
                pkg,
                &r["sheetId"],
                &input["address"],
            )?;
            let a = input["address"].as_str().unwrap();
            if sheet["cells"][a]
                .as_str()
                .is_some_and(|v| v.starts_with('='))
                && r["replaceFormulas"] != true
            {
                return Err("This scenario replaces formula cells. Review and explicitly allow that change.".into());
            }
            edits.push(
                json!({"sheetId":r["sheetId"],"address":input["address"],"value":input["value"]}),
            );
        }
        if edits.is_empty() {
            return Err("This scenario has no remaining changing cells".into());
        }
        let mut next = self.proposed(
            &json!({"summary":"Show scenario","baseRevision":r["baseRevision"],"cells":edits}),
        )?;
        for edit in &edits {
            let checked = crate::data_validation::check(&next, edit, self.calculator.date1904)?;
            if checked["valid"] != true && checked["shouldAlert"] == true {
                return Err(format!(
                    "{}: {}",
                    edit["address"].as_str().unwrap(),
                    checked["reason"]
                        .as_str()
                        .unwrap_or("The scenario value does not match cell validation")
                ));
            }
        }
        next["sheets"][si]["scenarios"]["current"] = r["index"].clone();
        next["sheets"][si]["scenarios"]["show"] = r["index"].clone();
        Ok(next)
    }
    fn scenario_preflight(&self, next: &Value) -> Result<(), String> {
        if let Some(pkg) = self.workspace["officeWorkbook"]["packageId"]
            .as_str()
            .and_then(|id| self.packages.get(id))
        {
            let mut check = (**pkg).clone();
            for (sheet, before) in next["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .zip(self.workspace["sheets"].as_array().unwrap())
            {
                if let Some(path) = sheet["officePath"].as_str() {
                    crate::scenarios::write(&mut check, sheet, Some(before), path)?;
                }
            }
        }
        Ok(())
    }
    fn scenario_summary(&self, r: &Value) -> Result<Value, String> {
        let si = self.scenario_scope(r)?;
        let sheet = &self.workspace["sheets"][si];
        let scenarios = crate::scenarios::items(sheet).collect::<Vec<_>>();
        if scenarios.is_empty() {
            return Err("Add a scenario before creating a summary".into());
        }
        let results = crate::scenarios::addresses(text(r, "resultCells")?, 32)?;
        let mut inputs = vec![];
        for item in &scenarios {
            for input in item["inputs"]
                .as_array()
                .ok_or("Unsupported scenario inputs")?
            {
                if input["deleted"] != true {
                    let a = input["address"]
                        .as_str()
                        .ok_or("Invalid changing cell")?
                        .to_string();
                    if !inputs.contains(&a) {
                        inputs.push(a);
                    }
                }
            }
        }
        let rows = inputs.len() + results.len() + 6;
        if (scenarios.len() + 2) * rows > 100000 || scenarios.len() + 2 > 16384 {
            return Err("The scenario summary exceeds the workbook's current size limit".into());
        }
        let sid = sheet["id"].as_str().unwrap();
        let mut columns = vec![(
            "Current values".to_string(),
            self.calculator.values[sid].clone(),
            self.calculator.cell_errors[sid].clone(),
        )];
        for (index, item) in scenarios.iter().enumerate() {
            let mut request = r.clone();
            request["index"] = json!(index);
            request["replaceFormulas"] = json!(true);
            let next = self.scenario_next(&request)?;
            let mut isolated = Core {
                workspace: next,
                packages: self.packages.clone(),
                ..Default::default()
            };
            isolated.calculator.clock_override = self.calculator.clock;
            isolated.sync(&ChangeSet::Structural);
            if let Some(error) = &isolated.calculator.error {
                return Err(error.clone());
            }
            columns.push((
                item["name"].as_str().unwrap().into(),
                isolated.calculator.values[sid].clone(),
                isolated.calculator.cell_errors[sid].clone(),
            ));
        }
        let mut cells = serde_json::Map::new();
        let mut styles = serde_json::Map::new();
        let literal = |s: &str| format!("'{s}");
        cells.insert("A1".into(), json!("Scenario Summary"));
        cells.insert(
            "A2".into(),
            json!(format!(
                "Static values for {}",
                sheet["name"].as_str().unwrap()
            )),
        );
        cells.insert("A4".into(), json!("Changing cells"));
        let result_row = 4 + inputs.len() + 1;
        cells.insert(address(result_row, 0), json!("Result cells"));
        for (offset, a) in inputs.iter().enumerate() {
            cells.insert(address(4 + offset, 0), json!(literal(a)));
        }
        for (offset, a) in results.iter().enumerate() {
            cells.insert(address(result_row + 1 + offset, 0), json!(literal(a)));
        }
        for (col, (name, values, errors)) in columns.iter().enumerate() {
            cells.insert(address(2, col + 1), json!(literal(name)));
            styles.insert(address(2, col + 1), json!({"bold":true}));
            for (start, refs) in [(4, &inputs), (result_row + 1, &results)] {
                for (offset, a) in refs.iter().enumerate() {
                    let v = &values[a];
                    let value = match v {
                        Value::String(s) => {
                            if errors[a] == true {
                                s.clone()
                            } else {
                                literal(s)
                            }
                        }
                        Value::Bool(b) => {
                            if *b {
                                "TRUE".into()
                            } else {
                                "FALSE".into()
                            }
                        }
                        Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    let address = address(start + offset, col + 1);
                    cells.insert(address.clone(), json!(value));
                    if let Some(style) = sheet["cellStyles"].get(a) {
                        styles.insert(
                            address,
                            Value::Object(
                                style
                                    .as_object()
                                    .unwrap()
                                    .iter()
                                    .filter(|(k, _)| !k.starts_with('_'))
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                            ),
                        );
                    }
                }
            }
        }
        let mut next = self.workspace.clone();
        let sheets = next["sheets"].as_array_mut().unwrap();
        if sheets.len() >= 30 {
            return Err("Maximum 30 worksheets".into());
        }
        let name = (1..)
            .map(|n| {
                if n == 1 {
                    "Scenario Summary".into()
                } else {
                    format!("Scenario Summary {n}")
                }
            })
            .find(|n| {
                !sheets.iter().any(|s| {
                    s["name"]
                        .as_str()
                        .is_some_and(|s| s.eq_ignore_ascii_case(n))
                })
            })
            .unwrap();
        let id = format!(
            "scenario-summary-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        sheets[si]["scenarios"]["resultCells"] = json!(results.join(" "));
        sheets.push(json!({"id":id,"name":name,"cells":cells,"cellStyles":styles,"columnWidths":{"1":24,"2":22},"frozenPane":{"state":"frozen","rows":3,"columns":1,"topLeftCell":"B4"}}));
        validate(&next)?;
        Ok(next)
    }
    /// Handles one request. A request with `"delta": <epoch>` (the epoch of
    /// the state the caller holds) may get `{"delta": …}` instead of a full
    /// snapshot; see `delta`.
    pub fn dispatch(&mut self, r: Value) -> Result<Value, String> {
        self.want_delta = r.get("delta").and_then(Value::as_u64);
        let result = self.dispatch_op(r);
        self.want_delta = None;
        result
    }
    fn dispatch_op(&mut self, r: Value) -> Result<Value, String> {
        let op = text(&r, "op")?;
        // Loading a file may reuse the same workspace id/revision while changing
        // its source package. Invalidate staged work on every intervening write,
        // rather than trusting revision equality alone.
        if !matches!(
            op,
            "prepareLayoutEdit"
                | "commitLayoutEdit"
                | "cancelLayoutEdit"
                | "get"
                | "fileStates"
                | "documentRevisions"
                | "documentReviewView"
                | "documentPageSetup"
                | "documentSectionContext"
                | "documentHeaders"
                | "documentPagePreview"
                | "copyCells"
                | "previewNumberFormat"
                | "validationEntryCheck"
                | "validationChoices"
                | "conditionalRange"
                | "conditionalCheck"
                | "formulaAudit"
                | "scenarioCapture"
                | "inspectScenarioSource"
                | "closeScenarioSource"
                | "goalSeekStart"
                | "goalSeekStep"
                | "goalSeekStop"
                | "goalSeekCancel"
                | "filterChoices"
                | "propose"
                | "save"
                | "saveEmptyWorkspace"
                | "inspectWorkspace"
                | "inspectTemplate"
                | "exportHtml"
                | "exportCsv"
                | "writeExport"
                | "readRange"
                | "exportOffice"
                | "context"
        ) {
            if !(op == "scenarios" && r["action"] == "mergeExternal") {
                self.scenario_source = None;
            }
            if let Some(previous) = self.layout.take() {
                self.layout_bundle = Some(previous.staged.into_bundle());
            }
        }
        match op {
            "prepareLayoutEdit" => {
                let goal_seek = if r["edit"]["op"] == "goalSeekAccept" {
                    let search = self
                        .goal_seek
                        .as_ref()
                        .ok_or("Goal Seek result expired. Run it again.")?;
                    search.check(
                        &self.workspace,
                        self.calculator.generation,
                        &r["edit"]["token"],
                    )?;
                    Some(search.accept(&r["edit"])?)
                } else {
                    None
                };
                let edit = goal_seek.as_ref().map_or(&r["edit"], |g| &g.edit);
                if !matches!(
                    edit["op"].as_str(),
                    Some(
                        "cells"
                            | "scenarioShow"
                            | "scenarioSummary"
                            | "recalculate"
                            | "refreshCalendar"
                            | "enterCell"
                            | "apply"
                            | "formatCells"
                            | "conditionalFormat"
                            | "formatCellText"
                            | "fillCells"
                            | "pasteCells"
                            | "sheetGeometry"
                            | "structure"
                            | "table"
                            | "sortRange"
                            | "autoFilter"
                    )
                ) {
                    return Err("This operation cannot be staged for row layout".into());
                }
                if let Some(previous) = self.layout.take() {
                    self.layout_bundle = Some(previous.staged.into_bundle());
                }
                let bundle = self.layout_bundle.take();
                #[cfg(test)]
                let bundle = bundle.filter(|_| !self.slow_layout);
                let (calculator, conditional, lag) = match bundle {
                    Some(b) => (b.calculator, Some(b.conditional), b.lag),
                    None => (Default::default(), None, None),
                };
                let mut staged = Box::new(Core {
                    workspace: stage("layout.clone", || self.workspace.clone()),
                    packages: self.packages.clone(),
                    package_digests: self.package_digests.clone(),
                    clipboard: self.clipboard.clone(),
                    calculator,
                    serialized_size: self.serialized_size,
                    stable_document: self.stable_document.clone(),
                    ..Default::default()
                });
                #[cfg(test)]
                {
                    staged.slow_cells = self.slow_cells;
                    staged.slow_layout = self.slow_layout;
                }
                if let Some((c, state, warnings)) = conditional {
                    staged.conditional = c;
                    staged.conditional_state = state;
                    staged.conditional_warnings = warnings;
                }
                let calculation_only =
                    matches!(edit["op"].as_str(), Some("recalculate" | "refreshCalendar"));
                let clock_override = self.calculator.clock_override;
                staged.calculator.clock_override = Some(
                    clock_override
                        .unwrap_or_else(crate::calculation_clock::CalculationClock::system),
                );
                // Plain cell edits don't read calculated values before they
                // sync, so the staged calculator can catch up and apply the
                // edit in one evaluation. Other edits catch up first.
                let deferred = lag.is_some()
                    && !calculation_only
                    && goal_seek.is_none()
                    && matches!(edit["op"].as_str(), Some("cells" | "enterCell"));
                #[cfg(test)]
                let deferred = deferred && !self.slow_layout;
                if deferred {
                    staged.catch_up = lag;
                } else {
                    stage("layout.catch_up", || {
                        staged.sync(&lag.map_or(ChangeSet::Structural, ChangeSet::Cells))
                    });
                }
                staged.calculator.generation = self.calculator.generation;
                if calculation_only {
                    // A staged calculator may have been reused from a cancelled edit.
                    // Always evaluate against this refresh's captured instant.
                    staged.calculator.invalidate();
                }
                let edit_base = staged.epoch;
                if !goal_seek.as_ref().is_some_and(|g| g.unchanged) {
                    // Its own response is discarded; don't build a snapshot.
                    staged.quiet = true;
                    let edited = staged.dispatch(edit.clone());
                    staged.quiet = false;
                    edited?;
                }
                if staged.catch_up.is_some() {
                    // The edit didn't sync; catch up now.
                    staged.sync(&ChangeSet::Cells(vec![]));
                }
                if deferred {
                    // As when catching up first, the count advances only if
                    // the edit itself changed an input.
                    let edit_changed = staged.history.last_undo().is_some_and(|entry| {
                        entry.patch.paths().iter().any(|path| {
                            crate::json_patch::get(&staged.workspace, path)
                                != crate::json_patch::get(&self.workspace, path)
                        })
                    });
                    staged.calculator.generation = self.calculator.generation + edit_changed as u64;
                }
                if let Some(goal) = &goal_seek {
                    goal.check(&staged.calculator)?;
                }
                staged.calculator.clock_override = clock_override;
                let same_calendar = matches!(
                    (staged.calculator.clock, self.calculator.clock),
                    (Some(a), Some(b)) if a.same_calendar(b)
                );
                // The staged edit's recorded change is relative to `edit_base`:
                // this engine's state, or with a deferred catch-up the lagging
                // state. The lag (every cell that differs between those two)
                // was merged into that sync's cells, so either way the record
                // covers every difference from this engine's state.
                let staged_base = same_calendar.then_some(edit_base);
                let pkg = staged.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| staged.packages.get(id))
                    .map(AsRef::as_ref);
                let candidates = staged
                    .last_change
                    .as_mut()
                    .filter(|c| Some(c.from) == staged_base)
                    .map(|c| {
                        let mut cells = c.cells.clone();
                        cells.extend(c.paths.iter().filter_map(|path| match path.as_slice() {
                            [Key::Field(s), Key::Index(i), Key::Field(f), Key::Field(a)]
                                if s == "sheets" && f == "cells" =>
                            {
                                Some((*i, a.clone()))
                            }
                            _ => None,
                        }));
                        cells.sort();
                        cells.dedup();
                        // Published by the prepare and commit deltas too.
                        c.cells = cells.clone();
                        cells
                    });
                #[cfg(test)]
                let candidates = candidates.filter(|_| !self.slow_layout);
                let rows = stage("layout.targets", || {
                    crate::row_reflow::targets(
                        &self.workspace,
                        &self.calculator.values,
                        &staged.workspace,
                        &staged.calculator.values,
                        &self.calculator.display.values,
                        &staged.calculator.display.values,
                        pkg,
                        candidates.as_deref(),
                    )
                })?;
                self.layout_serial += 1;
                // Relative to this engine's state, which the caught-up staged
                // engine published exactly (see `staged_base`).
                if self.want_delta == Some(self.epoch) {
                    staged.want_delta = staged_base;
                }
                let mut snapshot = stage("layout.response", || staged.snapshot());
                staged.want_delta = None;
                if snapshot.get("delta").is_some() {
                    snapshot["delta"]["base"] = json!(self.epoch);
                }
                snapshot["layoutToken"] = json!(self.layout_serial);
                snapshot["layoutRows"] = json!(rows);
                self.layout = Some(LayoutEdit {
                    token: self.layout_serial,
                    base_epoch: self.epoch,
                    staged_base,
                    candidates,
                    staged,
                    rows,
                    calculation_only,
                });
                Ok(snapshot)
            }
            "cancelLayoutEdit" => {
                if self.layout.as_ref().is_some_and(|p| r["token"] == p.token) {
                    self.layout_bundle = self.layout.take().map(|p| p.staged.into_bundle());
                }
                Ok(json!({"cancelled":true}))
            }
            "commitLayoutEdit" => {
                if self.layout.as_ref().is_none_or(|p| r["token"] != p.token) {
                    return Err("The measured edit expired. Try again.".into());
                }
                let mut pending = self.layout.take().unwrap();
                if pending.base_epoch != self.epoch {
                    return Err("The workbook changed while sizing its rows. Try again.".into());
                }
                #[cfg(test)]
                let direct = !self.slow_layout;
                #[cfg(not(test))]
                let direct = true;
                // The staged edit made exactly one history entry: commit it
                // directly. (Its revision bump means it always commits.)
                if direct && !pending.calculation_only && pending.staged.history.undo_len() == 1 {
                    let entry = pending.staged.history.pop_undo().unwrap();
                    return stage("layout.commit", || {
                        self.commit_staged(pending, entry, &r["sizes"])
                    });
                }
                crate::row_reflow::apply(
                    &mut pending.staged.workspace,
                    &pending.rows,
                    &r["sizes"],
                )?;
                validate(&pending.staged.workspace)?;
                if !pending.calculation_only && pending.staged.workspace == self.workspace {
                    return Ok(self.snapshot());
                }
                self.packages = pending.staged.packages;
                self.package_digests = pending.staged.package_digests;
                let conditional = std::mem::replace(
                    &mut self.conditional,
                    std::mem::take(&mut pending.staged.conditional),
                );
                let conditional_state = std::mem::replace(
                    &mut self.conditional_state,
                    std::mem::take(&mut pending.staged.conditional_state),
                );
                let conditional_warnings = std::mem::replace(
                    &mut self.conditional_warnings,
                    std::mem::take(&mut pending.staged.conditional_warnings),
                );
                self.layout_bundle = Some(LayoutBundle {
                    calculator: std::mem::replace(&mut self.calculator, pending.staged.calculator),
                    conditional: (conditional, conditional_state, conditional_warnings),
                    // What the staged engine changed isn't known here.
                    lag: None,
                });
                if pending.calculation_only {
                    // Derived values and automatic row sizes do not create edits,
                    // consume Undo/Redo or advance the document revision.
                    self.history
                        .rebase(&self.workspace, &pending.staged.workspace)?;
                    self.workspace = pending.staged.workspace;
                    let restore_clock = self.calculator.clock_override;
                    self.calculator.clock_override = self.calculator.clock;
                    self.sync(&ChangeSet::Structural);
                    self.calculator.clock_override = restore_clock;
                    Ok(self.snapshot())
                } else {
                    let journal = pending
                        .staged
                        .history
                        .last_undo()
                        .and_then(|entry| entry.document.clone());
                    self.commit_journal(pending.staged.workspace, journal)
                }
            }
            "recalculate" | "refreshCalendar" => {
                if op == "recalculate" {
                    self.calculator.invalidate();
                }
                // Only calculated results can change.
                self.pending_paths = Some(vec![]);
                self.sync(&ChangeSet::Cells(vec![]));
                Ok(self.snapshot())
            }
            "get" => Ok(self.snapshot()),
            // Delta responses omit file-state digests; callers fetch them here.
            "fileStates" => {
                if self
                    .file_states_cache
                    .as_ref()
                    .is_none_or(|(epoch, _)| *epoch != self.epoch)
                {
                    self.file_states_cache = Some((self.epoch, self.file_states()));
                }
                Ok(
                    json!({"epoch":self.epoch,"fileStates":self.file_states_cache.as_ref().unwrap().1}),
                )
            }
            "filterChoices" => crate::sheet_filter::choices(
                &self.workspace,
                &self.calculator.values,
                &self.calculator.display.values,
                &r,
            ),
            "new" => {
                self.workspace = seed();
                if r["blank"] == true {
                    self.workspace["title"] = json!("Untitled workspace");
                    self.workspace["document"] =
                        json!({"type":"doc","content":[{"type":"paragraph"}]});
                    self.workspace["sheets"] =
                        json!([{"id":"sheet-1","name":"Sheet 1","cells":{}}]);
                }
                self.history.clear();
                self.packages.clear();
                self.package_digests.clear();
                self.sync(&ChangeSet::Structural);
                Ok(self.snapshot())
            }
            "inspectWorkspace" => {
                // Validate and hydrate in an isolated core. Preview must never
                // replace the active workspace, its Undo, or staged layout.
                let mut preview = Core::new();
                preview.dispatch(json!({"op":"load","path":text(&r,"path")?}))?;
                Ok(workspace_description(&preview.workspace))
            }
            "load" | "recoverWorkspace" => {
                let path = text(&r, "path")?;
                if fs::metadata(path).map_err(|e| e.to_string())?.len() > 128_000_000 {
                    return Err("Workspace exceeds 128 MB".into());
                }
                let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
                if s.len() > 128_000_000 {
                    return Err("Workspace exceeds 32 MB".into());
                }
                let mut w: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
                let mut packages = HashMap::new();
                if let Some(encoded) = w
                    .as_object_mut()
                    .ok_or("Invalid Folio workspace")?
                    .remove("officePackages")
                {
                    for (id, data) in encoded.as_object().ok_or("Invalid Office packages")? {
                        packages.insert(
                            id.clone(),
                            Arc::new(crate::office::Package::decode(
                                data.as_str().ok_or("Invalid Office package")?,
                            )?),
                        );
                    }
                }
                if let Some(pkg) = w["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| packages.get(id))
                {
                    crate::xlsx_styles::hydrate(&mut w, pkg)?;
                    for key in ["sheets", "baseline"] {
                        let sheets = if key == "sheets" {
                            &mut w["sheets"]
                        } else {
                            &mut w["officeWorkbook"]["baseline"]
                        };
                        if let Some(sheets) = sheets.as_array_mut() {
                            for sheet in sheets {
                                if let Some(path) = sheet["officePath"].as_str().map(String::from) {
                                    if sheet.get(crate::sheet_reviews::KEY).is_none() {
                                        crate::xlsx_reviews::read(pkg, &path, sheet)?;
                                    }
                                    crate::conditional_format::hydrate(pkg, &path, sheet)?;
                                    if sheet.get(crate::cell_watches::KEY).is_none() {
                                        crate::cell_watches::read(pkg, &path, sheet)?;
                                    }
                                    if sheet.get(crate::scenarios::KEY).is_none() {
                                        crate::scenarios::read(pkg, &path, sheet)?;
                                    }
                                    crate::sheet_geometry::read_merges(pkg, &path, sheet)?;
                                }
                                if sheet.get("dataValidations").is_none() {
                                    if let Some(path) =
                                        sheet["officePath"].as_str().map(String::from)
                                    {
                                        crate::data_validation::read(pkg, &path, sheet)?;
                                    }
                                }
                                if sheet.get("frozenPane").is_none() {
                                    if let Some(path) =
                                        sheet["officePath"].as_str().map(String::from)
                                    {
                                        crate::sheet_view::read(pkg, &path, sheet)?;
                                    }
                                }
                                if sheet.get("tables").is_none() {
                                    if let Some(path) = sheet["officePath"].as_str() {
                                        let (tables, hidden) = crate::xlsx_tables::read(pkg, path)?;
                                        sheet["tables"] = json!(tables);
                                        sheet["hiddenRows"] = json!(hidden);
                                    }
                                }
                                if sheet.get("autoFilter").is_none() {
                                    if let Some(path) =
                                        sheet["officePath"].as_str().map(String::from)
                                    {
                                        crate::sheet_filter::read(pkg, &path, sheet)?;
                                    }
                                }
                                if sheet.get("rowHeights").is_none()
                                    && sheet.get("columnWidths").is_none()
                                {
                                    if let Some(path) =
                                        sheet["officePath"].as_str().map(String::from)
                                    {
                                        crate::sheet_geometry::read(pkg, &path, sheet)?;
                                    }
                                }
                                if let Some(path) = sheet["officePath"].as_str().map(String::from) {
                                    crate::sheet_geometry::hydrate_modes(pkg, &path, sheet)?;
                                }
                            }
                        }
                    }
                }
                if let Some(pkg) = w["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| packages.get(id))
                {
                    if w["officeDocument"].get("tracking").is_some() {
                        let source = crate::docx_tracking::import_current(pkg)?.0;
                        crate::docx_page_setup::hydrate(&mut w["document"], &source);
                        crate::docx_page_setup::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                        );
                        crate::docx_tracking::validate_settings(&w["officeDocument"]["tracking"])?;
                        if w["document"] != w["officeDocument"]["baseline"] {
                            return Err(
                                "Tracked recovery contains unrecorded document edits".into()
                            );
                        }
                        crate::docx_tracking::validate_projection(pkg, &w["document"])?;
                    } else {
                        w["officeDocument"]["recordingRequested"] =
                            crate::docx_tracking::read_settings(pkg)?["enabled"].clone();
                        let source = crate::docx::import(pkg)?.0;
                        crate::docx_page_setup::hydrate(&mut w["document"], &source);
                        crate::docx_page_setup::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                        );
                        let baseline = w["officeDocument"]["baseline"].clone();
                        crate::docx_revisions::hydrate(&mut w["document"], &source, &baseline);
                        crate::docx_revisions::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                            &baseline,
                        );
                        crate::docx_styles::hydrate(&mut w["document"], &source);
                        crate::docx_styles::hydrate(&mut w["officeDocument"]["baseline"], &source);
                        let old_baseline = w["officeDocument"]["baseline"].clone();
                        crate::docx_paragraph::hydrate(&mut w["document"], &source, &old_baseline);
                        crate::docx_paragraph::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                            &old_baseline,
                        );
                        crate::docx_tabs::hydrate(&mut w["document"], &source);
                        crate::docx_tabs::hydrate(&mut w["officeDocument"]["baseline"], &source);
                        crate::docx_tables::hydrate(&mut w["document"], &source);
                        crate::docx_tables::hydrate(&mut w["officeDocument"]["baseline"], &source);
                        crate::docx_numbering::hydrate(&mut w["document"], &source);
                        crate::docx_numbering::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                        );
                        crate::document_comments::hydrate(&mut w["document"], &source)?;
                        crate::document_comments::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                        )?;
                        crate::document_bookmarks::hydrate(&mut w["document"], &source)?;
                        crate::document_bookmarks::hydrate(
                            &mut w["officeDocument"]["baseline"],
                            &source,
                        )?;
                    }
                }
                validate(&w)?;
                if op == "recoverWorkspace" {
                    w["id"] = seed()["id"].clone();
                    w["revision"] = json!(0);
                }
                self.packages = packages;
                self.package_digests.clear();
                self.workspace = w;
                self.history.clear();
                self.calculator.invalidate();
                self.sync(&ChangeSet::Structural);
                Ok(self.snapshot())
            }
            "replace" => {
                validate(&r["workspace"])?;
                self.commit(r["workspace"].clone())
            }
            "document" => {
                let mut n = self.workspace.clone();
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The document changed. Reopen its settings and try again.".into());
                }
                let mut document = r["document"].clone();
                if !document.is_object()
                    || document
                        .get("attrs")
                        .is_some_and(|a| !a.is_null() && !a.is_object())
                {
                    return Err("Invalid document attributes".into());
                }
                if r["inheritStyleCatalog"] == true {
                    // Queued editor snapshots can predate automatic heading
                    // creation. This flag can only retain the current catalog;
                    // it never accepts declarations from the request.
                    if let Some(catalog) = n["document"]["attrs"].get(crate::docx_styles::KEY) {
                        document["attrs"][crate::docx_styles::KEY] = catalog.clone();
                    } else if let Some(attrs) =
                        document.get_mut("attrs").and_then(Value::as_object_mut)
                    {
                        attrs.remove(crate::docx_styles::KEY);
                    }
                }
                n["document"] = document_with_settings(&document, &n["document"])?;
                self.materialize_document_styles(&mut n)?;
                crate::document_bookmarks::rebase(
                    &self.workspace["document"],
                    &mut n["document"],
                    r.get("textEdits"),
                )?;
                crate::document_comments::rebase(
                    &self.workspace["document"],
                    &mut n["document"],
                    r.get("textEdits"),
                )?;
                self.commit_document(n, r.get("textEdits"))
            }
            "documentTracking" => {
                if r["workspaceId"] != self.workspace["id"]
                    || r["baseRevision"] != self.workspace["revision"]
                {
                    return Err("The document changed. Reopen Track Changes settings.".into());
                }
                let settings = json!({"version":1,"author":r["author"],"enabled":r["enabled"],"formatting":r["formatting"]});
                crate::docx_tracking::validate_settings(&settings)?;
                crate::docx_tracking::check_bindings(&self.workspace["document"])?;
                let source = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id));
                let mut package = if self.workspace["officeDocument"].get("tracking").is_some() {
                    source.ok_or("Tracked source is missing")?.as_ref().clone()
                } else {
                    crate::office::Package::from_bytes(crate::docx::export(
                        &self.workspace["document"],
                        source.map(AsRef::as_ref),
                        self.workspace["officeDocument"].get("baseline"),
                    )?)?
                };
                if crate::docx_revisions::list(&package)?["protected"] == true {
                    return Err("This document has enforced editing protection".into());
                }
                crate::docx_tracking::write_settings(&mut package, &settings)?;
                let mut n = self.workspace.clone();
                if n["officeDocument"].get("tracking").is_none() {
                    let (mut document, report) = crate::docx_tracking::import_current(&package)?;
                    if let Some(attrs) = n["document"]["attrs"].as_object() {
                        for (k, v) in attrs {
                            if document["attrs"].get(k).is_none() {
                                document["attrs"][k] = v.clone();
                            }
                        }
                    }
                    n["document"] = document.clone();
                    n["officeDocument"]["baseline"] = document;
                    n["officeDocument"]["report"] = report;
                }
                n["officeDocument"]["tracking"] = settings;
                n["officeDocument"]["recordingRequested"] = json!(false);
                let id = format!(
                    "package-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                crate::file_state::replace_package(&mut n, "officeDocument", &id);
                validate(&n)?;
                self.packages.insert(id, Arc::new(package));
                self.commit_journal(n, None)
            }
            "documentPageSetup"
            | "documentPageSetupApply"
            | "documentSectionContext"
            | "documentSectionBreak"
            | "documentHeaders"
            | "documentPagePreview"
            | "documentHeadersApply" => {
                if matches!(
                    op,
                    "documentPageSetupApply" | "documentSectionBreak" | "documentHeadersApply"
                ) && (r["baseRevision"] != self.workspace["revision"]
                    || r["workspaceId"] != self.workspace["id"])
                {
                    return Err("The document changed. Reopen the layout dialog.".into());
                }
                let source = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id));
                let tracking = self.workspace["officeDocument"].get("tracking");
                let resolved = self.resolved_document()?;
                let baked = if tracking.is_some() {
                    source.ok_or("Tracked source is missing")?.as_ref().clone()
                } else {
                    crate::docx::export_review(
                        &resolved,
                        source.map(|p| p.as_ref()),
                        self.workspace["officeDocument"].get("baseline"),
                    )?
                };
                if matches!(
                    op,
                    "documentPageSetup"
                        | "documentSectionContext"
                        | "documentHeaders"
                        | "documentPagePreview"
                ) {
                    let mut data = if op == "documentPagePreview" {
                        crate::docx_headers::page_preview(&baked)?
                    } else if op == "documentHeaders" {
                        crate::docx_headers::read(&baked, &r)?
                    } else if op == "documentSectionContext" {
                        crate::docx_sections::context(&baked, &resolved, &r)?
                    } else {
                        crate::docx_page_setup::read(&baked)?
                    };
                    if matches!(op, "documentSectionContext" | "documentHeaders") {
                        data["recordingEnabled"] =
                            json!(tracking.is_some_and(|s| s["enabled"] == true));
                        data["needsAuthor"] = json!(
                            tracking.is_none()
                                && self.workspace["officeDocument"]["recordingRequested"] == true
                        );
                        if op == "documentSectionContext"
                            && tracking.is_some()
                            && crate::docx_revisions::has_changes(
                                crate::office::parse(baked.text("word/document.xml")?)?
                                    .root_element(),
                            )
                        {
                            data["canInsert"] = json!(false);
                            data["insertReason"] = json!(
                                "Review pending recorded changes before inserting a section break"
                            );
                        }
                    }
                    data["workspaceId"] = self.workspace["id"].clone();
                    data["baseRevision"] = self.workspace["revision"].clone();
                    data["protected"] = json!(crate::docx_revisions::protection(&baked)?);
                    return Ok(data);
                }
                if tracking.is_none()
                    && self.workspace["officeDocument"]["recordingRequested"] == true
                {
                    return Err(
                        "Choose a review author in Recording settings before changing page setup"
                            .into(),
                    );
                }
                let mut updated = if op == "documentHeadersApply" {
                    crate::docx_headers::apply(&baked, &r, tracking)?
                } else if op == "documentSectionBreak" {
                    if r["action"] == "insert"
                        && tracking.is_some()
                        && crate::docx_revisions::has_changes(
                            crate::office::parse(baked.text("word/document.xml")?)?.root_element(),
                        )
                    {
                        return Err(
                            "Review pending recorded changes before inserting a section break"
                                .into(),
                        );
                    }
                    crate::docx_sections::apply(&baked, &resolved, &r, tracking)?
                } else {
                    crate::docx_page_setup::apply(&baked, &r, tracking)?
                };
                if updated.entries == baked.entries {
                    return Ok(self.snapshot());
                }
                let section_journal = if op == "documentSectionBreak" {
                    Some(crate::docx_sections::journal(&baked, &resolved, &r)?)
                } else if op == "documentHeadersApply" {
                    Some(json!([]))
                } else {
                    None
                };
                let bindings = crate::docx_revisions::restore_bindings(
                    &baked,
                    &mut updated,
                    &self.workspace["document"],
                )?;
                let (mut doc, report) = if tracking.is_some() {
                    crate::docx_tracking::import_current(&updated)?
                } else {
                    crate::docx::import(&updated)?
                };
                crate::docx_revisions::restore_model_bindings(&mut doc, &bindings)?;
                let mut n = self.workspace.clone();
                if let Some(attrs) = n["document"]["attrs"].as_object() {
                    for (key, value) in attrs {
                        if doc["attrs"].get(key).is_none() {
                            doc["attrs"][key] = value.clone();
                        }
                    }
                }
                n["document"] = doc.clone();
                n["officeDocument"]["baseline"] = doc;
                n["officeDocument"]["report"] = report;
                let id = format!(
                    "package-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                crate::file_state::replace_package(&mut n, "officeDocument", &id);
                validate(&n)?;
                self.packages.insert(id, Arc::new(updated));
                self.commit_document(n, section_journal.as_ref())
            }
            "documentRevisions" | "documentRevision" | "documentReviewView" => {
                if op == "documentRevision"
                    && (r["baseRevision"] != self.workspace["revision"]
                        || r["workspaceId"] != self.workspace["id"])
                {
                    return Err("The document changed. Reopen Track Changes.".into());
                }
                let source = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id));
                let tracking = self.workspace["officeDocument"].get("tracking").is_some();
                let baked = if tracking {
                    source
                        .ok_or("Tracked source package is missing")?
                        .as_ref()
                        .clone()
                } else {
                    crate::docx::export_review(
                        &self.resolved_document()?,
                        source.map(|p| p.as_ref()),
                        self.workspace["officeDocument"].get("baseline"),
                    )?
                };
                if op == "documentRevisions" || op == "documentReviewView" {
                    let mut list = crate::docx_revisions::list(&baked)?;
                    list["tracking"] = self.workspace["officeDocument"]
                        .get("tracking")
                        .cloned()
                        .unwrap_or_else(
                            || json!({"version":1,"author":"","enabled":false,"formatting":true}),
                        );
                    list["sourceRecording"] = crate::docx_tracking::read_settings(&baked)?;
                    list["recordingReason"] = json!(crate::docx_tracking::check_bindings(
                        &self.workspace["document"]
                    )
                    .err());
                    list["workspaceId"] = self.workspace["id"].clone();
                    list["baseRevision"] = self.workspace["revision"].clone();
                    list["canUndo"] = json!(self.history.can_undo());
                    list["canRedo"] = json!(self.history.can_redo());
                    if op == "documentReviewView" {
                        let view = crate::docx_revisions::display(&baked, text(&r, "mode")?)?;
                        list["view"] = view;
                    }
                    return Ok(list);
                }
                let mut updated = crate::docx_revisions::resolve(&baked, &r)?;
                let bindings = crate::docx_revisions::restore_bindings(
                    &baked,
                    &mut updated,
                    &self.workspace["document"],
                )?;
                let (mut doc, report) = if tracking {
                    crate::docx_tracking::import_current(&updated)?
                } else {
                    crate::docx::import(&updated)?
                };
                if !bindings.is_empty() {
                    let nodes = doc["content"].as_array().unwrap();
                    let mut merged = vec![];
                    let mut i = 0;
                    while i < nodes.len() {
                        if let Some((count, node)) = bindings.get(&i) {
                            merged.push(node.clone());
                            i += count;
                        } else {
                            merged.push(nodes[i].clone());
                            i += 1;
                        }
                    }
                    doc["content"] = json!(merged);
                }
                let mut n = self.workspace.clone();
                // Keep local document settings not represented in a Word package.
                if let Some(attrs) = n["document"]["attrs"].as_object() {
                    for (key, value) in attrs {
                        if ![
                            crate::docx_styles::KEY,
                            "bookmarks",
                            "comments",
                            "defaultTabStop",
                        ]
                        .contains(&key.as_str())
                            && doc["attrs"].get(key).is_none()
                        {
                            doc["attrs"][key] = value.clone();
                        }
                    }
                }
                n["document"] = doc.clone();
                n["officeDocument"]["baseline"] = doc;
                n["officeDocument"]["report"] = report;
                let id = format!(
                    "package-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                crate::file_state::replace_package(&mut n, "officeDocument", &id);
                validate(&n)?;
                self.packages.insert(id, Arc::new(updated));
                self.commit(n)
            }
            "documentStyles" => {
                let catalog = self.workspace["document"]["attrs"]
                    .get(crate::docx_styles::KEY)
                    .cloned()
                    .unwrap_or(crate::docx_styles::read(&crate::docx::new_package())?);
                let source = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(|p| crate::docx_styles::read(p))
                    .unwrap_or_else(|| crate::docx_styles::read(&crate::docx::new_package()))?;
                let catalog = crate::docx_styles::with_origins(&catalog, &source);
                let available = crate::builtin_styles::available(&catalog);
                let expanded = crate::builtin_styles::expanded(&catalog);
                let resolved = expanded["styles"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| {
                        (
                            s["id"].as_str().unwrap().to_string(),
                            crate::docx_styles::resolve(&expanded, s["id"].as_str()),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                Ok(
                    json!({"defaults":crate::docx_styles::resolve(&catalog,Some("")),"catalog":catalog,"availableStyles":available,"resolved":resolved,"revision":self.workspace["revision"],"workspaceId":self.workspace["id"]}),
                )
            }
            "documentStylePreview" => {
                if r["baseRevision"] != self.workspace["revision"]
                    || r["workspaceId"] != self.workspace["id"]
                    || r["action"] != "apply"
                {
                    return Err("The document changed. Reopen Styles and try again.".into());
                }
                let mut pkg = self.workspace["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(|p| (**p).clone())
                    .unwrap_or_else(crate::docx::new_package);
                let mut document = r["document"].clone();
                document["attrs"][crate::docx_styles::KEY] = self.workspace["document"]["attrs"]
                    .get(crate::docx_styles::KEY)
                    .cloned()
                    .unwrap_or(crate::docx_styles::read(&pkg)?);
                document_validate(&document, 0, &mut 0)?;
                let result = crate::docx_styles::command(&document, &r, &mut pkg, None)?;
                document_validate(&result, 0, &mut 0)?;
                Ok(json!({"document":result}))
            }
            "documentStyle" => {
                if self.workspace["officeDocument"].get("tracking").is_some() {
                    return Err("Changing the style catalog while recording revisions needs additional support".into());
                }
                if r["baseRevision"] != self.workspace["revision"]
                    || r["workspaceId"] != self.workspace["id"]
                {
                    return Err("The document changed. Reopen Styles and try again.".into());
                }
                let mut n = self.workspace.clone();
                let mut pkg = n["officeDocument"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(|p| (**p).clone())
                    .unwrap_or_else(crate::docx::new_package);
                n["document"] = crate::docx_styles::command(
                    &n["document"],
                    &r,
                    &mut pkg,
                    n["officeDocument"].get("baseline"),
                )?;
                if n == self.workspace {
                    return Ok(self.snapshot());
                }
                if r["action"] == "delete" && r.get("replacementStyleId").is_some() {
                    // Retain changed references in preserved headers, footnotes,
                    // numbering and opaque objects across history and recovery.
                    // Keep the original body baseline: its office IDs still map
                    // to the unchanged source structure, including pending edits.
                    let updated = crate::office::Package::from_bytes(pkg.bytes()?)?;
                    let id = format!(
                        "package-{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );
                    crate::file_state::replace_package(&mut n, "officeDocument", &id);
                    validate(&n)?;
                    self.packages.insert(id, Arc::new(updated));
                }
                self.commit(n)
            }
            "sheetReview" => {
                if r["workspaceId"] != self.workspace["id"]
                    || r["baseRevision"] != self.workspace["revision"]
                {
                    return Err("The workbook changed. Reopen Comments and Notes.".into());
                }
                let mut n = self.workspace.clone();
                let sheet = Self::sheet_mut(&mut n, text(&r, "sheetId")?)?;
                *sheet = crate::sheet_reviews::command(sheet, &r)?;
                if let Some(pkg) = self.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                {
                    let si = n["sheets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .position(|s| s["id"] == r["sheetId"])
                        .unwrap();
                    let sheet = &n["sheets"][si];
                    if let Some(path) = sheet["officePath"].as_str() {
                        let dom = crate::office::parse(pkg.text(path)?)?;
                        if crate::office::child(dom.root_element(), "sheetProtection").is_some() {
                            return Err("Remove worksheet protection in Excel before changing comments or notes.".into());
                        }
                        // Reject collisions with retained unsupported annotations before
                        // accepting an edit, rather than deferring the error to Save.
                        let mut check = (**pkg).clone();
                        crate::xlsx_reviews::write(
                            &mut check,
                            sheet,
                            self.workspace["officeWorkbook"]["baseline"].get(si),
                            path,
                        )?;
                    }
                }
                self.commit(n)
            }
            "documentComment" => {
                if r["workspaceId"] != self.workspace["id"]
                    || r["baseRevision"] != self.workspace["revision"]
                {
                    return Err("The document changed. Reopen Comments and try again.".into());
                }
                let mut n = self.workspace.clone();
                n["document"] = crate::document_comments::command(&n["document"], &r)?;
                self.commit(n)
            }
            "bookmark" => {
                if r.get("workspaceId")
                    .is_some_and(|id| *id != self.workspace["id"])
                {
                    return Err("This bookmark belongs to a different workspace.".into());
                }
                if r["baseRevision"] != self.workspace["revision"] {
                    return Err("The document changed. Reopen Bookmarks and try again.".into());
                }
                let mut n = self.workspace.clone();
                n["document"] = crate::document_bookmarks::command(&n["document"], &r)?;
                self.commit(n)
            }
            "title" => {
                let mut n = self.workspace.clone();
                n["title"] = json!(text(&r, "title")?.chars().take(100).collect::<String>());
                self.commit(n)
            }
            "dataValidation" | "conditionalFormat" | "conditionalCheck" => {
                if let Some(pkg) = self.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                {
                    if let Some(path) = self.workspace["sheets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|s| s["id"] == r["sheetId"])
                        .and_then(|s| s["officePath"].as_str())
                    {
                        let doc = crate::office::parse(pkg.text(path)?)?;
                        if crate::office::child(doc.root_element(), "sheetProtection").is_some() {
                            return Err(
                                "Remove worksheet protection in Excel before changing its rules."
                                    .into(),
                            );
                        }
                    }
                }
                let next = if op.starts_with("conditional") {
                    crate::conditional_format::change(&self.workspace, &r)?
                } else {
                    crate::data_validation::change(&self.workspace, &r)?
                };
                if op == "conditionalCheck" {
                    return Ok(json!({"valid":true}));
                }
                if next == self.workspace {
                    return Ok(self.snapshot());
                }
                self.commit(next)
            }
            "goalSeekStart" => {
                let pkg = self.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(AsRef::as_ref);
                self.goal_seek_serial += 1;
                let search = crate::goal_seek::Session::new(
                    &self.workspace,
                    &self.calculator,
                    pkg,
                    &r,
                    self.goal_seek_serial,
                )?;
                let result = search.response();
                self.goal_seek = Some(search);
                Ok(result)
            }
            "goalSeekStep" | "goalSeekStop" => {
                let search = self
                    .goal_seek
                    .as_mut()
                    .ok_or("Goal Seek result expired. Run it again.")?;
                search.check(&self.workspace, self.calculator.generation, &r["token"])?;
                Ok(if op == "goalSeekStep" {
                    search.advance()
                } else {
                    search.stop()
                })
            }
            "goalSeekCancel" => {
                if self
                    .goal_seek
                    .as_ref()
                    .is_some_and(|s| r["token"] == s.token)
                {
                    self.goal_seek = None;
                }
                Ok(json!({"cancelled":true}))
            }
            "goalSeekAccept" => {
                Err("Apply Goal Seek through the measured cell-edit transaction".into())
            }
            "scenarioCapture" => {
                let si = self.scenario_scope(&r)?;
                let sheet = &self.workspace["sheets"][si];
                let sid = sheet["id"].as_str().unwrap();
                let addresses = crate::scenarios::addresses(text(&r, "range")?, 32)?;
                let inputs = addresses
                    .into_iter()
                    .map(|a| {
                        let raw = sheet["cells"][&a].as_str().unwrap_or("");
                        let value = if raw.starts_with('=') {
                            match &self.calculator.values[sid][&a] {
                                Value::String(s) => format!("'{s}"),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => {
                                    if *b {
                                        "TRUE".into()
                                    } else {
                                        "FALSE".into()
                                    }
                                }
                                _ => String::new(),
                            }
                        } else {
                            raw.to_string()
                        };
                        json!({"address":a,"value":value})
                    })
                    .collect::<Vec<_>>();
                Ok(json!({"inputs":inputs}))
            }
            "inspectScenarioSource" => {
                self.scenario_scope(&r)?;
                self.scenario_source = None;
                self.scenario_source_serial += 1;
                let source = crate::scenario_source::Source::read(
                    text(&r, "path")?,
                    self.scenario_source_serial,
                )?;
                let preview = source.preview();
                self.scenario_source = Some(source);
                Ok(preview)
            }
            "closeScenarioSource" => {
                if self
                    .scenario_source
                    .as_ref()
                    .is_some_and(|s| r["sourceToken"] == s.token)
                {
                    self.scenario_source = None;
                }
                Ok(json!({"closed":true}))
            }
            "scenarios" => {
                self.scenario_scope(&r)?;
                let pkg = self.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(AsRef::as_ref);
                let next = if r["action"] == "mergeExternal" {
                    self.scenario_source
                        .as_ref()
                        .ok_or("The source workbook expired. Choose it again.")?
                        .merge(&self.workspace, &r, pkg)?
                } else {
                    crate::scenarios::change(&self.workspace, &r, pkg)?
                };
                if next == self.workspace {
                    return Ok(self.snapshot());
                }
                self.scenario_preflight(&next)?;
                self.commit(next)
            }
            "scenarioShow" => {
                let next = self.scenario_next(&r)?;
                if next == self.workspace {
                    return Ok(self.snapshot());
                }
                self.scenario_preflight(&next)?;
                self.commit(next)
            }
            "scenarioSummary" => {
                let next = self.scenario_summary(&r)?;
                self.scenario_preflight(&next)?;
                self.commit(next)
            }
            "cellWatches" => {
                if r["workspaceId"] != self.workspace["id"]
                    || r["baseRevision"] != self.workspace["revision"]
                {
                    return Err("The workbook changed. Select the watches again.".into());
                }
                let next = crate::cell_watches::change(&self.workspace, &r)?;
                if next == self.workspace {
                    return Ok(self.snapshot());
                }
                // Validate source-package writing before adding a history entry.
                if let Some(pkg) = self.workspace["officeWorkbook"]["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                {
                    let mut check = (**pkg).clone();
                    for (sheet, before) in next["sheets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .zip(self.workspace["sheets"].as_array().unwrap())
                    {
                        if let Some(path) = sheet["officePath"].as_str() {
                            crate::cell_watches::write(&mut check, sheet, Some(before), path)?;
                        }
                    }
                }
                self.commit(next)
            }
            "formulaAudit" => {
                if r["workspaceId"] != self.workspace["id"]
                    || r["baseRevision"] != self.workspace["revision"]
                    || r["calculationGeneration"].as_u64() != Some(self.calculator.generation)
                {
                    return Err(
                        "The workbook changed. Reopen Formula inspector to refresh its references."
                            .into(),
                    );
                }
                let offset = r
                    .get("offset")
                    .map_or(Ok(0), |v| v.as_u64().ok_or("Invalid trace offset"))?;
                let limit = r
                    .get("limit")
                    .map_or(Ok(100), |v| v.as_u64().ok_or("Invalid trace page size"))?;
                if limit == 0 || limit > 500 || offset > usize::MAX as u64 {
                    return Err("Invalid trace page size or offset".into());
                }
                let mut result = self.calculator.audit(
                    text(&r, "sheetId")?,
                    text(&r, "address")?,
                    offset as usize,
                    limit as usize,
                )?;
                result["workspaceId"] = self.workspace["id"].clone();
                result["revision"] = self.workspace["revision"].clone();
                Ok(result)
            }
            "conditionalRange" => {
                if r["baseRevision"] != self.workspace["revision"] {
                    return Err("The workbook changed before formatting the view".into());
                }
                let index = self.workspace["sheets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .position(|s| s["id"] == r["sheetId"])
                    .ok_or("Worksheet missing")?;
                let requested = r["cells"].as_array().ok_or("Visible cells missing")?;
                if requested.len() > 10000 {
                    return Err("Too many visible cells".into());
                }
                let cells = requested
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(String::from)
                            .ok_or("Invalid cell address".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut result = crate::conditional_format::evaluate(
                    &self.workspace["sheets"],
                    index,
                    &cells,
                    &mut self.calculator,
                )?;
                let id = r["sheetId"].as_str().unwrap();
                let sheet = &self.workspace["sheets"][index];
                let mut display = serde_json::Map::new();
                for cell in cells {
                    let style = &result["styles"][&cell];
                    if style.is_null() {
                        continue;
                    }
                    let value = &self.calculator.values[id][&cell];
                    let mut formatted = if self.calculator.cell_errors[id][&cell] == true {
                        json!({"text":display_value(value)})
                    } else {
                        crate::number_format::render(
                            value,
                            style["numFmt"]
                                .as_str()
                                .unwrap_or_else(|| crate::number_format::code(sheet, &cell)),
                            self.calculator.date1904,
                            &self.calculator.display.palette,
                        )
                    };
                    formatted["conditionalStyle"] = style.clone();
                    display.insert(cell, formatted);
                }
                result["formatted"] = json!(display);
                Ok(result)
            }
            "validationChoices" => {
                crate::data_validation::choices(&self.workspace, &r, &mut self.calculator)
            }
            "validationEntryCheck" | "enterCell" => {
                if r["baseRevision"] != self.workspace["revision"] {
                    return Err("The workbook changed. Review the cell entry again.".into());
                }
                let edit =
                    json!({"sheetId":r["sheetId"],"address":r["address"],"value":r["value"]});
                if let Some((back, size)) = self.apply_cells(&json!([edit]))? {
                    let checked = crate::data_validation::check(
                        &self.workspace,
                        &r,
                        self.calculator.date1904,
                    )
                    .and_then(|mut checked| {
                        checked["revision"] = self.workspace["revision"].clone();
                        if op == "validationEntryCheck" {
                            return Ok(checked);
                        }
                        entry_allowed(&checked, &r).map(|_| Value::Null)
                    });
                    if op == "enterCell" && checked.as_ref().is_ok_and(Value::is_null) {
                        return self.finish_cells(back, size);
                    }
                    crate::json_patch::apply_inverse(&mut self.workspace, back)?;
                    return checked;
                }
                let next=self.proposed(&json!({"summary":"Enter cell","baseRevision":r["baseRevision"],"cells":[edit]}))?;
                let mut checked =
                    crate::data_validation::check(&next, &r, self.calculator.date1904)?;
                checked["revision"] = self.workspace["revision"].clone();
                if op == "validationEntryCheck" {
                    return Ok(checked);
                }
                entry_allowed(&checked, &r)?;
                self.commit(next)
            }
            "cells" => {
                if let Some(result) = self.commit_cells(&r["cells"])? {
                    return Ok(result);
                }
                let p = json!({"summary":"Edit cells","baseRevision":self.workspace["revision"],"cells":r["cells"]});
                let n = stage("proposed", || self.proposed(&p))?;
                self.commit(n)
            }
            "previewNumberFormat" => {
                let code = text(&r, "format")?;
                crate::number_format::validate(code)?;
                Ok(crate::number_format::render(
                    &r["value"],
                    code,
                    self.calculator.display.date1904,
                    &self.calculator.display.palette,
                ))
            }
            "copyCells" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Copy the selection again.".into());
                }
                let bytes = self.clipboard_package()?;
                let copied = crate::sheet_clipboard::Snapshot::new(
                    &self.workspace,
                    &self.calculator.values,
                    &self.calculator.cell_errors,
                    &r,
                    bytes,
                )?;
                let response = copied.response(&self.calculator.display.values);
                self.clipboard = Some(Arc::new(copied));
                Ok(response)
            }
            "pasteCells" => {
                if r.get("workspaceId")
                    .is_some_and(|v| *v != self.workspace["id"])
                {
                    return Err("The active workbook changed. Paste again.".into());
                }
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Paste again.".into());
                }
                let pkg = crate::office::Package::from_bytes(self.clipboard_package()?)?;
                let (mut next, mut updated) = crate::sheet_clipboard::paste(
                    &self.workspace,
                    &r,
                    self.clipboard.as_deref(),
                    pkg,
                )?;
                let mut projection = crate::xlsx::import(&updated)?.0;
                bind_rules(&mut next["sheets"], &projection);
                for (i, sheet) in next["sheets"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    projection[i]["id"] = sheet["id"].clone();
                    for key in [
                        "officePath",
                        "officeStyles",
                        "cellStyles",
                        "styleDefaults",
                        "usedRows",
                        "usedColumns",
                    ] {
                        sheet[key] = projection[i][key].clone();
                    }
                }
                crate::xlsx_tables::bind_paths(&mut next["sheets"], &projection, true)?;
                let mut calc = crate::calculation::WorkbookCalculator::default();
                calc.sync_calendar(&next["sheets"], self.calculator.date1904);
                updated = crate::office::Package::from_bytes(crate::xlsx::export_recalculated(
                    &next["sheets"],
                    &calc.values,
                    &calc.cell_errors,
                    &updated,
                    &projection,
                )?)?;
                let id = format!(
                    "package-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                crate::file_state::replace_package(&mut next, "officeWorkbook", &id);
                bind_rules(&mut next["sheets"], &crate::xlsx::import(&updated)?.0);
                next["officeWorkbook"]["baseline"] = next["sheets"].clone();
                validate(&next)?;
                self.packages.insert(id, Arc::new(updated));
                self.commit(next)
            }
            "fillCells" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Try filling the selection again.".into());
                }
                let fill = crate::sheet_fill::Fill::from_request(&self.workspace, &r)?;
                let mut next = fill.apply(&self.workspace)?;
                if let Some(id) = self.workspace["officeWorkbook"]["packageId"].as_str() {
                    let pkg = self.packages.get(id).ok_or("Workbook source unavailable")?;
                    let bytes = crate::xlsx::export(
                        &self.workspace["sheets"],
                        &self.calculator.values,
                        &self.calculator.cell_errors,
                        Some(pkg),
                        self.workspace["officeWorkbook"].get("baseline"),
                    )?;
                    let mut updated = crate::office::Package::from_bytes(bytes)?;
                    let source = crate::xlsx::import(&updated)?.0;
                    fill.copy_package(&mut updated, &source)?;
                    let mut projection = crate::xlsx::import(&updated)?.0;
                    bind_rules(&mut next["sheets"], &projection);
                    for (i, sheet) in next["sheets"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .enumerate()
                    {
                        projection[i]["id"] = sheet["id"].clone();
                        for key in [
                            "officePath",
                            "officeStyles",
                            "cellStyles",
                            "styleDefaults",
                            "usedRows",
                            "usedColumns",
                        ] {
                            sheet[key] = projection[i][key].clone();
                        }
                    }
                    crate::xlsx_tables::bind_paths(&mut next["sheets"], &projection, true)?;
                    let mut calc = crate::calculation::WorkbookCalculator::default();
                    calc.sync_calendar(&next["sheets"], self.calculator.date1904);
                    updated =
                        crate::office::Package::from_bytes(crate::xlsx::export_recalculated(
                            &next["sheets"],
                            &calc.values,
                            &calc.cell_errors,
                            &updated,
                            &projection,
                        )?)?;
                    let id = format!(
                        "package-{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );
                    crate::file_state::replace_package(&mut next, "officeWorkbook", &id);
                    bind_rules(&mut next["sheets"], &crate::xlsx::import(&updated)?.0);
                    next["officeWorkbook"]["baseline"] = next["sheets"].clone();
                    validate(&next)?;
                    self.packages.insert(id, Arc::new(updated));
                }
                self.commit(next)
            }
            "sheetGeometry" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Try the operation again.".into());
                }
                let n = crate::sheet_geometry::change(&self.workspace, &r)?;
                if r["action"] == "autoFit" {
                    if let Some(pkg) = self.workspace["officeWorkbook"]["packageId"]
                        .as_str()
                        .and_then(|id| self.packages.get(id))
                    {
                        if let Some(path) = n["sheets"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["id"] == r["sheetId"])
                            .and_then(|s| s["officePath"].as_str())
                        {
                            crate::sheet_geometry::check_autofit_merges(pkg, path, &r)?;
                        }
                    }
                    if n == self.workspace {
                        return Ok(self.snapshot());
                    }
                }
                self.commit(n)
            }
            "sheetView" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Try the operation again.".into());
                }
                let n = crate::sheet_view::change(&self.workspace, &r)?;
                self.commit(n)
            }
            "formatCellText" => {
                let n = crate::sheet_text::change(&self.workspace, &r)?;
                self.commit(n)
            }
            "formatCells" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err(
                        "The workbook changed. Open Font again to review the current cells.".into(),
                    );
                }
                let mut n = self.workspace.clone();
                let sheet = Self::sheet_mut(&mut n, text(&r, "sheetId")?)?;
                let addresses = r["addresses"].as_array().ok_or("No cells selected")?;
                if addresses.len() > 2000 {
                    return Err("Format up to 2,000 cells at once".into());
                }
                if !sheet["cellStyles"].is_object() {
                    sheet["cellStyles"] = json!({})
                }
                let style = r["style"].as_object().ok_or("Invalid format")?;
                crate::xlsx_styles::validate_format(style)?;
                for a in addresses {
                    let a = a.as_str().ok_or("Invalid address")?;
                    parse_address(a)?;
                    if !sheet["cellStyles"][a].is_object() {
                        sheet["cellStyles"][a] = json!({});
                    }
                    for (key, v) in style {
                        sheet["cellStyles"][a][key] = v.clone();
                        if crate::xlsx_styles::FONT_KEYS.contains(&key.as_str()) {
                            if !sheet["cellStyles"][a]["_fontOverrides"].is_object() {
                                sheet["cellStyles"][a]["_fontOverrides"] = json!({});
                            }
                            sheet["cellStyles"][a]["_fontOverrides"][key] = v.clone();
                        }
                    }
                    if sheet["cells"].get(a).is_none() {
                        sheet["cells"][a] = json!("");
                    }
                }
                self.commit(n)
            }
            "addSheet" => {
                let mut n = self.workspace.clone();
                let sheets = n["sheets"].as_array_mut().unwrap();
                if sheets.len() >= 30 {
                    return Err("Maximum 30 sheets".into());
                }
                let id = format!(
                    "sheet-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                let name = (1..)
                    .map(|i| format!("Sheet {i}"))
                    .find(|name| {
                        !sheets.iter().any(|s| {
                            s["name"]
                                .as_str()
                                .is_some_and(|n| n.eq_ignore_ascii_case(name))
                        })
                    })
                    .unwrap();
                sheets.push(json!({"id":id,"name":name,"cells":{}}));
                self.commit(n)
            }
            "structure" => {
                if let Some(revision) = r.get("baseRevision") {
                    if *revision != self.workspace["revision"] {
                        return Err("The workbook changed. Try the operation again.".into());
                    }
                }
                let change = crate::structure::Change::from_request(&self.workspace, &r)?;
                let mut next = change.apply_calendar(&self.workspace, self.calculator.date1904)?;
                if let Some(id) = self.workspace["officeWorkbook"]["packageId"].as_str() {
                    let pkg = self
                        .packages
                        .get(id)
                        .ok_or("Original workbook package is unavailable")?;
                    // First bake in existing cell edits and added worksheets.
                    // Transform only a disposable package; any error leaves both
                    // the current document and its source package untouched.
                    let bytes = crate::xlsx::export(
                        &self.workspace["sheets"],
                        &self.calculator.values,
                        &self.calculator.cell_errors,
                        Some(pkg),
                        self.workspace["officeWorkbook"].get("baseline"),
                    )?;
                    let mut updated = crate::office::Package::from_bytes(bytes)?;
                    crate::xlsx_tables::rewrite_references(
                        &mut updated,
                        &self.workspace["sheets"],
                        &next["sheets"],
                    )?;
                    crate::xlsx_structure::transform(&mut updated, &change)?;
                    updated.original = updated.bytes()?;
                    let (mut projection, _) = crate::xlsx::import(&updated)?;
                    bind_rules(&mut next["sheets"], &projection);
                    crate::xlsx_tables::bind_paths(&mut next["sheets"], &projection, true)?;
                    // The transformed package still has its original table
                    // descriptors. Write the canonical new columns/ranges and
                    // calculated rows before making it the next baseline.
                    for projected in projection.as_array_mut().unwrap() {
                        if let Some(s) = next["sheets"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["name"] == projected["name"])
                        {
                            projected["id"] = s["id"].clone();
                        }
                    }
                    let mut calc = crate::calculation::WorkbookCalculator::default();
                    calc.sync_calendar(&next["sheets"], self.calculator.date1904);
                    let bytes = crate::xlsx::export(
                        &next["sheets"],
                        &calc.values,
                        &calc.cell_errors,
                        Some(&updated),
                        Some(&projection),
                    )?;
                    updated = crate::office::Package::from_bytes(bytes)?;
                    crate::xlsx_tables::bind_paths(
                        &mut next["sheets"],
                        &crate::xlsx::import(&updated)?.0,
                        true,
                    )?;
                    let projected = crate::xlsx::import(&updated)?.0;
                    for sheet in next["sheets"].as_array_mut().unwrap() {
                        if let Some(source) = projected
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["name"] == sheet["name"])
                        {
                            sheet["styleDefaults"] = source["styleDefaults"].clone();
                        }
                    }
                    let id = format!(
                        "package-{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );
                    crate::file_state::replace_package(&mut next, "officeWorkbook", &id);
                    bind_rules(&mut next["sheets"], &crate::xlsx::import(&updated)?.0);
                    next["officeWorkbook"]["baseline"] = next["sheets"].clone();
                    validate(&next)?;
                    self.packages.insert(id, Arc::new(updated));
                } else if self.workspace["sheets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| {
                        crate::data_validation::rules(s).next().is_some()
                            || crate::conditional_format::rules(s).next().is_some()
                    })
                {
                    // Reuse the OOXML reference transformer for new workbooks as
                    // well, including relative formulas whose anchor is deleted.
                    let bytes = crate::xlsx::export(
                        &self.workspace["sheets"],
                        &self.calculator.values,
                        &self.calculator.cell_errors,
                        None,
                        None,
                    )?;
                    let mut package = crate::office::Package::from_bytes(bytes)?;
                    crate::xlsx_structure::transform(&mut package, &change)?;
                    bind_rules(&mut next["sheets"], &crate::xlsx::import(&package)?.0);
                }
                self.commit(next)
            }
            "table" | "sortRange" | "autoFilter" => {
                if r.get("baseRevision")
                    .is_some_and(|v| *v != self.workspace["revision"])
                {
                    return Err("The workbook changed. Try the operation again.".into());
                }
                let range_sort = r["op"] == "sortRange";
                let (mut next, order) = if r["op"] == "autoFilter" {
                    (
                        crate::sheet_filter::change(&self.workspace, &self.calculator.values, &r)?,
                        None,
                    )
                } else if range_sort {
                    crate::sheet_sort::change(&self.workspace, &self.calculator.values, &r)?
                } else {
                    crate::tables::change_calendar(
                        &self.workspace,
                        &self.calculator.values,
                        &r,
                        self.calculator.date1904,
                    )?
                };
                let meta = &self.workspace["officeWorkbook"];
                if let Some(id) = meta["packageId"].as_str() {
                    let pkg = self.packages.get(id).ok_or("Workbook source unavailable")?;
                    let mut before = self.workspace["sheets"].clone();
                    let bytes = crate::xlsx::export(
                        &before,
                        &self.calculator.values,
                        &self.calculator.cell_errors,
                        Some(pkg),
                        meta.get("baseline"),
                    )?;
                    let mut updated = crate::office::Package::from_bytes(bytes)?;
                    let projection = crate::xlsx::import(&updated)?.0;
                    crate::xlsx_tables::bind_paths(&mut before, &projection, false)?;
                    bind_rules(&mut before, &projection);
                    bind_rules(&mut next["sheets"], &projection);
                    // New sheets and tables have acquired paths in the baked
                    // package. Match by stable application id, including a table
                    // being renamed by this operation.
                    for sheet in next["sheets"].as_array_mut().unwrap() {
                        let old = before
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["id"] == sheet["id"])
                            .unwrap();
                        sheet["officePath"] = old["officePath"].clone();
                        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut)
                        {
                            for t in tables {
                                if let Some(old) =
                                    crate::tables::list(old).find(|old| old["id"] == t["id"])
                                {
                                    t["officePath"] = old["officePath"].clone();
                                    t["officeId"] = old["officeId"].clone();
                                }
                            }
                        }
                    }
                    if let Some(order) = order {
                        let sheet = before
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["id"] == r["sheetId"])
                            .ok_or("Worksheet missing")?;
                        let source_range = json!({"range":r["range"]});
                        let table = if range_sort {
                            &source_range
                        } else {
                            crate::tables::list(sheet)
                                .find(|t| t["id"] == r["tableId"])
                                .ok_or("Table missing")?
                        };
                        let mut moving = table.clone();
                        if !range_sort && r["action"] == "resize" {
                            let old = crate::tables::area(table)?;
                            let new = crate::tables::Area::parse(text(&r, "range")?.trim())?;
                            moving["range"] = json!(crate::tables::Area {
                                top: old.bottom.min(new.bottom),
                                bottom: old.bottom.max(new.bottom),
                                left: old.left.max(new.left),
                                right: old.right.min(new.right),
                            }
                            .text());
                        }
                        crate::xlsx_tables::sort_objects(
                            &mut updated,
                            sheet,
                            &moving,
                            &order,
                            range_sort || r["action"] != "resize",
                        )?;
                        let projection = crate::xlsx::import(&updated)?.0;
                        for old in before.as_array_mut().unwrap() {
                            if let Some(p) = projection
                                .as_array()
                                .unwrap()
                                .iter()
                                .find(|p| p["name"] == old["name"])
                            {
                                for key in ["cells", "officeStyles", "cellStyles", "cellReviews"] {
                                    old[key] = p[key].clone();
                                }
                            }
                        }
                    }
                    let mut calc = crate::calculation::WorkbookCalculator::default();
                    calc.sync_calendar(&next["sheets"], self.calculator.date1904);
                    // Include moved notes and metadata even if the row values
                    // compare equal, as can happen when sorting duplicate rows.
                    updated.original = updated.bytes()?;
                    let bytes = crate::xlsx::export(
                        &next["sheets"],
                        &calc.values,
                        &calc.cell_errors,
                        Some(&updated),
                        Some(&before),
                    )?;
                    updated = crate::office::Package::from_bytes(bytes)?;
                    let projection = crate::xlsx::import(&updated)?.0;
                    for sheet in next["sheets"].as_array_mut().unwrap() {
                        let imported = projection
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|s| s["name"] == sheet["name"])
                            .ok_or("Updated worksheet missing")?;
                        sheet["officePath"] = imported["officePath"].clone();
                        sheet["officeStyles"] = imported["officeStyles"].clone();
                        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut)
                        {
                            for t in tables {
                                if let Some(it) =
                                    crate::tables::list(imported).find(|it| it["name"] == t["name"])
                                {
                                    t["officePath"] = it["officePath"].clone();
                                    t["officeId"] = it["officeId"].clone();
                                }
                            }
                        }
                    }
                    let id = format!(
                        "package-{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                    );
                    crate::file_state::replace_package(&mut next, "officeWorkbook", &id);
                    bind_rules(&mut next["sheets"], &crate::xlsx::import(&updated)?.0);
                    next["officeWorkbook"]["baseline"] = next["sheets"].clone();
                    validate(&next)?;
                    self.packages.insert(id, Arc::new(updated));
                }
                self.commit(next)
            }
            "propose" => {
                self.proposed(&r["proposal"])?;
                let mut p = r["proposal"].clone();
                p["_workspaceId"] = self.workspace["id"].clone();
                Ok(p)
            }
            "apply" => {
                let n = self.proposed(&r["proposal"])?;
                self.commit_document(n, r["proposal"].get("textEdits"))
            }
            "undo" | "redo" => {
                let back = r["op"] == "undo";
                let next = if back {
                    self.history.pop_undo()
                } else {
                    self.history.pop_redo()
                };
                let mut change = None;
                if let Some(entry) = next {
                    let previous_revision = self.workspace["revision"].clone();
                    let revision = json!(previous_revision.as_u64().unwrap() + 1);
                    let changes = ChangeSet::from_patch(&entry.patch);
                    self.pending_paths = Some(entry.patch.paths());
                    let opposite =
                        crate::json_patch::apply_inverse(&mut self.workspace, entry.patch)?;
                    self.workspace["revision"] = revision;
                    change = entry.document.as_ref().map(|journal| {
                        json!({
                            "workspaceId":self.workspace["id"], "beforeRevision":previous_revision,
                            "afterRevision":self.workspace["revision"],
                            "textEdits":if back { &journal.reverse } else { &journal.forward },
                        })
                    });
                    let opposite = crate::document_history::Entry::new(opposite, entry.document);
                    if back {
                        self.history.push_redo(opposite)
                    } else {
                        self.history.push_undo(opposite)
                    }
                    self.sync(&changes);
                }
                let mut result = self.snapshot();
                if let Some(change) = change {
                    result["documentHistoryChange"] = change;
                }
                Ok(result)
            }
            "saveEmptyWorkspace" => {
                // Closing with Don't Save must not revive discarded edits on
                // restart. Stage an independent blank workspace so a failed
                // write cannot mutate the still-open document or its history.
                let mut empty = Core::new();
                empty.dispatch(json!({"op":"new","blank":true}))?;
                empty.dispatch(json!({"op":"save","path":text(&r,"path")?}))
            }
            "save" => {
                let path = text(&r, "path")?;
                let data = serde_json::to_string_pretty(&{
                    let mut saved = self.workspace.clone();
                    let mut packages = serde_json::Map::new();
                    for key in ["officeDocument", "officeWorkbook"] {
                        if let Some(id) = saved[key]["packageId"].as_str() {
                            if let Some(p) = self.packages.get(id) {
                                packages.insert(id.into(), json!(p.encoded()));
                            }
                        }
                    }
                    if !packages.is_empty() {
                        saved["officePackages"] = json!(packages);
                    }
                    saved
                })
                .map_err(|e| e.to_string())?;
                if data.len() > 128_000_000 {
                    return Err("File exceeds 128 MB".into());
                }
                let fingerprint = crate::office::write_checked_bytes(
                    path,
                    data.as_bytes(),
                    r["expectedFingerprint"].as_str(),
                )?;
                let mut result = json!({"saved":true,"revision":self.workspace["revision"],"fingerprint":fingerprint,"contentSignature":self.file_states()["folio"]});
                if r["describe"] == true {
                    result["description"] = workspace_description(&self.workspace);
                }
                Ok(result)
            }
            "exportHtml" => Ok(
                json!({"text":format!("<!doctype html><meta charset=\"utf-8\"><title>{}</title><style>body{{max-width:760px;margin:60px auto;font:18px/1.7 Georgia}}table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:8px}}</style>{}",esc(self.workspace["title"].as_str().unwrap()),html(&self.resolved_document()?))}),
            ),
            "exportCsv" => {
                let id = text(&r, "sheetId")?;
                let s = self.workspace["sheets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == id)
                    .ok_or("Sheet not found")?;
                let c = cells(s);
                let used: Vec<_> = c
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(a, _)| parse_address(a).unwrap())
                    .collect();
                let maxr = used.iter().map(|p| p.0).max().unwrap_or(0);
                let maxc = used.iter().map(|p| p.1).max().unwrap_or(0);
                if (maxr + 1).saturating_mul(maxc + 1) > 2_000_000 {
                    return Err(
                        "CSV export exceeds two million cells; use XLSX for sparse sheets.".into(),
                    );
                }
                let mut lines = vec![];
                for row in 0..=maxr {
                    let mut fields = vec![];
                    for col in 0..=maxc {
                        let v = self.calculator.values[id][address(row, col)].clone();
                        let s = v.as_str().map(String::from).unwrap_or_else(|| {
                            v.as_f64().map(|n| n.to_string()).unwrap_or_default()
                        });
                        fields.push(if s.contains([',', '"', '\n', '\r']) {
                            format!("\"{}\"", s.replace('"', "\"\""))
                        } else {
                            s
                        });
                    }
                    lines.push(fields.join(","));
                }
                Ok(json!({"text":lines.join("\r\n")}))
            }
            "writeExport" => {
                atomic_save(text(&r, "path")?, text(&r, "text")?)?;
                Ok(json!({"saved":true}))
            }
            "readRange" => self.read_range(text(&r, "sheetId")?, text(&r, "range")?),
            "inspectTemplate" | "newFromTemplate" => {
                let path = text(&r, "path")?;
                let format = crate::office_templates::format(path)?;
                if !matches!(format, "dotx" | "xltx") {
                    return Err("Choose a DOTX or XLTX template".into());
                }
                let original = crate::office::Package::read(path)?;
                let fingerprint = crate::office::fingerprint(&original.original);
                if r["expectedFingerprint"]
                    .as_str()
                    .is_some_and(|v| v != fingerprint)
                {
                    return Err(
                        "This template changed. Preview it again before creating a file.".into(),
                    );
                }
                let target = crate::office_templates::document_format(format)?;
                let package = crate::office_templates::convert(original, target)?;
                let mut staged = Core::new();
                staged.dispatch(json!({"op":"new","blank":true}))?;
                let initial = staged.file_states();
                staged.import_office_package(package, target)?;
                staged.workspace["title"] = json!(std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled"));
                staged.workspace["revision"] = json!(0);
                staged.history.clear();
                if op == "inspectTemplate" {
                    let mut preview = workspace_description(&staged.workspace);
                    preview["format"] = json!(format);
                    preview["fingerprint"] = json!(fingerprint);
                    if target == "xlsx" {
                        let snapshot = staged.snapshot();
                        let sheet = snapshot["workspace"]["sheets"][0]["id"].as_str().unwrap();
                        preview["sample"] = json!((0..7)
                            .map(|row| (0..4)
                                .map(|col| {
                                    let cell = address(row, col);
                                    snapshot["formatted"][sheet][&cell]["text"]
                                        .as_str()
                                        .map(String::from)
                                        .unwrap_or_else(|| {
                                            snapshot["workspace"]["sheets"][0]["cells"][&cell]
                                                .as_str()
                                                .unwrap_or("")
                                                .to_string()
                                        })
                                })
                                .collect::<Vec<_>>())
                            .collect::<Vec<_>>());
                    }
                    return Ok(preview);
                }
                *self = staged;
                let mut result = self.snapshot();
                result["initialFileStates"] = initial;
                result["newFileKind"] = json!(target);
                Ok(result)
            }
            "importOffice" => {
                let path = text(&r, "path")?;
                let package = crate::office::Package::read(path)?;
                let lower = path.to_lowercase();
                let format = crate::office_templates::format(&lower)?;
                let package = if matches!(format, "dotx" | "xltx") {
                    crate::office_templates::convert(package, format)?
                } else {
                    package
                };
                self.import_office_package(
                    package,
                    crate::office_templates::document_format(format)?,
                )
            }
            "exportOffice" => {
                let format = text(&r, "format")?;
                let content_format = crate::office_templates::document_format(format)?;
                let meta = &self.workspace[if content_format == "docx" {
                    "officeDocument"
                } else {
                    "officeWorkbook"
                }];
                let pkg = meta["packageId"]
                    .as_str()
                    .and_then(|id| self.packages.get(id))
                    .map(AsRef::as_ref);
                let baseline = meta.get("baseline");
                let bytes = match content_format {
                    "docx" if meta.get("tracking").is_some() => {
                        if baseline != Some(&self.workspace["document"])
                            || self.resolved_document()? != self.workspace["document"]
                        {
                            return Err("Tracked document has unrecorded edits".into());
                        }
                        pkg.ok_or("Tracked source package is missing")?
                            .original
                            .clone()
                    }
                    "docx" => crate::docx::export(&self.resolved_document()?, pkg, baseline)?,
                    "xlsx" => crate::xlsx::export(
                        &self.workspace["sheets"],
                        &self.calculator.values,
                        &self.calculator.cell_errors,
                        pkg,
                        baseline,
                    )?,
                    _ => return Err("Unsupported Office format".into()),
                };
                let bytes = crate::office_templates::convert(
                    crate::office::Package::from_bytes(bytes)?,
                    format,
                )?
                .original;
                let fingerprint = crate::office::write_checked_bytes(
                    text(&r, "path")?,
                    &bytes,
                    r["expectedFingerprint"].as_str(),
                )?;
                Ok(
                    json!({"saved":true,"bytes":bytes.len(),"fullFidelity":false,"revision":self.workspace["revision"],"fingerprint":fingerprint,"contentSignature":self.file_states()[content_format]}),
                )
            }
            "context" => {
                let mut sheets = vec![];
                let mut calculated = serde_json::Map::new();
                let mut total = 0;
                for sheet in self.workspace["sheets"].as_array().unwrap() {
                    let mut cells = serde_json::Map::new();
                    let mut values = serde_json::Map::new();
                    let id = sheet["id"].as_str().unwrap();
                    for (a, value) in sheet["cells"].as_object().unwrap().iter().take(400) {
                        if total >= 2000 {
                            break;
                        }
                        total += 1;
                        cells.insert(a.clone(), value.clone());
                        values.insert(a.clone(), self.calculator.values[id][a].clone());
                    }
                    sheets.push(json!({"id":id,"name":sheet["name"],"sampleCells":cells,"totalPopulatedCells":sheet["cells"].as_object().unwrap().len()}));
                    calculated.insert(id.into(), json!(values));
                }
                Ok(
                    json!({"workspaceId":self.workspace["id"],"title":self.workspace["title"],"revision":self.workspace["revision"],"document":document_text(&self.resolved_document()?).chars().take(60000).collect::<String>(),"sheets":sheets,"calculated":calculated,"selection":r["selection"],"note":"Cell samples are limited to 400 per worksheet, 2000 total. Document preview is limited to 60000 characters. Use read_range for exact source values."}),
                )
            }
            _ => Err("Unknown operation".into()),
        }
    }
}
/// Whether a checked cell entry may be committed, or the error to show.
fn entry_allowed(checked: &Value, r: &Value) -> Result<(), String> {
    if checked["valid"] != true
        && checked["shouldAlert"] == true
        && !(checked["canOverride"] == true && r["allowInvalid"] == true)
    {
        return Err(checked["reason"]
            .as_str()
            .or(checked["rule"]["error"].as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("This value does not match the cell's validation rule.")
            .into());
    }
    Ok(())
}
fn display_value(v: &Value) -> String {
    if v.is_null() {
        String::new()
    } else {
        v.as_str().map(String::from).unwrap_or(v.to_string())
    }
}
fn atomic_save(path: &str, text: &str) -> Result<(), String> {
    if text.len() > 128_000_000 {
        return Err("File exceeds 128 MB".into());
    }
    crate::office::write_bytes(path, text.as_bytes())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolving_export_does_not_add_fields_to_unlinked_text() {
        let mut core = Core::new();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/word-conversion.docx"
        );
        core.dispatch(json!({"op":"importOffice","path":path}))
            .unwrap();
        assert_eq!(
            core.resolved_document().unwrap(),
            core.workspace["document"]
        );
        let output = std::env::temp_dir().join("folio-conversion-untouched.docx");
        core.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
            .unwrap();
        assert_eq!(std::fs::read(output).unwrap(), std::fs::read(path).unwrap());
    }
    #[test]
    fn atomic_proposal_and_history() {
        let mut c = Core::new();
        let rev = c.workspace["revision"].clone();
        let p = json!({"baseRevision":rev,"summary":"Test","cells":[{"sheetId":"budget","address":"B4","value":"3000"}],"appendParagraphs":["Verified"]});
        let proposed = c.dispatch(json!({"op":"propose","proposal":p})).unwrap();
        assert_eq!(c.workspace["sheets"][0]["cells"]["B4"], "2400");
        c.dispatch(json!({"op":"apply","proposal":proposed}))
            .unwrap();
        assert_eq!(c.snapshot()["calculated"]["budget"]["B9"], json!(20600.0));
        assert!(c.dispatch(json!({"op":"apply","proposal":p})).is_err());
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["sheets"][0]["cells"]["B4"], "2400");
        assert!(!document_text(&c.workspace["document"]).contains("Verified"));
    }
    #[test]
    fn bad_cell_rolls_back() {
        let mut c = Core::new();
        let before = c.workspace.clone();
        let p = json!({"baseRevision":0,"summary":"bad","cells":[{"sheetId":"budget","address":"B4","value":"3000"},{"sheetId":"bad","address":"A1","value":"bad"}]});
        assert!(c.dispatch(json!({"op":"apply","proposal":p})).is_err());
        assert_eq!(c.workspace, before);
    }
    #[test]
    fn legacy_fixture_roundtrip_and_export() {
        let mut c = Core::new();
        let path = std::env::temp_dir().join(format!(
            "folio-{}.folio",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        c.dispatch(json!({"op":"save","path":path})).unwrap();
        let original = c.workspace.clone();
        c.dispatch(json!({"op":"load","path":path})).unwrap();
        assert_eq!(c.workspace, original);
        assert!(c
            .dispatch(json!({"op":"exportCsv","sheetId":"budget"}))
            .unwrap()["text"]
            .as_str()
            .unwrap()
            .contains("20000"));
        let _ = fs::remove_file(path);
    }
}
#[cfg(test)]
mod office_tests {
    use super::*;
    #[test]
    fn office_packages_survive_workspace_save_and_linked_data_tracks_history() {
        let dir = std::env::temp_dir().join(format!("folio-office-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut core = Core::new();
        let source = dir.join("source.xlsx");
        fs::write(&source, include_bytes!("../../test/fixtures/source.xlsx")).unwrap();
        core.dispatch(json!({"op":"importOffice","path":source}))
            .unwrap();
        core.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["B2"],"style":{"bold":false,"italic":true,"numFmt":"0.00%"}})).unwrap();
        let proposal=core.dispatch(json!({"op":"propose","proposal":{"baseRevision":core.workspace["revision"],"summary":"Live table","linkedBlock":{"sheetId":"office-1","range":"A1:C4","mode":"table","title":"Budget"}}})).unwrap();
        core.dispatch(json!({"op":"apply","proposal":proposal}))
            .unwrap();
        core.dispatch(
            json!({"op":"cells","cells":[{"sheetId":"office-1","address":"B2","value":"3100"}]}),
        )
        .unwrap();
        assert!(document_text(&core.resolved_document().unwrap()).contains("3100"));
        core.dispatch(json!({"op":"undo"})).unwrap();
        assert!(document_text(&core.resolved_document().unwrap()).contains("2400"));
        core.dispatch(json!({"op":"save","path":dir.join("session.folio")}))
            .unwrap();
        let mut reopened = Core::new();
        reopened
            .dispatch(json!({"op":"load","path":dir.join("session.folio")}))
            .unwrap();
        assert_eq!(reopened.packages.len(), 1);
        reopened.dispatch(json!({"op":"exportOffice","format":"xlsx","path":std::env::temp_dir().join("folio-verified-styled.xlsx")})).unwrap();
        reopened.dispatch(json!({"op":"exportOffice","format":"docx","path":std::env::temp_dir().join("folio-verified-linked.docx")})).unwrap();
        let exported = crate::office::Package::read(
            std::env::temp_dir()
                .join("folio-verified-styled.xlsx")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let styles = crate::xlsx::import(&exported).unwrap().0;
        assert_eq!(styles[0]["cellStyles"]["B2"]["italic"], true);
        assert_eq!(styles[0]["cellStyles"]["B2"]["bold"], false);
    }
}
