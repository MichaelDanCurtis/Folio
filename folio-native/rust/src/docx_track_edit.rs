//! Record exact rich-text edits in a Word paragraph, retaining prior revisions.
//! This layer is independent of editor selection and does not infer authorship.
use crate::office::*;
use roxmltree::Node;
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Revision {
    tag: String,
    id: String,
    author: String,
    date: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Atom {
    ch: char,
    kind: &'static str,
    props: Arc<str>,
    shell: Arc<str>,
    history: Arc<Vec<Revision>>,
}
impl Atom {
    fn deleted(&self) -> bool {
        self.history.iter().any(|r| r.tag == "del")
    }
    fn insertion(&self) -> Option<&Revision> {
        self.history.iter().rev().find(|r| r.tag == "ins")
    }
}
#[derive(Clone, Debug)]
pub struct Author {
    pub name: String,
    pub date: String,
    pub enabled: bool,
    pub formatting: bool,
}
impl Author {
    fn validate(&self) -> Result<(), String> {
        if self.enabled
            && (self.name.trim().is_empty()
                || self.name.len() > 512
                || self.name.chars().any(char::is_control))
        {
            return Err("Enter a review author before tracking changes".into());
        }
        if chrono::DateTime::parse_from_rfc3339(&self.date).is_err() {
            return Err("Invalid revision date".into());
        }
        Ok(())
    }
}
struct Ids {
    used: HashSet<String>,
    next: u32,
    emitted: HashSet<String>,
}
impl Ids {
    fn new(xml: &str) -> Result<Self, String> {
        let d = parse(xml)?;
        let used = d
            .descendants()
            .filter_map(|n| n.attribute((W, "id")).map(String::from))
            .collect();
        Ok(Self {
            used,
            next: 0,
            emitted: HashSet::new(),
        })
    }
    fn take(&mut self) -> Result<String, String> {
        loop {
            let value = self.next.to_string();
            self.next = self
                .next
                .checked_add(1)
                .ok_or("Revision identifiers exhausted")?;
            if self.used.insert(value.clone()) {
                return Ok(value);
            }
        }
    }
    fn revision(&mut self, tag: &str, a: &Author) -> Result<Revision, String> {
        Ok(Revision {
            tag: tag.into(),
            id: self.take()?,
            author: a.name.clone(),
            date: a.date.clone(),
        })
    }
}
fn named(n: Node, tag: &str) -> bool {
    n.has_tag_name((W, tag))
}
fn property(xml: &str, n: Node, tag: &str) -> String {
    n.children()
        .find(|n| named(*n, tag))
        .map(|n| scoped_record(xml, n))
        .unwrap_or_default()
}
fn clean_props(props: &str) -> Result<String, String> {
    if props.is_empty() {
        return Ok(String::new());
    }
    let d = parse(props)?;
    let edits = d
        .descendants()
        .filter(|n| {
            (named(*n, "rPrChange") || named(*n, "pPrChange"))
                && !n
                    .ancestors()
                    .skip(1)
                    .any(|a| named(a, "rPrChange") || named(a, "pPrChange"))
        })
        .map(|n| (n.range(), String::new()))
        .collect();
    Ok(replace_ranges(props, edits))
}
fn signature(xml: &str) -> Result<String, String> {
    if xml.is_empty() {
        return Ok(String::new());
    }
    fn visit(n: Node, out: &mut String) {
        if n.is_text() {
            if !n.text().unwrap_or("").trim().is_empty() {
                out.push_str(&format!("T{:?}", n.text()));
            }
            return;
        }
        if !n.is_element() {
            return;
        }
        out.push_str(&format!(
            "E{:?}:{}",
            n.tag_name().namespace(),
            n.tag_name().name()
        ));
        let mut attrs = n
            .attributes()
            .map(|a| (a.namespace(), a.name(), a.value()))
            .collect::<Vec<_>>();
        attrs.sort();
        out.push_str(&format!("{attrs:?}"));
        for c in n.children() {
            visit(c, out);
        }
        out.push('/');
    }
    let d = parse(xml)?;
    let mut out = String::new();
    // Missing and empty properties represent the same direct property set.
    for n in d.root_element().children() {
        visit(n, &mut out);
    }
    Ok(out)
}
fn read(xml: &str) -> Result<Vec<Atom>, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    if !named(root, "p") {
        return Err("Expected a Word paragraph".into());
    }
    fn walk(
        xml: &str,
        n: Node,
        history: &mut Vec<Revision>,
        out: &mut Vec<Atom>,
    ) -> Result<(), String> {
        if !n.is_element() {
            return Ok(());
        }
        if named(n, "pPr") {
            return Ok(());
        }
        if named(n, "ins") || named(n, "del") {
            history.push(Revision {
                tag: n.tag_name().name().into(),
                id: n
                    .attribute((W, "id"))
                    .ok_or("Revision ID is missing")?
                    .into(),
                author: n.attribute((W, "author")).unwrap_or("").into(),
                date: n.attribute((W, "date")).unwrap_or("").into(),
            });
            for c in n.children() {
                walk(xml, c, history, out)?;
            }
            history.pop();
            return Ok(());
        }
        if named(n, "r") {
            let props: Arc<str> = property(xml, n, "rPr").into();
            let full = scoped_record(xml, n);
            let d = parse(&full)?;
            let shell: Arc<str> = replace_inner(&full, d.root_element(), "").into();
            let history = Arc::new(history.clone());
            for c in n.children().filter(|c| c.is_element()) {
                if named(c, "rPr") {
                    continue;
                }
                let (kind, value) = if named(c, "t") || named(c, "delText") {
                    ("text", c.text().unwrap_or("").to_string())
                } else if named(c, "tab") {
                    ("tab", "\t".into())
                } else if named(c, "br") || named(c, "cr") {
                    if c.attributes()
                        .any(|a| a.name() != "type" || a.value() != "textWrapping")
                    {
                        return Err(
                            "Tracking edits around page/column breaks needs additional support"
                                .into(),
                        );
                    }
                    ("break", "\n".into())
                } else {
                    return Err("Tracking this inline Word object needs additional support".into());
                };
                for ch in value.chars() {
                    out.push(Atom {
                        ch,
                        kind,
                        props: props.clone(),
                        shell: shell.clone(),
                        history: history.clone(),
                    });
                }
            }
            return Ok(());
        }
        Err("Tracking edits through links, bookmarks, comments or other inline objects needs additional support".into())
    }
    let mut out = vec![];
    for n in root.children() {
        walk(xml, n, &mut vec![], &mut out)?;
    }
    Ok(out)
}
fn positive(atoms: &[Atom]) -> String {
    atoms
        .iter()
        .filter(|a| !a.deleted())
        .map(|a| a.ch)
        .collect()
}
type PreservationCache = Option<(&'static str, Arc<str>, Arc<str>, Arc<str>, Arc<str>)>;
fn preserve_properties(
    desired: &mut Atom,
    source: &Atom,
    projection: &Atom,
    cache: &mut PreservationCache,
) -> Result<(), String> {
    let result = if let Some((_, _, _, _, result)) =
        cache.as_ref().filter(|(kind, source_props, old, new, _)| {
            *kind == desired.kind
                && *source_props == source.props
                && *old == projection.props
                && *new == desired.props
        }) {
        result.clone()
    } else {
        let value: Arc<str> = crate::docx_track_properties::overlay(
            &clean_props(&source.props)?,
            &projection.props,
            &desired.props,
            if desired.kind == "paragraph" {
                "pPr"
            } else {
                "rPr"
            },
        )?
        .into();
        *cache = Some((
            desired.kind,
            source.props.clone(),
            projection.props.clone(),
            desired.props.clone(),
            value.clone(),
        ));
        value
    };
    desired.props = result;
    Ok(())
}
fn inherit_insert(
    desired: &mut Atom,
    source: &[&Atom],
    projection: &[Atom],
    at: usize,
    cache: &mut PreservationCache,
) -> Result<(), String> {
    // A new paragraph inherits its paragraph settings. New text at the end of
    // a paragraph inherits the preceding run; at its start it inherits the next.
    let anchor = if desired.kind == "paragraph" {
        (at..source.len()).find(|&i| source[i].kind == "paragraph")
    } else {
        (at < source.len() && source[at].kind != "paragraph")
            .then_some(at)
            .or_else(|| at.checked_sub(1).filter(|&i| source[i].kind != "paragraph"))
    };
    if let Some(i) = anchor {
        preserve_properties(desired, source[i], &projection[i], cache)?;
        if desired.kind != "paragraph" {
            desired.shell = source[i].shell.clone();
        }
    } else if desired.kind != "paragraph" && at < source.len() && source[at].kind == "paragraph" {
        // Empty paragraphs take their typing format from the paragraph mark.
        // The editor need not display every property of that mark.
        fn mark_run(atom: &Atom) -> Result<Atom, String> {
            let mut run = atom.clone();
            run.kind = "text";
            run.props = if atom.props.is_empty() {
                "".into()
            } else {
                property(&atom.props, parse(&atom.props)?.root_element(), "rPr").into()
            };
            Ok(run)
        }
        preserve_properties(
            desired,
            &mark_run(source[at])?,
            &mark_run(&projection[at])?,
            cache,
        )?;
    }
    Ok(())
}
fn metadata(r: &Revision) -> String {
    format!(
        "w:id=\"{}\" w:author=\"{}\"{}",
        esc(&r.id),
        esc(&r.author),
        if r.date.is_empty() {
            String::new()
        } else {
            format!(" w:date=\"{}\"", esc(&r.date))
        }
    )
}
fn changed_props(
    before: &str,
    after: &str,
    tag: &str,
    author: &Author,
    ids: &mut Ids,
) -> Result<String, String> {
    if tag == "pPr" {
        fn split(xml: &str) -> Result<(String, String), String> {
            if xml.is_empty() {
                return Ok((String::new(), String::new()));
            }
            let d = parse(xml)?;
            let mark = property(xml, d.root_element(), "rPr");
            let plain = replace_ranges(
                xml,
                d.root_element()
                    .children()
                    .filter(|n| named(*n, "rPr"))
                    .map(|n| (n.range(), String::new()))
                    .collect(),
            );
            Ok((plain, mark))
        }
        let (old, old_mark) = split(before)?;
        let (new, new_mark) = split(after)?;
        let mark = changed_props(&old_mark, &new_mark, "rPr", author, ids)?;
        let paragraph = changed_simple_props(&old, &new, tag, author, ids)?;
        if mark.is_empty() {
            return Ok(paragraph);
        }
        return Ok(crate::docx::set_prop(&paragraph, "", "rPr", mark));
    }
    changed_simple_props(before, after, tag, author, ids)
}
fn changed_simple_props(
    before: &str,
    after: &str,
    tag: &str,
    author: &Author,
    ids: &mut Ids,
) -> Result<String, String> {
    let clean = clean_props(before)?;
    if signature(&clean)? == signature(after)? {
        return Ok(before.into());
    }
    let previous = if before.is_empty() {
        format!("<w:{tag} xmlns:w=\"{W}\"/>")
    } else {
        before.into()
    };
    let existing = if before.is_empty() {
        None
    } else {
        let d = parse(before)?;
        d.root_element()
            .children()
            .find(|n| named(*n, &format!("{tag}Change")))
            .map(|n| scoped_record(before, n))
    };
    let change = if let Some(existing) = existing {
        existing
    } else if author.enabled && author.formatting {
        let r = ids.revision(&format!("{tag}Change"), author)?;
        format!(
            "<w:{tag}Change xmlns:w=\"{W}\" {}>{previous}</w:{tag}Change>",
            metadata(&r)
        )
    } else {
        String::new()
    };
    let base = if after.is_empty() {
        format!("<w:{tag} xmlns:w=\"{W}\"/>")
    } else {
        after.into()
    };
    let d = parse(&base)?;
    let root = d.root_element();
    let contents = root.children().map(|n| raw(&base, n)).collect::<String>();
    Ok(replace_inner(&base, root, &(contents + &change)))
}
fn render(atoms: &[Atom], ids: &mut Ids) -> Result<String, String> {
    let mut output = String::new();
    let mut stack: Vec<Revision> = vec![];
    let mut at = 0;
    while at < atoms.len() {
        let atom = &atoms[at];
        let common = stack
            .iter()
            .zip(atom.history.iter())
            .take_while(|(a, b)| a == b)
            .count();
        for r in stack[common..].iter().rev() {
            output.push_str(&format!("</w:{}>", r.tag));
        }
        stack.truncate(common);
        for original in &atom.history[common..] {
            let mut r = original.clone();
            if !ids.emitted.insert(r.id.clone()) {
                r.id = ids.take()?;
                ids.emitted.insert(r.id.clone());
            }
            output.push_str(&format!("<w:{} xmlns:w=\"{W}\" {}>", r.tag, metadata(&r)));
            stack.push(original.clone());
        }
        let mut end = at + 1;
        while end < atoms.len()
            && atoms[end].history == atom.history
            && atoms[end].props == atom.props
            && atoms[end].shell == atom.shell
            && atoms[end].kind == atom.kind
        {
            end += 1;
        }
        let text = atoms[at..end].iter().map(|a| a.ch).collect::<String>();
        let contents = match atom.kind {
            "text" => {
                let tag = if atom.deleted() { "delText" } else { "t" };
                format!(
                    "<w:{tag} xmlns:w=\"{W}\" xml:space=\"preserve\">{}</w:{tag}>",
                    esc(&text)
                )
            }
            "tab" => format!("<w:tab xmlns:w=\"{W}\"/>").repeat(end - at),
            "break" => format!("<w:br xmlns:w=\"{W}\"/>").repeat(end - at),
            _ => return Err("Unknown tracked text atom".into()),
        };
        let mut props = atom.props.to_string();
        if !props.is_empty() {
            let d = parse(&props)?;
            let mut edits = vec![];
            for n in d.descendants().filter(|n| named(*n, "rPrChange")) {
                if let Some(a) = n
                    .attributes()
                    .find(|a| a.namespace() == Some(W) && a.name() == "id")
                {
                    if !ids.emitted.insert(a.value().to_string()) {
                        let id = ids.take()?;
                        ids.emitted.insert(id.clone());
                        let attr = &props[a.range()];
                        let name = attr.split('=').next().unwrap().trim();
                        edits.push((a.range(), format!("{name}=\"{id}\"")));
                    }
                }
            }
            props = replace_ranges(&props, edits);
        }
        let d = parse(&atom.shell)?;
        output.push_str(&replace_inner(
            &atom.shell,
            d.root_element(),
            &(props + &contents),
        ));
        at = end;
    }
    for r in stack.iter().rev() {
        output.push_str(&format!("</w:{}>", r.tag));
    }
    Ok(output)
}
/// Current-text edits use UTF-16, matching the document editor. They are sorted,
/// disjoint and exact; repeated text is never located by a substring search.
/// `after` supplies the desired rich runs and paragraph properties.
#[cfg(test)]
pub fn record(
    before: &str,
    after: &str,
    edits: &Value,
    author: &Author,
    document_xml: &str,
) -> Result<String, String> {
    let mut ids = Ids::new(document_xml)?;
    let out = record_atoms(
        read(before)?,
        read(after)?,
        edits,
        author,
        document_xml,
        None,
    )?;
    let a = parse(before)?;
    let b = parse(after)?;
    let old_props = property(before, a.root_element(), "pPr");
    let new_props = property(after, b.root_element(), "pPr");
    // Paragraph break revisions need the structural recorder, not this run layer.
    if !old_props.is_empty() {
        let d = parse(&old_props)?;
        if d.descendants()
            .any(|n| named(n, "ins") || named(n, "del") || named(n, "sectPr"))
        {
            return Err(
                "Tracked paragraph or section boundaries require structural edit recording".into(),
            );
        }
    }
    for a in &out {
        for r in a.history.iter() {
            ids.used.insert(r.id.clone());
        }
        if !a.props.is_empty() {
            let d = parse(&a.props)?;
            for id in d.descendants().filter_map(|n| n.attribute((W, "id"))) {
                ids.used.insert(id.into());
            }
        }
    }
    let props = changed_props(&old_props, &new_props, "pPr", author, &mut ids)?;
    let output = replace_inner(after, b.root_element(), &(props + &render(&out, &mut ids)?));
    parse(&output)?;
    Ok(output)
}
fn record_atoms(
    old: Vec<Atom>,
    next: Vec<Atom>,
    edits: &Value,
    author: &Author,
    document_xml: &str,
    projection: Option<Vec<Atom>>,
) -> Result<Vec<Atom>, String> {
    author.validate()?;
    if next.iter().any(|a| !a.history.is_empty()) {
        return Err(
            "The replacement must contain current content, not pre-authored revisions".into(),
        );
    }
    let visible = old.iter().filter(|a| !a.deleted()).collect::<Vec<_>>();
    if let Some(projected) = &projection {
        if visible.len() != projected.len()
            || visible
                .iter()
                .zip(projected)
                .any(|(a, b)| a.ch != b.ch || a.kind != b.kind || !b.history.is_empty())
        {
            return Err("The editor projection no longer matches the source revisions".into());
        }
    }
    let mut model_index = 0;
    let mut preservation_cache = None;
    let old_text = positive(&old);
    let expected = positive(&next);
    struct Edit {
        start: usize,
        end: usize,
        text: String,
    }
    let mut parsed = vec![];
    let mut last = 0;
    let mut boundaries = HashSet::from([0]);
    let mut len = 0;
    for c in old_text.chars() {
        len += c.len_utf16();
        boundaries.insert(len);
    }
    for e in edits.as_array().ok_or("Exact text edits are required")? {
        let start = e["start"].as_u64().ok_or("Invalid edit start")? as usize;
        let end = e["end"].as_u64().ok_or("Invalid edit end")? as usize;
        if start < last || end < start || !boundaries.contains(&start) || !boundaries.contains(&end)
        {
            return Err("Tracked edits overlap or split a Unicode character".into());
        }
        let text = e["text"]
            .as_str()
            .ok_or("Invalid replacement text")?
            .to_string();
        parsed.push(Edit { start, end, text });
        last = end;
    }
    let utf16 = old_text.encode_utf16().collect::<Vec<_>>();
    let mut applied = utf16.clone();
    for e in parsed.iter().rev() {
        applied.splice(e.start..e.end, e.text.encode_utf16());
    }
    if String::from_utf16(&applied).map_err(|_| "Invalid UTF-16 edit")? != expected {
        return Err("Exact tracked edits do not match the replacement paragraph".into());
    }
    let mut ids = Ids::new(document_xml)?;
    // Paragraph-scoped tests/callers may pass only the body; always reserve old IDs.
    for a in &old {
        for r in a.history.iter() {
            ids.used.insert(r.id.clone());
        }
    }
    let mut out = vec![];
    let mut offset = 0;
    let mut edit_index = 0;
    let mut next_index = 0;
    let mut deletion: Option<Revision> = None;
    let mut pending_insertions = vec![];
    let mut format_cache: Option<(String, Arc<str>, Arc<str>)> = None;
    for (old_index, original) in old.iter().enumerate() {
        if original.deleted() {
            out.push(original.clone());
            continue;
        }
        while edit_index < parsed.len() && parsed[edit_index].start == offset {
            let e = &parsed[edit_index];
            let existing = original
                .insertion()
                .filter(|r| r.author == author.name)
                .cloned()
                .or_else(|| {
                    old[..old_index]
                        .iter()
                        .rev()
                        .find(|a| !a.deleted())
                        .and_then(|a| a.insertion())
                        .filter(|r| r.author == author.name)
                        .cloned()
                });
            let insertion = if author.enabled && !e.text.is_empty() {
                Some(existing.unwrap_or(ids.revision("ins", author)?))
            } else {
                None
            };
            for c in e.text.chars() {
                let mut a = next
                    .get(next_index)
                    .filter(|a| a.ch == c)
                    .cloned()
                    .ok_or("Replacement run boundary mismatch")?;
                next_index += 1;
                if let Some(projected) = &projection {
                    inherit_insert(
                        &mut a,
                        &visible,
                        projected,
                        model_index,
                        &mut preservation_cache,
                    )?;
                }
                if let Some(r) = &insertion {
                    Arc::make_mut(&mut a.history).push(r.clone());
                }
                pending_insertions.push(a);
            }
            deletion = if author.enabled && e.end > e.start {
                Some(ids.revision("del", author)?)
            } else {
                None
            };
            if e.end == offset {
                out.append(&mut pending_insertions);
                edit_index += 1;
                deletion = None;
            } else {
                break;
            }
        }
        let removed = parsed
            .get(edit_index)
            .is_some_and(|e| e.start <= offset && offset < e.end);
        if removed {
            if author.enabled && original.insertion().is_none_or(|r| r.author != author.name) {
                let mut a = original.clone();
                Arc::make_mut(&mut a.history)
                    .push(deletion.clone().ok_or("Deletion metadata missing")?);
                out.push(a);
            }
        } else {
            let mut desired = next
                .get(next_index)
                .filter(|a| a.ch == original.ch && a.kind == original.kind)
                .cloned()
                .ok_or("Unchanged text moved outside the exact edit journal")?;
            if let Some(projected) = &projection {
                preserve_properties(
                    &mut desired,
                    original,
                    &projected[model_index],
                    &mut preservation_cache,
                )?;
            }
            next_index += 1;
            let mut a = original.clone();
            // Formatting an author's own pending insertion changes that insertion.
            if a.insertion().is_some_and(|r| r.author == author.name) {
                a.props = desired.props.clone();
            } else {
                let props = if let Some((before, after, result)) =
                    format_cache.as_ref().filter(|(before, after, _)| {
                        before == &format!("{}:{}", a.kind, a.props) && after == &desired.props
                    }) {
                    let _ = (before, after);
                    result.clone()
                } else {
                    let result: Arc<str> = changed_props(
                        &a.props,
                        &desired.props,
                        if a.kind == "paragraph" { "pPr" } else { "rPr" },
                        author,
                        &mut ids,
                    )?
                    .into();
                    format_cache = Some((
                        format!("{}:{}", a.kind, a.props),
                        desired.props.clone(),
                        result.clone(),
                    ));
                    result
                };
                a.props = props;
            }
            out.push(a);
        }
        model_index += 1;
        offset += original.ch.len_utf16();
        if removed && parsed.get(edit_index).is_some_and(|e| e.end == offset) {
            out.append(&mut pending_insertions);
            edit_index += 1;
            deletion = None;
        }
    }
    while edit_index < parsed.len() {
        let e = &parsed[edit_index];
        if e.start != offset || e.end != offset {
            return Err("Unconsumed tracked edit".into());
        }
        let prior = out
            .iter()
            .rev()
            .find(|a| !a.deleted())
            .and_then(|a| a.insertion())
            .filter(|r| r.author == author.name)
            .cloned();
        let insertion = if author.enabled && !e.text.is_empty() {
            Some(prior.unwrap_or(ids.revision("ins", author)?))
        } else {
            None
        };
        for c in e.text.chars() {
            let mut a = next
                .get(next_index)
                .filter(|a| a.ch == c)
                .cloned()
                .ok_or("Replacement run boundary mismatch")?;
            next_index += 1;
            if let Some(projected) = &projection {
                inherit_insert(
                    &mut a,
                    &visible,
                    projected,
                    model_index,
                    &mut preservation_cache,
                )?;
            }
            if let Some(r) = &insertion {
                Arc::make_mut(&mut a.history).push(r.clone());
            }
            out.push(a);
        }
        edit_index += 1;
    }
    if next_index != next.len() {
        return Err("Unconsumed replacement text".into());
    }
    Ok(out)
}
/// Editable current view, while the source XML remains authoritative for review.
#[cfg(test)]
pub fn current_view(xml: &str) -> Result<String, String> {
    let atoms = read(xml)?
        .into_iter()
        .filter(|a| !a.deleted())
        .map(|mut a| {
            Arc::make_mut(&mut a.history).clear();
            a.props = clean_props(&a.props)?.into();
            Ok(a)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let d = parse(xml)?;
    let props = clean_props(&property(xml, d.root_element(), "pPr"))?;
    let mut ids = Ids::new(xml)?;
    Ok(replace_inner(
        xml,
        d.root_element(),
        &(props + &render(&atoms, &mut ids)?),
    ))
}

fn story_atoms(paragraphs: &[String]) -> Result<Vec<Atom>, String> {
    let mut result = vec![];
    for xml in paragraphs {
        let d = parse(xml)?;
        let p = d.root_element();
        let mut props = property(xml, p, "pPr");
        let mut history = vec![];
        if !props.is_empty() {
            let pd = parse(&props)?;
            let root = pd.root_element();
            let mut edits = vec![];
            if root
                .descendants()
                .any(|n| named(n, "sectPr") || named(n, "moveFrom") || named(n, "moveTo"))
            {
                return Err("Tracked section/move boundaries need additional support".into());
            }
            if let Some(rp) = root.children().find(|n| named(*n, "rPr")) {
                for n in rp
                    .children()
                    .filter(|n| named(*n, "ins") || named(*n, "del"))
                {
                    history.push(Revision {
                        tag: n.tag_name().name().into(),
                        id: n
                            .attribute((W, "id"))
                            .ok_or("Paragraph revision ID missing")?
                            .into(),
                        author: n.attribute((W, "author")).unwrap_or("").into(),
                        date: n.attribute((W, "date")).unwrap_or("").into(),
                    });
                    edits.push((n.range(), String::new()));
                }
            }
            props = replace_ranges(&props, edits);
        }
        result.extend(read(xml)?);
        result.push(Atom {
            ch: '\n',
            kind: "paragraph",
            props: props.into(),
            shell: replace_inner(xml, p, "").into(),
            history: Arc::new(history),
        });
    }
    Ok(result)
}
fn render_story(atoms: &[Atom], document_xml: &str) -> Result<Vec<String>, String> {
    let mut ids = Ids::new(document_xml)?;
    for a in atoms {
        for r in a.history.iter() {
            ids.used.insert(r.id.clone());
        }
        if !a.props.is_empty() {
            let d = parse(&a.props)?;
            for id in d.descendants().filter_map(|n| n.attribute((W, "id"))) {
                ids.used.insert(id.into());
            }
        }
    }
    let mut paragraphs = vec![];
    let mut start = 0;
    for (i, atom) in atoms
        .iter()
        .enumerate()
        .filter(|(_, a)| a.kind == "paragraph")
    {
        let mut props = atom.props.to_string();
        if !atom.history.is_empty() {
            if props.is_empty() {
                props = format!("<w:pPr xmlns:w=\"{W}\"/>");
            }
            let pd = parse(&props)?;
            let ppr = pd.root_element();
            let mut marks = String::new();
            for original in atom.history.iter() {
                let mut r = original.clone();
                if !ids.emitted.insert(r.id.clone()) {
                    r.id = ids.take()?;
                    ids.emitted.insert(r.id.clone());
                }
                marks.push_str(&format!("<w:{} xmlns:w=\"{W}\" {}/>", r.tag, metadata(&r)));
            }
            let changes = if let Some(rpr) = ppr.children().find(|n| named(*n, "rPr")) {
                let inner = rpr.children().map(|n| raw(&props, n)).collect::<String>();
                replace_inner(&props, rpr, &(marks + &inner))
            } else {
                // rPr follows ordinary paragraph settings and precedes sectPr/pPrChange.
                let at = ppr
                    .children()
                    .find(|n| named(*n, "sectPr") || named(*n, "pPrChange"))
                    .map(|n| n.range().start)
                    .unwrap_or_else(|| {
                        ppr.range().end
                            - raw(&props, ppr)
                                .rfind("</")
                                .map(|x| raw(&props, ppr).len() - x)
                                .unwrap_or(0)
                    });
                let rpr = format!("<w:rPr xmlns:w=\"{W}\">{marks}</w:rPr>");
                if raw(&props, ppr).ends_with("/>") {
                    replace_inner(&props, ppr, &rpr)
                } else {
                    replace_ranges(&props, vec![(at..at, rpr)])
                }
            };
            props = changes;
        }
        let d = parse(&atom.shell)?;
        let content = props + &render(&atoms[start..i], &mut ids)?;
        paragraphs.push(replace_inner(&atom.shell, d.root_element(), &content));
        start = i + 1;
    }
    if start != atoms.len() {
        return Err("Tracked story must end with a paragraph mark".into());
    }
    Ok(paragraphs)
}
/// Record run edits and paragraph splits/joins across a contiguous paragraph
/// story. The final paragraph mark belongs to the document and is never deleted.
#[cfg(test)]
pub fn record_story(
    before: &[String],
    after: &[String],
    edits: &Value,
    author: &Author,
    document_xml: &str,
) -> Result<Vec<String>, String> {
    let old = story_atoms(before)?;
    let next = story_atoms(after)?;
    let length = positive(&old).encode_utf16().count();
    if length == 0
        || edits
            .as_array()
            .ok_or("Exact edits required")?
            .iter()
            .any(|e| e["end"].as_u64().is_none_or(|end| end as usize >= length))
    {
        return Err("The final paragraph mark must remain intact".into());
    }
    render_story(
        &record_atoms(old, next, edits, author, document_xml, None)?,
        document_xml,
    )
}
pub fn current_story_view(paragraphs: &[String]) -> Result<Vec<String>, String> {
    let mut visible = vec![];
    for mut a in story_atoms(paragraphs)? {
        if a.deleted() {
            continue;
        }
        Arc::make_mut(&mut a.history).clear();
        a.props = clean_props(&a.props)?.into();
        visible.push(a);
    }
    render_story(&visible, &format!("<root>{}</root>", paragraphs.concat()))
}

pub fn supports_paragraph(xml: &str) -> bool {
    story_atoms(&[xml.into()]).is_ok()
}

/// Editor bridge: compare the editor's before/after formatting projections,
/// then overlay only their changes onto the source. Unmodeled source properties
/// must never be interpreted as properties the user removed.
pub fn record_projected_story(
    source: &[String],
    before: &[String],
    after: &[String],
    edits: &Value,
    author: &Author,
    document_xml: &str,
) -> Result<Vec<String>, String> {
    let old = story_atoms(source)?;
    let length = positive(&old).encode_utf16().count();
    if length == 0
        || edits
            .as_array()
            .ok_or("Exact edits required")?
            .iter()
            .any(|e| e["end"].as_u64().is_none_or(|end| end as usize >= length))
    {
        return Err("The final paragraph mark must remain intact".into());
    }
    render_story(
        &record_atoms(
            old,
            story_atoms(after)?,
            edits,
            author,
            document_xml,
            Some(story_atoms(before)?),
        )?,
        document_xml,
    )
}
