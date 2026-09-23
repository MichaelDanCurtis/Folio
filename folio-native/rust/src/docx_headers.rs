//! Section-owned header/footer stories. Story relationships remain scoped to
//! their own part; body content and unrelated package parts stay intact.
use crate::{docx, docx_page_setup as pages, office::*};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const MAIN_RELS: &str = "word/_rels/document.xml.rels";

fn resolve(part: &str, target: &str) -> Result<String, String> {
    let full = if target.starts_with('/') {
        target.into()
    } else {
        format!(
            "{}/{}",
            part.rsplit_once('/').map(|p| p.0).unwrap_or(""),
            target
        )
    };
    let mut out = vec![];
    for item in full.split('/') {
        match item {
            "" | "." => {}
            ".." => {
                out.pop().ok_or("Invalid header/footer relationship path")?;
            }
            s => out.push(s),
        }
    }
    if out.is_empty() || full.contains('\\') {
        return Err("Invalid header/footer part path".into());
    }
    Ok(out.join("/"))
}
fn rel_path(part: &str) -> String {
    let (dir, name) = part.rsplit_once('/').unwrap_or(("", part));
    format!("{dir}/_rels/{name}.rels")
        .trim_start_matches('/')
        .into()
}
fn empty_rels() -> String {
    format!("<Relationships xmlns=\"{REL_NS}\"/>")
}
fn on(node: Option<roxmltree::Node>) -> bool {
    node.is_some_and(|n| !matches!(n.attribute((W, "val")), Some("0" | "false" | "off")))
}
fn target(r: &Value) -> Result<(usize, &str, &str), String> {
    Ok((
        r["section"].as_u64().ok_or("Choose a section")? as usize,
        r["kind"]
            .as_str()
            .filter(|s| ["header", "footer"].contains(s))
            .ok_or("Choose Header or Footer")?,
        r["variant"]
            .as_str()
            .filter(|s| ["default", "first", "even"].contains(s))
            .ok_or("Choose a page type")?,
    ))
}
#[derive(Clone)]
struct Story {
    direct: bool,
    owner: usize,
    part: Option<String>,
}
fn stories(pkg: &Package, kind: &str, variant: &str) -> Result<Vec<Story>, String> {
    let x = pkg.text("word/document.xml")?;
    let d = parse(x)?;
    let body = pages::document_body(&d)?;
    let rels = pkg
        .text(MAIN_RELS)
        .map(String::from)
        .unwrap_or_else(|_| empty_rels());
    let rd = parse(&rels)?;
    let mut current = Story {
        direct: false,
        owner: 0,
        part: None,
    };
    let mut out = vec![];
    for (i, section) in pages::sections(body).iter().enumerate() {
        let refs = section
            .into_iter()
            .flat_map(|n| n.children())
            .filter(|n| {
                n.has_tag_name((W, format!("{kind}Reference").as_str()))
                    && n.attribute((W, "type")) == Some(variant)
            })
            .collect::<Vec<_>>();
        if refs.len() > 1 {
            return Err("Duplicate header/footer references need repair before editing".into());
        }
        if let Some(n) = refs.first() {
            let id = n
                .attribute((R, "id"))
                .ok_or("Header/footer relationship is missing")?;
            let matches = rd
                .root_element()
                .children()
                .filter(|n| attr(*n, "Id") == Some(id))
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err("Header/footer relationship is missing or ambiguous".into());
            }
            let rel = matches[0];
            if attr(rel, "Type") != Some(&format!("{R}/{kind}"))
                || attr(rel, "TargetMode") == Some("External")
            {
                return Err("This header/footer relationship cannot be edited".into());
            }
            let path = resolve(
                "word/document.xml",
                attr(rel, "Target").ok_or("Missing header/footer target")?,
            )?;
            let xml = pkg.text(&path)?;
            let dom = parse(xml)?;
            if !dom
                .root_element()
                .has_tag_name((W, if kind == "header" { "hdr" } else { "ftr" }))
            {
                return Err("Invalid header/footer part".into());
            }
            current = Story {
                direct: true,
                owner: i,
                part: Some(path),
            };
        } else {
            current.direct = false;
        }
        out.push(current.clone());
    }
    Ok(out)
}
fn normalized_rels(pkg: &Package, part: &str) -> Result<String, String> {
    let x = pkg
        .text(&rel_path(part))
        .map(String::from)
        .unwrap_or_else(|_| empty_rels());
    let d = parse(&x)?;
    let mut edits = vec![];
    for n in d.root_element().children().filter(|n| n.is_element()) {
        if attr(n, "TargetMode") == Some("External") {
            continue;
        }
        if let Some(a) = n.attribute_node("Target") {
            edits.push((
                a.range(),
                format!("Target=\"/{}\"", esc(&resolve(part, a.value())?)),
            ));
        }
    }
    Ok(replace_ranges(&x, edits))
}
/// Project a single story through the existing rich-text codec. Shared style,
/// numbering and settings relationships are temporary and never written into
/// the header's own relationship collection.
fn projection(
    pkg: &Package,
    part: Option<&str>,
    kind: &str,
) -> Result<(Package, Value, HashSet<String>), String> {
    let source = part
        .map(|p| pkg.text(p).map(String::from))
        .transpose()?
        .unwrap_or_else(|| {
            format!(
                "<w:{} xmlns:w=\"{W}\"><w:p/></w:{}>",
                if kind == "header" { "hdr" } else { "ftr" },
                if kind == "header" { "hdr" } else { "ftr" }
            )
        });
    let dom = parse(&source)?;
    let root = dom.root_element();
    let content = root
        .children()
        .map(|n| {
            if n.is_element() {
                scoped_record(&source, n)
            } else {
                raw(&source, n).into()
            }
        })
        .collect::<String>();
    let mut temp = pkg.clone();
    temp.set(
        "word/document.xml",
        format!(
            "<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\"><w:body>{content}</w:body></w:document>"
        ),
    );
    let mut rels = part
        .map(|p| normalized_rels(pkg, p))
        .transpose()?
        .unwrap_or_else(empty_rels);
    let rd = parse(&rels)?;
    let mut ids = rd
        .root_element()
        .children()
        .filter_map(|n| attr(n, "Id").map(String::from))
        .collect::<HashSet<_>>();
    let mut synthetic = HashSet::new();
    let mut added = String::new();
    if let Ok(main) = pkg.text(MAIN_RELS) {
        let d = parse(main)?;
        for n in d.root_element().children().filter(|n| n.is_element()) {
            let Some(kind) = attr(n, "Type").and_then(|s| s.strip_prefix(&format!("{R}/"))) else {
                continue;
            };
            if ![
                "styles",
                "numbering",
                "settings",
                "comments",
                "commentsExtended",
            ]
            .contains(&kind)
                || attr(n, "TargetMode") == Some("External")
            {
                continue;
            }
            let mut i = 1;
            while ids.contains(&format!("folioStoryShared{i}")) {
                i += 1;
            }
            let id = format!("folioStoryShared{i}");
            ids.insert(id.clone());
            synthetic.insert(id.clone());
            let path = resolve(
                "word/document.xml",
                attr(n, "Target").ok_or("Missing shared Word part")?,
            )?;
            added.push_str(&format!(
                "<Relationship xmlns=\"{REL_NS}\" Id=\"{id}\" Type=\"{R}/{kind}\" Target=\"/{}\"/>",
                esc(&path)
            ));
        }
    }
    let inner = rd
        .root_element()
        .children()
        .map(|n| raw(&rels, n))
        .collect::<String>()
        + &added;
    rels = replace_inner(&rels, rd.root_element(), &inner);
    temp.set(MAIN_RELS, rels);
    temp.original = temp.bytes()?;
    let doc = docx::import(&temp)?.0;
    Ok((temp, doc, synthetic))
}
pub fn read(pkg: &Package, r: &Value) -> Result<Value, String> {
    let (section, kind, variant) = target(r)?;
    let all = stories(pkg, kind, variant)?;
    let story = all.get(section).ok_or("Section no longer exists")?;
    let mut out = pages::read(pkg)?;
    let (_, document, _) = projection(pkg, story.part.as_deref(), kind)?;
    let previous = if section > 0 {
        projection(pkg, all[section - 1].part.as_deref(), kind)?.1
    } else {
        document.clone()
    };
    let settings = crate::docx_tabs::part(pkg, "settings")?;
    let even_odd = if let Ok(x) = pkg.text(&settings) {
        on(child(parse(x)?.root_element(), "evenAndOddHeaders"))
    } else {
        false
    };
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let body = pages::document_body(&d)?;
    let first = on(pages::sections(body)[section].and_then(|s| child(s, "titlePg")));
    let key = fingerprint(
        format!(
            "{}\n{}\n{}\n{}",
            xml,
            story.part.as_deref().unwrap_or(""),
            story
                .part
                .as_deref()
                .and_then(|p| pkg.text(p).ok())
                .unwrap_or(""),
            story
                .part
                .as_deref()
                .map(|p| normalized_rels(pkg, p))
                .transpose()?
                .unwrap_or_default()
        )
        .as_bytes(),
    );
    let affected = all
        .iter()
        .enumerate()
        .filter(|(_, s)| s.part == story.part && (story.part.is_some() || s.owner == story.owner))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    out["section"] = json!(section);
    out["kind"] = json!(kind);
    out["variant"] = json!(variant);
    out["document"] = document;
    out["previousDocument"] = previous;
    out["storyKey"] = json!(key);
    out["linked"] = json!(section > 0 && !story.direct);
    out["ownerSection"] = json!(story.owner);
    out["affectedSections"] = json!(affected);
    out["pageNumbering"] = crate::docx_page_numbers::read(pages::sections(body)[section]);
    out["differentFirstPage"] = json!(first);
    out["differentOddEven"] = json!(even_odd);
    out["protected"] = json!(crate::docx_revisions::protection(pkg)?);
    Ok(out)
}
/// Immutable, source-resolved snapshot for physical page layout. Shared stories
/// are projected once per part rather than once per section or displayed page.
pub fn page_preview(pkg: &Package) -> Result<Value, String> {
    let (document, _) = crate::docx::import(pkg)?;
    let mut out = pages::read(pkg)?;
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let body = pages::document_body(&d)?;
    let sections = pages::sections(body);
    let settings = crate::docx_tabs::part(pkg, "settings")?;
    out["differentOddEven"] = json!(pkg
        .text(&settings)
        .ok()
        .and_then(|x| parse(x).ok())
        .is_some_and(|d| on(child(d.root_element(), "evenAndOddHeaders"))));
    out["noColumnBalance"] = json!(pkg
        .text(&settings)
        .ok()
        .and_then(|x| parse(x).ok())
        .is_some_and(|d| on(
            child(d.root_element(), "compat").and_then(|n| child(n, "noColumnBalance"))
        )));
    out["hasPendingRevisions"] = json!(crate::docx_revisions::has_changes(d.root_element()));
    out["document"] = document;
    let mut cached = std::collections::HashMap::<(String, String), Value>::new();
    for kind in ["header", "footer"] {
        for variant in ["default", "first", "even"] {
            for (index, story) in stories(pkg, kind, variant)?.iter().enumerate() {
                let key = (kind.to_string(), story.part.clone().unwrap_or_default());
                let value = if let Some(value) = cached.get(&key) {
                    value.clone()
                } else {
                    let value = projection(pkg, story.part.as_deref(), kind)?.1;
                    cached.insert(key, value.clone());
                    value
                };
                out["sections"][index][kind][variant] = value;
            }
        }
    }
    for (index, section) in sections.into_iter().enumerate() {
        out["sections"][index]["pageNumbering"] = crate::docx_page_numbers::read(section);
        out["sections"][index]["differentFirstPage"] =
            json!(on(section.and_then(|s| child(s, "titlePg"))));
    }
    Ok(out)
}

fn replace_section(
    pkg: &mut Package,
    index: usize,
    change: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let x = pkg.text("word/document.xml")?.to_string();
    let d = parse(&x)?;
    let body = pages::document_body(&d)?;
    let n = pages::sections(body)
        .get(index)
        .copied()
        .ok_or("Section no longer exists")?;
    if n.is_some_and(crate::docx_revisions::has_changes) {
        return Err(
            "Review pending section formatting before editing its headers and footers".into(),
        );
    }
    let source = n
        .map(|n| scoped_record(&x, n))
        .unwrap_or_else(|| format!("<w:sectPr xmlns:w=\"{W}\"/>"));
    let changed = change(&source)?;
    let output = if let Some(n) = n {
        replace_ranges(&x, vec![(n.range(), changed)])
    } else {
        let inner = body.children().map(|n| raw(&x, n)).collect::<String>() + &changed;
        replace_inner(&x, body, &inner)
    };
    pkg.set("word/document.xml", output);
    Ok(())
}
fn reference(
    pkg: &mut Package,
    section: usize,
    kind: &str,
    variant: &str,
    id: Option<&str>,
) -> Result<(), String> {
    replace_section(pkg, section, |x| {
        let d = parse(x)?;
        let root = d.root_element();
        let tag = format!("{kind}Reference");
        let mut refs = BTreeMap::new();
        for n in root.children().filter(|n| {
            n.has_tag_name((W, "headerReference")) || n.has_tag_name((W, "footerReference"))
        }) {
            let key = (
                n.tag_name().name().to_string(),
                n.attribute((W, "type")).unwrap_or("default").to_string(),
            );
            if key != (tag.clone(), variant.into()) {
                refs.insert(key, scoped_record(x, n));
            }
        }
        if let Some(id) = id {
            refs.insert((tag,variant.into()),format!("<w:{kind}Reference xmlns:w=\"{W}\" xmlns:r=\"{R}\" w:type=\"{variant}\" r:id=\"{}\"/>",esc(id)));
        }
        let references = ["headerReference", "footerReference"]
            .iter()
            .flat_map(|tag| {
                refs.iter()
                    .filter(move |((name, _), _)| name == tag)
                    .map(|(_, v)| v.clone())
            })
            .collect::<String>();
        let inner = references
            + &root
                .children()
                .filter(|n| {
                    !n.has_tag_name((W, "headerReference"))
                        && !n.has_tag_name((W, "footerReference"))
                })
                .map(|n| raw(x, n))
                .collect::<String>();
        Ok(replace_inner(x, root, &inner))
    })
}
fn add_part(
    pkg: &mut Package,
    section: usize,
    kind: &str,
    variant: &str,
    source: Option<&str>,
) -> Result<String, String> {
    let mut serial = 1;
    while pkg
        .entries
        .contains_key(&format!("word/folio-{kind}-{serial}.xml"))
    {
        serial += 1;
    }
    let path = format!("word/folio-{kind}-{serial}.xml");
    let value = source
        .map(|p| pkg.text(p).map(String::from))
        .transpose()?
        .unwrap_or_else(|| {
            format!(
                "<w:{} xmlns:w=\"{W}\"><w:p/></w:{}>",
                if kind == "header" { "hdr" } else { "ftr" },
                if kind == "header" { "hdr" } else { "ftr" }
            )
        });
    if let Some(source) = source {
        pkg.set(&rel_path(&path), normalized_rels(pkg, source)?);
    }
    pkg.set(&path, value);
    let rels = pkg
        .text(MAIN_RELS)
        .map(String::from)
        .unwrap_or_else(|_| empty_rels());
    let d = parse(&rels)?;
    let root = d.root_element();
    let mut i = 1;
    while root
        .children()
        .any(|n| attr(n, "Id") == Some(&format!("folioStory{i}")))
    {
        i += 1;
    }
    let id = format!("folioStory{i}");
    let inner = root.children().map(|n| raw(&rels, n)).collect::<String>();
    pkg.set(MAIN_RELS,replace_inner(&rels,root,&format!("{inner}<Relationship xmlns=\"{REL_NS}\" Id=\"{id}\" Type=\"{R}/{kind}\" Target=\"/{path}\"/>")));
    let x = pkg.text("[Content_Types].xml")?.to_string();
    let d = parse(&x)?;
    let root = d.root_element();
    let inner = root.children().map(|n| raw(&x, n)).collect::<String>();
    pkg.set("[Content_Types].xml",replace_inner(&x,root,&format!("{inner}<Override xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\" PartName=\"/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}+xml\"/>")));
    reference(pkg, section, kind, variant, Some(&id))?;
    Ok(path)
}
fn set_even_odd(pkg: &mut Package, value: bool) -> Result<(), String> {
    let path = crate::docx_tabs::ensure_settings(pkg)?;
    let x = pkg.text(&path)?.to_string();
    let d = parse(&x)?;
    let root = d.root_element();
    let tag = "evenAndOddHeaders";
    let fragment = if value {
        format!("<w:{tag} xmlns:w=\"{W}\"/>")
    } else {
        String::new()
    };
    if let Some(n) = child(root, tag) {
        pkg.set(&path, replace_ranges(&x, vec![(n.range(), fragment)]));
        return Ok(());
    }
    if !value {
        return Ok(());
    }
    // CT_Settings places this after defaultTabStop, autoHyphenation,
    // consecutiveHyphenLimit, hyphenationZone and doNotHyphenateCaps.
    let preceding = [
        "writeProtection",
        "view",
        "zoom",
        "removePersonalInformation",
        "removeDateAndTime",
        "doNotDisplayPageBoundaries",
        "displayBackgroundShape",
        "printPostScriptOverText",
        "printFractionalCharacterWidth",
        "printFormsData",
        "embedTrueTypeFonts",
        "embedSystemFonts",
        "saveSubsetFonts",
        "saveFormsData",
        "mirrorMargins",
        "alignBordersAndEdges",
        "bordersDoNotSurroundHeader",
        "bordersDoNotSurroundFooter",
        "gutterAtTop",
        "hideSpellingErrors",
        "hideGrammaticalErrors",
        "activeWritingStyle",
        "proofState",
        "formsDesign",
        "attachedTemplate",
        "linkStyles",
        "stylePaneFormatFilter",
        "stylePaneSortMethod",
        "documentType",
        "mailMerge",
        "revisionView",
        "trackRevisions",
        "doNotTrackMoves",
        "doNotTrackFormatting",
        "documentProtection",
        "autoFormatOverride",
        "styleLockTheme",
        "styleLockQFSet",
        "defaultTabStop",
        "autoHyphenation",
        "consecutiveHyphenLimit",
        "hyphenationZone",
        "doNotHyphenateCaps",
    ];
    let mut inserted = false;
    let mut inner = String::new();
    for n in root.children() {
        if !inserted
            && n.is_element()
            && n.tag_name().namespace() == Some(W)
            && !preceding.contains(&n.tag_name().name())
        {
            inner.push_str(&fragment);
            inserted = true;
        }
        inner.push_str(raw(&x, n));
    }
    if !inserted {
        inner.push_str(&fragment);
    }
    pkg.set(&path, replace_inner(&x, root, &inner));
    Ok(())
}

/// A copied story must not shadow document-wide bookmark names. Retarget
/// references inside the copy; references in other stories retain their source.
fn unique_copy_bookmarks(pkg: &mut Package, path: &str) -> Result<(), String> {
    let mut used = HashSet::new();
    for (name, bytes) in &pkg.entries {
        if name == path || !name.ends_with(".xml") {
            continue;
        }
        let Ok(x) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(d) = parse(x) else { continue };
        for n in d
            .descendants()
            .filter(|n| n.has_tag_name((W, "bookmarkStart")))
        {
            if let Some(name) = n.attribute((W, "name")) {
                used.insert(name.to_lowercase());
            }
        }
    }
    let x = pkg.text(path)?.to_string();
    let d = parse(&x)?;
    let mut names = BTreeMap::new();
    let mut edits = vec![];
    let attribute = |a: roxmltree::Attribute, value: &str| {
        let original = &x[a.range()];
        let name = original.split('=').next().unwrap().trim_end();
        (a.range(), format!("{name}=\"{}\"", esc(value)))
    };
    for n in d
        .descendants()
        .filter(|n| n.has_tag_name((W, "bookmarkStart")))
    {
        let Some(a) = n.attribute_node((W, "name")) else {
            continue;
        };
        let old = a.value();
        if !used.contains(&old.to_lowercase()) {
            used.insert(old.to_lowercase());
            continue;
        }
        let mut serial = 1;
        let new = loop {
            let suffix = format!("_Copy{serial}");
            let head = old.chars().take(40 - suffix.len()).collect::<String>();
            let name = head + &suffix;
            if used.insert(name.to_lowercase()) {
                break name;
            }
            serial += 1;
        };
        names.insert(old.to_lowercase(), new.clone());
        edits.push(attribute(a, &new));
    }
    fn instruction(code: &str, names: &BTreeMap<String, String>) -> String {
        let chars = code.char_indices().collect::<Vec<_>>();
        let mut i = 0;
        let mut tokens = vec![];
        while i < chars.len() {
            if chars[i].1.is_whitespace() {
                i += 1;
                continue;
            }
            let quoted = chars[i].1 == '"';
            if quoted {
                i += 1;
            }
            let start = chars.get(i).map(|p| p.0).unwrap_or(code.len());
            while i < chars.len()
                && if quoted {
                    chars[i].1 != '"'
                } else {
                    !chars[i].1.is_whitespace()
                }
            {
                i += 1;
            }
            let end = chars.get(i).map(|p| p.0).unwrap_or(code.len());
            tokens.push((start, end, &code[start..end]));
            if quoted && i < chars.len() {
                i += 1;
            }
        }
        let Some(first) = tokens.first() else {
            return code.into();
        };
        let index = match first.2.to_ascii_uppercase().as_str() {
            "REF" | "PAGEREF" | "NOTEREF" => Some(1),
            "HYPERLINK" => tokens
                .iter()
                .position(|t| t.2.eq_ignore_ascii_case("\\l"))
                .map(|i| i + 1),
            "TOC" => tokens
                .iter()
                .position(|t| t.2.eq_ignore_ascii_case("\\b"))
                .map(|i| i + 1),
            _ => None,
        };
        if let Some((start, end, old)) = index.and_then(|i| tokens.get(i)) {
            if let Some(new) = names.get(&old.to_lowercase()) {
                return format!("{}{}{}", &code[..*start], new, &code[*end..]);
            }
        }
        code.into()
    }
    let mut fields: Vec<Option<Vec<roxmltree::Node>>> = vec![];
    let mut groups = vec![];
    for n in d.descendants().filter(|n| n.is_element()) {
        if n.has_tag_name((W, "hyperlink")) {
            if let Some(a) = n.attribute_node((W, "anchor")) {
                if let Some(new) = names.get(&a.value().to_lowercase()) {
                    edits.push(attribute(a, new));
                }
            }
        }
        if n.has_tag_name((W, "fldSimple")) {
            if let Some(a) = n.attribute_node((W, "instr")) {
                let value = instruction(a.value(), &names);
                if value != a.value() {
                    edits.push(attribute(a, &value));
                }
            }
        }
        if n.has_tag_name((W, "fldChar")) {
            match n.attribute((W, "fldCharType")) {
                Some("begin") => fields.push(Some(vec![])),
                Some("separate") => {
                    if let Some(value) = fields.last_mut().and_then(Option::take) {
                        groups.push(value);
                    }
                }
                Some("end") => {
                    if let Some(Some(value)) = fields.pop() {
                        groups.push(value);
                    }
                }
                _ => {}
            }
        }
        if n.has_tag_name((W, "instrText")) {
            if let Some(Some(nodes)) = fields.last_mut() {
                nodes.push(n);
            } else {
                groups.push(vec![n]);
            }
        }
    }
    groups.extend(fields.into_iter().flatten());
    for nodes in groups {
        let old = nodes
            .iter()
            .map(|n| n.text().unwrap_or(""))
            .collect::<String>();
        let new = instruction(&old, &names);
        if old == new {
            continue;
        }
        for (i, n) in nodes.into_iter().enumerate() {
            let fragment = scoped_record(&x, n);
            let parsed = parse(&fragment)?;
            edits.push((
                n.range(),
                replace_inner(
                    &fragment,
                    parsed.root_element(),
                    &if i == 0 { esc(&new) } else { String::new() },
                ),
            ));
        }
    }
    pkg.set(path, replace_ranges(&x, edits));
    Ok(())
}
pub fn apply(pkg: &Package, r: &Value, tracking: Option<&Value>) -> Result<Package, String> {
    if crate::docx_revisions::protection(pkg)? {
        return Err("Unlock document protection before editing headers and footers".into());
    }
    if tracking.is_some_and(|s| s["enabled"] == true) {
        return Err("Recording header/footer edits is not supported yet. Turn recording off explicitly before making an unrecorded change".into());
    }
    let (section, kind, variant) = target(r)?;
    let data = read(pkg, r)?;
    if data["storyKey"] != r["storyKey"] {
        return Err("The header/footer changed. Reopen it and try again".into());
    }
    let linked = r["linked"].as_bool().ok_or("Choose Link to Previous")?;
    if section == 0 && linked {
        return Err("The first section has no previous header or footer".into());
    }
    let mut result = pkg.clone();
    let all = stories(pkg, kind, variant)?;
    let current = &all[section];
    let mut path = current.part.clone();
    let cloned = !linked && !current.direct && section > 0 && current.part.is_some();
    if linked && current.direct {
        reference(&mut result, section, kind, variant, None)?;
        path = all[section - 1].part.clone();
    } else if !linked && !current.direct && section > 0 {
        path = Some(add_part(
            &mut result,
            section,
            kind,
            variant,
            current.part.as_deref(),
        )?);
    }
    let (temp, baseline, synthetic) = projection(&result, path.as_deref(), kind)?;
    let mut document = baseline.clone();
    document["content"] = r["document"]["content"].clone();
    if !document["content"].is_array() {
        return Err("Invalid header/footer content".into());
    }
    crate::workspace::document_validate(&document, 0, &mut 0)?;
    fn has_workbook_block(node: &Value) -> bool {
        node["type"] == "workbookBlock"
            || node["content"]
                .as_array()
                .is_some_and(|children| children.iter().any(has_workbook_block))
    }
    if has_workbook_block(&document) {
        return Err("Live workbook blocks are not supported in headers and footers yet".into());
    }

    crate::document_bookmarks::rebase(&baseline, &mut document, r.get("textEdits"))?;
    crate::document_comments::rebase(&baseline, &mut document, r.get("textEdits"))?;
    if document != baseline {
        if path.is_none() {
            path = Some(add_part(&mut result, 0, kind, variant, None)?);
        }
        let path = path.as_deref().unwrap();
        let exported = docx::export_review(&document, Some(&temp), Some(&baseline))?;
        let x = exported.text("word/document.xml")?;
        let d = parse(x)?;
        let body = pages::document_body(&d)?;
        let source = result.text(path)?.to_string();
        let dom = parse(&source)?;
        let inner = body
            .children()
            .filter(|n| !n.has_tag_name((W, "sectPr")))
            .map(|n| {
                if n.is_element() {
                    scoped_record(x, n)
                } else {
                    raw(x, n).into()
                }
            })
            .collect::<String>();
        result.set(path, replace_inner(&source, dom.root_element(), &inner));
        // Export may add shared numbering definitions. Preserve those package
        // changes, while keeping the synthetic main story/relationships private.
        for (name, value) in &exported.entries {
            if name != "word/document.xml"
                && name != MAIN_RELS
                && name != "[Content_Types].xml"
                && temp.entries.get(name) != Some(value)
            {
                result.entries.insert(name.clone(), value.clone());
            }
        }
        let rels = exported.text(MAIN_RELS)?;
        let rd = parse(rels)?;
        let original_rels = parse(temp.text(MAIN_RELS)?)?;
        let original_ids = original_rels
            .root_element()
            .children()
            .filter_map(|n| attr(n, "Id"))
            .collect::<HashSet<_>>();
        let mut shared_ids = synthetic.clone();
        for n in rd.root_element().children().filter(|n| n.is_element()) {
            let id = attr(n, "Id").unwrap_or("");
            let kind = attr(n, "Type").unwrap_or("");
            if !original_ids.contains(id) && kind != format!("{R}/hyperlink") {
                if attr(n, "TargetMode") == Some("External") {
                    return Err("Unexpected external shared story relationship".into());
                }
                let target = resolve(
                    "word/document.xml",
                    attr(n, "Target").ok_or("Shared story target is missing")?,
                )?;
                let main = result.text(MAIN_RELS)?.to_string();
                let md = parse(&main)?;
                let root = md.root_element();
                if !root.children().any(|n| attr(n, "Type") == Some(kind)) {
                    let mut serial = 1;
                    while root
                        .children()
                        .any(|n| attr(n, "Id") == Some(&format!("folioSharedStory{serial}")))
                    {
                        serial += 1;
                    }
                    let inner = root.children().map(|n| raw(&main, n)).collect::<String>();
                    result.set(MAIN_RELS,replace_inner(&main,root,&format!("{inner}<Relationship xmlns=\"{REL_NS}\" Id=\"folioSharedStory{serial}\" Type=\"{}\" Target=\"/{}\"/>",esc(kind),esc(&target))));
                }
                shared_ids.insert(id.into());
            }
        }
        // Merge newly added content declarations without replacing a header
        // part created after the temporary projection was prepared.
        let source_types = exported.text("[Content_Types].xml")?;
        let td = parse(source_types)?;
        let current_types = result.text("[Content_Types].xml")?.to_string();
        let cd = parse(&current_types)?;
        let mut additions = String::new();
        for n in td.root_element().children().filter(|n| n.is_element()) {
            let key = if n.tag_name().name() == "Override" {
                "PartName"
            } else {
                "Extension"
            };
            if !cd
                .root_element()
                .children()
                .any(|c| c.tag_name().name() == n.tag_name().name() && attr(c, key) == attr(n, key))
            {
                additions.push_str(&scoped_record(source_types, n));
            }
        }
        if !additions.is_empty() {
            let root = cd.root_element();
            let inner = root
                .children()
                .map(|n| raw(&current_types, n))
                .collect::<String>()
                + &additions;
            result.set(
                "[Content_Types].xml",
                replace_inner(&current_types, root, &inner),
            );
        }
        let story_rels = rd
            .root_element()
            .children()
            .filter(|n| n.is_element() && !shared_ids.contains(attr(*n, "Id").unwrap_or("")))
            .map(|n| scoped_record(rels, n))
            .collect::<String>();
        result.set(
            &rel_path(path),
            format!("<Relationships xmlns=\"{REL_NS}\">{story_rels}</Relationships>"),
        );
    }
    if cloned {
        unique_copy_bookmarks(&mut result, path.as_deref().unwrap())?;
    }
    let first = r["differentFirstPage"]
        .as_bool()
        .ok_or("Choose first-page behavior")?;
    if data["differentFirstPage"] != first {
        replace_section(&mut result, section, |x| {
            pages::set_child(
                x,
                "titlePg",
                &if first {
                    format!("<w:titlePg xmlns:w=\"{W}\"/>")
                } else {
                    String::new()
                },
            )
        })?;
    }
    let even = r["differentOddEven"]
        .as_bool()
        .ok_or("Choose odd/even-page behavior")?;
    if data["differentOddEven"] != even {
        set_even_odd(&mut result, even)?;
    }
    if let Some(numbers) = r.get("pageNumbering") {
        if *numbers != data["pageNumbering"] {
            replace_section(&mut result, section, |x| {
                crate::docx_page_numbers::patch(x, numbers)
            })?;
        }
    }
    result.original = result.bytes()?;
    Ok(result)
}
