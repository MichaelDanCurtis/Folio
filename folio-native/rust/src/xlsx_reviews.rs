//! Spreadsheet notes, threaded comments, authors and legacy placeholders.
use crate::{
    formula::{address, parse_address},
    office::*,
    sheet_reviews as reviews,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const T: &str = "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const TR: &str = "http://schemas.microsoft.com/office/2017/10/relationships/threadedComment";
const PR: &str = "http://schemas.microsoft.com/office/2017/10/relationships/person";
const V: &str = "urn:schemas-microsoft-com:vml";
const X: &str = "urn:schemas-microsoft-com:office:excel";
fn part(pkg: &Package, owner: &str, kind: &str) -> Result<Option<String>, String> {
    let matches = crate::xlsx_structure::relationships(pkg, owner)?
        .into_iter()
        .filter(|(_, k, _)| k == kind)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "Multiple {kind} parts need additional review support"
        ));
    }
    Ok(matches.first().map(|p| p.2.clone()))
}
fn guid(s: &str) -> bool {
    s.len() == 38
        && s.starts_with('{')
        && s.ends_with('}')
        && s[1..37].bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
pub fn identifier(s: &str) -> String {
    if guid(s) {
        return s.to_uppercase();
    }
    let h = crate::office::fingerprint(s.as_bytes());
    format!(
        "{{{}-{}-4{}-8{}-{}}}",
        &h[..8],
        &h[8..12],
        &h[13..16],
        &h[17..20],
        &h[20..32]
    )
    .to_uppercase()
}
fn xtext(s: &str) -> String {
    crate::xlsx_strings::xml(s, false)
}
fn text(n: roxmltree::Node) -> String {
    crate::xlsx_strings::decode(
        &n.descendants()
            .filter(|n| n.has_tag_name((S, "t")))
            .filter_map(|n| n.text())
            .collect::<String>(),
    )
}
fn root_text(n: roxmltree::Node) -> String {
    child(n, "text")
        .map(|n| crate::xlsx_strings::decode(n.text().unwrap_or("")))
        .unwrap_or_default()
}
fn note_cell(shape: roxmltree::Node) -> Option<String> {
    let data = shape
        .descendants()
        .find(|n| n.has_tag_name((X, "ClientData")) && attr(*n, "ObjectType") == Some("Note"))?;
    let row = child(data, "Row")?.text()?.parse::<usize>().ok()?;
    let col = child(data, "Column")?.text()?.parse::<usize>().ok()?;
    if row >= 1_048_576 || col >= 16_384 {
        return None;
    }
    Some(address(row, col))
}
pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<bool, String> {
    let legacy = part(pkg, path, "comments")?;
    let threaded = part(pkg, path, "threadedComment")?;
    if legacy.is_none() && threaded.is_none() {
        return Ok(false);
    }
    let mut result = serde_json::Map::new();
    let mut warning = false;
    let mut placeholders = HashMap::new();
    if let Some(p) = legacy {
        let xml = pkg.text(&p)?;
        let dom = parse(xml)?;
        let authors = dom
            .descendants()
            .filter(|n| n.has_tag_name((S, "author")))
            .map(|n| crate::xlsx_strings::decode(n.text().unwrap_or("")))
            .collect::<Vec<_>>();
        for n in dom.descendants().filter(|n| n.has_tag_name((S, "comment"))) {
            let Some(a) = attr(n, "ref").filter(|a| parse_address(a).is_ok()) else {
                warning = true;
                continue;
            };
            let Some(author) = attr(n, "authorId")
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|i| authors.get(i))
            else {
                warning = true;
                continue;
            };
            if let Some(i) = author.find("tc={") {
                if let Some(end) = author[i + 3..].find('}') {
                    let id = &author[i + 3..i + 3 + end + 1];
                    if guid(id) {
                        placeholders.insert(id.to_uppercase(), a.to_string());
                        continue;
                    }
                }
            }
            if result.contains_key(a) {
                warning = true;
                continue;
            }
            let id = attr(n, "guid")
                .filter(|v| guid(v))
                .map(String::from)
                .unwrap_or_else(|| identifier(&format!("{}:{p}:{a}", fingerprint(&pkg.original))));
            let rich = n
                .descendants()
                .any(|n| n.has_tag_name((S, "r")) || n.has_tag_name((S, "commentPr")));
            result.insert(a.into(),json!({"kind":"note","id":format!("note-{id}"),"guid":id,"resolved":false,"sourceXml":scoped_record(xml,n),"messages":[{"id":format!("note-message-{id}"),"author":author,"date":"","text":text(n),"readOnly":rich}]}));
        }
    }
    if let Some(p) = threaded {
        let people_path = part(pkg, "xl/workbook.xml", "person")?;
        let mut people = HashMap::new();
        if let Some(p) = people_path {
            let xml = pkg.text(&p)?;
            for n in parse(xml)?
                .descendants()
                .filter(|n| n.has_tag_name((T, "person")))
            {
                if let (Some(id), Some(name)) = (attr(n, "id"), attr(n, "displayName")) {
                    people.insert(
                        id.to_uppercase(),
                        (crate::xlsx_strings::decode(name), scoped_record(xml, n)),
                    );
                }
            }
        }
        let xml = pkg.text(&p)?;
        let dom = parse(xml)?;
        let mut records = HashMap::new();
        let mut order = vec![];
        let mut duplicates = HashSet::new();
        for n in dom
            .descendants()
            .filter(|n| n.has_tag_name((T, "threadedComment")))
        {
            let Some(id) = attr(n, "id").filter(|v| guid(v)).map(str::to_uppercase) else {
                warning = true;
                continue;
            };
            if records.insert(id.clone(), n).is_some() {
                duplicates.insert(id.clone());
            }
            order.push(id);
        }
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        let mut blocked = duplicates;
        for id in &order {
            let mut root = id.clone();
            let mut seen = HashSet::new();
            let mut valid = true;
            while let Some(parent) = records.get(&root).and_then(|n| attr(*n, "parentId")) {
                if !seen.insert(root.clone()) {
                    valid = false;
                    break;
                }
                root = parent.to_uppercase();
                if !records.contains_key(&root) {
                    valid = false;
                    break;
                }
            }
            if !valid {
                warning = true;
                continue;
            }
            let n = records[id];
            if attr(n, "personId").is_none_or(|p| !people.contains_key(&p.to_uppercase())) {
                blocked.insert(root.clone());
            }
            groups.entry(root).or_default().push(id.clone());
        }
        for (root, ids) in groups {
            if blocked.contains(&root) || ids.iter().any(|id| blocked.contains(id)) {
                warning = true;
                continue;
            }
            let node = records[&root];
            let a = placeholders
                .get(&root)
                .map(String::as_str)
                .or_else(|| attr(node, "ref"));
            let Some(a) = a.filter(|a| parse_address(a).is_ok()) else {
                warning = true;
                continue;
            };
            if result.contains_key(a) {
                warning = true;
                continue;
            }
            let mut ordered = vec![root.clone()];
            ordered.extend(ids.into_iter().filter(|id| id != &root));
            let mut messages = vec![];
            for id in ordered {
                let n = records[&id];
                let person_id = attr(n, "personId").unwrap().to_uppercase();
                let (author, person) = &people[&person_id];
                messages.push(json!({"id":id,"author":author,"personId":person_id,"personXml":person,"date":attr(n,"dT").unwrap_or(""),"text":root_text(n),"sourceXml":scoped_record(xml,n),"parentId":attr(n,"parentId"),"readOnly":child(n,"mentions").is_some()}));
            }
            result.insert(a.into(),json!({"kind":"comment","id":format!("thread-{root}"),"resolved":matches!(attr(node,"done"),Some("1"|"true")),"messages":messages}));
        }
    }
    if let Some(p) = part(pkg, path, "vmlDrawing")? {
        let xml = pkg.text(&p)?;
        let dom = parse(xml)?;
        for n in dom.descendants().filter(|n| n.has_tag_name((V, "shape"))) {
            if let Some(a) = note_cell(n) {
                if let Some(review) = result.get_mut(&a) {
                    review["sourceVml"] = json!(scoped_record(xml, n));
                    if let Some(id) = attr(n, "type").and_then(|s| s.strip_prefix('#')) {
                        if let Some(kind) = dom.descendants().find(|n| {
                            n.has_tag_name((V, "shapetype")) && attr(*n, "id") == Some(id)
                        }) {
                            review["sourceVmlType"] = json!(scoped_record(xml, kind));
                        }
                    }
                    review["vmlRelationships"] = json!(n.descendants().any(|n| n
                        .attributes()
                        .any(|a| a.name() == "relid" || a.namespace() == Some(R))));
                }
            }
        }
    }
    sheet[reviews::KEY] = json!(result);
    reviews::validate(sheet)?;
    Ok(warning)
}
/// Replies may omit ref; legacy placeholders take precedence over a root's ref.
/// Materialize the common anchor before row sorting or structural deletion.
pub fn normalize_anchors(pkg: &mut Package, owner: &str) -> Result<(), String> {
    let Some(path) = part(pkg, owner, "threadedComment")? else {
        return Ok(());
    };
    let mut anchors = HashMap::new();
    if let Some(legacy) = part(pkg, owner, "comments")? {
        let dom = parse(pkg.text(&legacy)?)?;
        let authors = dom
            .descendants()
            .filter(|n| n.has_tag_name((S, "author")))
            .map(|n| n.text().unwrap_or(""))
            .collect::<Vec<_>>();
        for n in dom.descendants().filter(|n| n.has_tag_name((S, "comment"))) {
            let author = attr(n, "authorId")
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| authors.get(i));
            if let (Some(author), Some(a)) = (author, attr(n, "ref")) {
                if let Some(i) = author.find("tc={") {
                    if let Some(end) = author[i + 3..].find('}') {
                        let id = &author[i + 3..i + 3 + end + 1];
                        if guid(id) && parse_address(a).is_ok() {
                            anchors.insert(id.to_uppercase(), a.to_string());
                        }
                    }
                }
            }
        }
    }
    let xml = pkg.text(&path)?.to_string();
    let dom = parse(&xml)?;
    let records = dom
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((T, "threadedComment")))
        .filter_map(|n| attr(n, "id").map(|id| (id.to_uppercase(), n)))
        .collect::<HashMap<_, _>>();
    let mut edits = vec![];
    for (id, node) in &records {
        let mut root = id.clone();
        let mut seen = HashSet::new();
        while seen.insert(root.clone()) {
            let Some(parent) = records.get(&root).and_then(|n| attr(*n, "parentId")) else {
                break;
            };
            root = parent.to_uppercase();
        }
        let anchor = anchors.get(&root).map(String::as_str).or_else(|| {
            records
                .get(&root)
                .filter(|n| attr(**n, "parentId").is_none())
                .and_then(|n| attr(*n, "ref"))
        });
        if let Some(a) = anchor.filter(|a| parse_address(a).is_ok()) {
            if attr(*node, "ref") != Some(a) {
                edits.push((
                    node.range(),
                    crate::xlsx_tables::attributes(&xml, *node, &[("ref", Some(a.into()))]),
                ));
            }
        }
    }
    pkg.set(&path, replace_ranges(&xml, edits));
    Ok(())
}
fn new_path(pkg: &Package, folder: &str, name: &str, ext: &str) -> String {
    let mut i = 1;
    loop {
        let path = format!("xl/{folder}/{name}{i}.{ext}");
        if !pkg.entries.contains_key(&path) {
            return path;
        }
        i += 1;
    }
}
fn append(pkg: &mut Package, path: &str, root_xml: &str, fragment: &str) -> Result<(), String> {
    let xml = pkg.text(path).unwrap_or(root_xml).to_string();
    let dom = parse(&xml)?;
    let root = dom.root_element();
    let inner = root.children().map(|n| raw(&xml, n)).collect::<String>();
    pkg.set(path, replace_inner(&xml, root, &(inner + fragment)));
    Ok(())
}
fn attach(
    pkg: &mut Package,
    owner: &str,
    path: &str,
    kind: &str,
    mime: &str,
) -> Result<String, String> {
    let (dir, file) = owner.rsplit_once('/').ok_or("Invalid annotation owner")?;
    let relpath = format!("{dir}/_rels/{file}.rels");
    let xml = pkg
        .text(&relpath)
        .map(String::from)
        .unwrap_or(format!("<Relationships xmlns=\"{REL_NS}\"/>"));
    let dom = parse(&xml)?;
    if let Some(n) = dom
        .root_element()
        .children()
        .find(|n| attr(*n, "Type") == Some(kind))
    {
        return Ok(attr(n, "Id").unwrap_or("").into());
    }
    let mut i = 1;
    while dom
        .root_element()
        .children()
        .any(|n| attr(n, "Id") == Some(&format!("folioReview{i}")))
    {
        i += 1;
    }
    let id = format!("folioReview{i}");
    append(
        pkg,
        &relpath,
        &format!("<Relationships xmlns=\"{REL_NS}\"/>"),
        &format!(
            "<Relationship xmlns=\"{REL_NS}\" Id=\"{id}\" Type=\"{kind}\" Target=\"/{}\"/>",
            esc(path)
        ),
    )?;
    let ct = pkg.text("[Content_Types].xml")?;
    let dom = parse(ct)?;
    if !dom
        .root_element()
        .children()
        .any(|n| attr(n, "PartName") == Some(&format!("/{path}")))
    {
        append(pkg,"[Content_Types].xml","",&format!("<Override xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\" PartName=\"/{}\" ContentType=\"{mime}\"/>",esc(path)))?;
    }
    Ok(id)
}
fn attrs(xml: &str, changes: &[(&str, Option<String>)]) -> Result<String, String> {
    let dom = parse(xml)?;
    Ok(crate::xlsx_tables::attributes(
        xml,
        dom.root_element(),
        changes,
    ))
}
fn content(xml: &str, name: &str, ns: &str, inner: &str) -> Result<String, String> {
    let dom = parse(xml)?;
    let root = dom.root_element();
    if let Some(n) = root.children().find(|n| n.has_tag_name((ns, name))) {
        Ok(replace_ranges(
            xml,
            vec![(
                n.range(),
                format!("<{name} xmlns=\"{ns}\">{inner}</{name}>"),
            )],
        ))
    } else {
        let at = root
            .children()
            .find(|n| n.is_element())
            .map(|n| n.range().start);
        if let Some(at) = at {
            Ok(replace_ranges(
                xml,
                vec![(at..at, format!("<{name} xmlns=\"{ns}\">{inner}</{name}>"))],
            ))
        } else {
            Ok(replace_inner(
                xml,
                root,
                &format!("<{name} xmlns=\"{ns}\">{inner}</{name}>"),
            ))
        }
    }
}
fn shape(review: &Value, a: &str, id: usize) -> Result<String, String> {
    let (r, c) = parse_address(a)?;
    let original=review["sourceVml"].as_str().map(String::from).unwrap_or(format!("<v:shape xmlns:v=\"{V}\" xmlns:x=\"{X}\" id=\"_x0000_s{id}\" type=\"#_x0000_t202\" style=\"position:absolute;width:144pt;height:79pt;visibility:hidden\" fillcolor=\"#ffffe1\"><v:fill color2=\"#ffffe1\"/><v:shadow on=\"t\" color=\"black\" obscured=\"t\"/><v:path xmlns:o=\"urn:schemas-microsoft-com:office:office\" o:connecttype=\"none\"/><v:textbox style=\"mso-direction-alt:auto\"><div style=\"text-align:left\"/></v:textbox><x:ClientData ObjectType=\"Note\"><x:MoveWithCells/><x:SizeWithCells/><x:AutoFill>False</x:AutoFill><x:Row>{r}</x:Row><x:Column>{c}</x:Column></x:ClientData></v:shape>"));
    let original = attrs(&original, &[("id", Some(format!("_x0000_s{id}")))])?;
    let dom = parse(&original)?;
    let mut edits = vec![];
    for n in dom.descendants() {
        if n.has_tag_name((X, "Row")) {
            edits.push((n.range(), format!("<x:Row xmlns:x=\"{X}\">{r}</x:Row>")));
        } else if n.has_tag_name((X, "Column")) {
            edits.push((
                n.range(),
                format!("<x:Column xmlns:x=\"{X}\">{c}</x:Column>"),
            ));
        } else if n.has_tag_name((X, "Anchor")) {
            let mut v = n
                .text()
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .map(str::parse::<i64>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "Invalid note anchor")?;
            if v.len() != 8 {
                return Err("Invalid note anchor".into());
            }
            let dx = v[4] - v[0];
            let dy = v[6] - v[2];
            v[0] = c as i64;
            v[2] = r as i64;
            v[4] = (c as i64 + dx).clamp(0, 16383);
            v[6] = (r as i64 + dy).clamp(0, 1048575);
            edits.push((
                n.range(),
                format!(
                    "<x:Anchor xmlns:x=\"{X}\">{}</x:Anchor>",
                    v.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    Ok(replace_ranges(&original, edits))
}
fn replace_child_list(xml: &str, name: &str, ns: &str, items: &str) -> Result<String, String> {
    content(xml, name, ns, items)
}
fn ensure_people(pkg: &mut Package, messages: &[&Value]) -> Result<(), String> {
    if messages.is_empty() {
        return Ok(());
    }
    let path = part(pkg, "xl/workbook.xml", "person")?
        .unwrap_or_else(|| new_path(pkg, "persons", "person", "xml"));
    let xml = pkg
        .text(&path)
        .map(String::from)
        .unwrap_or(format!("<personList xmlns=\"{T}\"/>"));
    let dom = parse(&xml)?;
    let mut known = dom
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((T, "person")))
        .filter_map(|n| attr(n, "id").map(str::to_uppercase))
        .collect::<HashSet<_>>();
    let mut added = String::new();
    for m in messages {
        let id = person_id(m);
        if !known.insert(id.clone()) {
            if dom
                .root_element()
                .children()
                .find(|n| attr(*n, "id").is_some_and(|s| s.eq_ignore_ascii_case(&id)))
                .is_some_and(|n| {
                    crate::xlsx_strings::decode(attr(n, "displayName").unwrap_or(""))
                        != m["author"].as_str().unwrap_or("")
                })
            {
                return Err("Conflicting comment author identities. Nothing changed.".into());
            }
            continue;
        }
        let record = if let Some(source) = m["personXml"].as_str() {
            let d = parse(source)?;
            if !d.root_element().has_tag_name((T, "person")) {
                return Err("Invalid review author record".into());
            }
            attrs(source, &[("id", Some(id))])?
        } else {
            format!(
                "<person xmlns=\"{T}\" displayName=\"{}\" id=\"{id}\"/>",
                crate::xlsx_strings::xml(m["author"].as_str().unwrap_or(""), true)
            )
        };
        added.push_str(&record);
    }
    if !added.is_empty() {
        let root = dom.root_element();
        let xml = if let Some(ext) = child(root, "extLst") {
            replace_ranges(&xml, vec![(ext.range().start..ext.range().start, added)])
        } else {
            let inner = root.children().map(|n| raw(&xml, n)).collect::<String>();
            replace_inner(&xml, root, &(inner + &added))
        };
        pkg.set(&path, xml);
    }
    attach(
        pkg,
        "xl/workbook.xml",
        &path,
        PR,
        "application/vnd.ms-excel.person+xml",
    )?;
    Ok(())
}
fn note_id(review: &Value) -> String {
    identifier(
        review["guid"]
            .as_str()
            .unwrap_or(review["id"].as_str().unwrap()),
    )
}
fn person_id(m: &Value) -> String {
    identifier(m["personId"].as_str().unwrap_or(&format!(
        "folio-local-reviewer:{}",
        m["author"].as_str().unwrap_or("")
    )))
}
pub fn write(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    reviews::validate(sheet)?;
    if before.is_some_and(|b| b[reviews::KEY] == sheet[reviews::KEY])
        || before.is_none() && reviews::entries(sheet).next().is_none()
    {
        return Ok(());
    }
    let prior = before
        .into_iter()
        .flat_map(reviews::entries)
        .collect::<HashMap<_, _>>();
    let managed = prior.keys().map(|a| a.as_str()).collect::<HashSet<_>>();
    let current = reviews::entries(sheet).collect::<Vec<_>>();
    let legacy_path =
        part(pkg, path, "comments")?.unwrap_or_else(|| new_path(pkg, "comments", "comment", "xml"));
    let legacy = pkg.text(&legacy_path).map(String::from).unwrap_or(format!(
        "<comments xmlns=\"{S}\"><authors/><commentList/></comments>"
    ));
    let dom = parse(&legacy)?;
    let mut authors = dom
        .descendants()
        .filter(|n| n.has_tag_name((S, "author")))
        .map(|n| crate::xlsx_strings::decode(n.text().unwrap_or("")))
        .collect::<Vec<_>>();
    let mut legacy_records = String::new();
    for n in dom.descendants().filter(|n| n.has_tag_name((S, "comment"))) {
        let a = attr(n, "ref").unwrap_or("");
        if managed.contains(a) {
            continue;
        }
        if current.iter().any(|(to, _)| to.as_str() == a) {
            return Err("This cell has a preserved annotation that cannot yet be replaced. Nothing changed.".into());
        }
        legacy_records.push_str(&scoped_record(&legacy, n));
    }
    let mut thread_records = String::new();
    let mut all_messages = vec![];
    for (a, review) in &current {
        let messages = review["messages"].as_array().unwrap();
        let root_id = identifier(messages[0]["id"].as_str().unwrap());
        let note = review["kind"] == "note";
        let author = if note {
            messages[0]["author"].as_str().unwrap_or("").to_string()
        } else {
            format!("tc={root_id}")
        };
        let author_id = authors
            .iter()
            .position(|s| s == &author)
            .unwrap_or_else(|| {
                authors.push(author.clone());
                authors.len() - 1
            });
        let text_value = if note {
            messages[0]["text"].as_str().unwrap_or("").to_string()
        } else {
            messages
                .iter()
                .map(|m| {
                    format!(
                        "{}: {}",
                        m["author"].as_str().unwrap_or(""),
                        m["text"].as_str().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        };
        let record = if let Some(source) = review["sourceXml"].as_str().filter(|_| note) {
            let d = parse(source)?;
            if !d.root_element().has_tag_name((S, "comment")) {
                return Err("Invalid note source".into());
            }
            let mut record = attrs(
                source,
                &[
                    ("ref", Some(a.to_string())),
                    ("authorId", Some(author_id.to_string())),
                    ("guid", Some(note_id(review))),
                ],
            )?;
            if text(d.root_element()) != text_value {
                if messages[0]["readOnly"] == true {
                    return Err("Rich note editing needs additional support".into());
                }
                record = content(
                    &record,
                    "text",
                    S,
                    &format!(
                        "<t xmlns=\"{S}\" xml:space=\"preserve\">{}</t>",
                        xtext(&text_value)
                    ),
                )?;
            }
            record
        } else {
            format!("<comment xmlns=\"{S}\" ref=\"{a}\" authorId=\"{author_id}\" guid=\"{}\"><text><t xml:space=\"preserve\">{}</t></text></comment>",if note{note_id(review)}else{root_id.clone()},xtext(&text_value))
        };
        legacy_records.push_str(&record);
        if note {
            continue;
        }
        let message_ids = messages
            .iter()
            .map(|m| identifier(m["id"].as_str().unwrap()))
            .collect::<HashSet<_>>();
        for (i, m) in messages.iter().enumerate() {
            let id = identifier(m["id"].as_str().unwrap());
            let person = person_id(m);
            let parent = if i == 0 {
                None
            } else {
                Some(
                    m["parentId"]
                        .as_str()
                        .map(identifier)
                        .filter(|p| message_ids.contains(p))
                        .unwrap_or(root_id.clone()),
                )
            };
            let source = m["sourceXml"].as_str().unwrap_or("");
            let mut record = if source.is_empty() {
                format!("<threadedComment xmlns=\"{T}\" id=\"{id}\" personId=\"{person}\"/>")
            } else {
                let d = parse(source)?;
                if !d.root_element().has_tag_name((T, "threadedComment")) {
                    return Err("Invalid comment source".into());
                }
                source.to_string()
            };
            record = attrs(
                &record,
                &[
                    ("id", Some(id)),
                    ("personId", Some(person)),
                    ("ref", Some(a.to_string())),
                    ("parentId", parent),
                    (
                        "done",
                        Some(if review["resolved"] == true { "1" } else { "0" }.into()),
                    ),
                    (
                        "dT",
                        m["date"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(String::from),
                    ),
                ],
            )?;
            if source.is_empty()
                || root_text(parse(source)?.root_element()) != m["text"].as_str().unwrap_or("")
            {
                if m["readOnly"] == true {
                    return Err("A comment containing mentions cannot be flattened".into());
                }
                record = content(&record, "text", T, &xtext(m["text"].as_str().unwrap_or("")))?;
            }
            thread_records.push_str(&record);
            all_messages.push(m);
        }
    }
    let authors_xml = authors
        .iter()
        .map(|a| format!("<author xmlns=\"{S}\">{}</author>", xtext(a)))
        .collect::<String>();
    let legacy = replace_child_list(&legacy, "authors", S, &authors_xml)?;
    let legacy = replace_child_list(&legacy, "commentList", S, &legacy_records)?;
    pkg.set(&legacy_path, legacy);
    attach(
        pkg,
        path,
        &legacy_path,
        &format!("{R}/comments"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml",
    )?;
    let thread_path = part(pkg, path, "threadedComment")?;
    if !thread_records.is_empty() || thread_path.is_some() {
        let thread_path = thread_path
            .unwrap_or_else(|| new_path(pkg, "threadedComments", "threadedComment", "xml"));
        let xml = pkg
            .text(&thread_path)
            .map(String::from)
            .unwrap_or(format!("<ThreadedComments xmlns=\"{T}\"/>"));
        let dom = parse(&xml)?;
        let managed_ids = prior
            .values()
            .filter(|v| v["kind"] == "comment")
            .flat_map(|v| v["messages"].as_array().unwrap())
            .map(|m| identifier(m["id"].as_str().unwrap()))
            .collect::<HashSet<_>>();
        let mut retained = String::new();
        let mut extras = String::new();
        for n in dom.root_element().children() {
            if n.has_tag_name((T, "threadedComment")) {
                if !attr(n, "id").is_some_and(|id| managed_ids.contains(&id.to_uppercase())) {
                    retained.push_str(raw(&xml, n));
                }
            } else {
                extras.push_str(raw(&xml, n));
            }
        }
        pkg.set(
            &thread_path,
            replace_inner(
                &xml,
                dom.root_element(),
                &(retained + &thread_records + &extras),
            ),
        );
        attach(
            pkg,
            path,
            &thread_path,
            TR,
            "application/vnd.ms-excel.threadedcomments+xml",
        )?;
        ensure_people(pkg, &all_messages)?;
    }
    write_vml(pkg, sheet, before, path)?;
    Ok(())
}
fn write_vml(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    let vml = part(pkg, path, "vmlDrawing")?
        .unwrap_or_else(|| new_path(pkg, "drawings", "commentsDrawing", "vml"));
    let xml = pkg
        .text(&vml)
        .map(String::from)
        .unwrap_or(format!("<xml xmlns:v=\"{V}\" xmlns:x=\"{X}\"/>"));
    let dom = parse(&xml)?;
    let managed = before
        .into_iter()
        .flat_map(reviews::entries)
        .map(|(a, _)| a.clone())
        .collect::<HashSet<_>>();
    let mut inner = String::new();
    let mut ids = HashSet::new();
    let mut has_type = false;
    for n in dom.root_element().children() {
        if n.has_tag_name((V, "shape")) && note_cell(n).is_some_and(|a| managed.contains(&a)) {
            continue;
        }
        if n.has_tag_name((V, "shapetype")) && attr(n, "id") == Some("_x0000_t202") {
            has_type = true;
        }
        if let Some(id) = attr(n, "id") {
            ids.insert(id.to_string());
        }
        inner.push_str(raw(&xml, n));
    }
    if !has_type {
        inner.push_str(&format!("<v:shapetype xmlns:v=\"{V}\" xmlns:o=\"urn:schemas-microsoft-com:office:office\" id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\" path=\"m,l,21600r21600,l21600,xe\"><v:stroke joinstyle=\"miter\"/><v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/></v:shapetype>"));
    }
    let mut id = 1025;
    for (a, review) in reviews::entries(sheet) {
        while ids.contains(&format!("_x0000_s{id}")) {
            id += 1;
        }
        let mut review = review.clone();
        if let Some(source) = review["sourceVmlType"].as_str() {
            let d = parse(source)?;
            let original_id = attr(d.root_element(), "id").ok_or("Missing note shape type")?;
            // Use a stable private type identifier to avoid collisions with a destination's definitions.
            let target_id = format!("folioNoteType{}", &fingerprint(source.as_bytes())[..16]);
            if !ids.contains(&target_id) {
                inner.push_str(&attrs(source, &[("id", Some(target_id.clone()))])?);
                ids.insert(target_id.clone());
            }
            if let Some(shape) = review["sourceVml"].as_str() {
                if attr(parse(shape)?.root_element(), "type") == Some(&format!("#{original_id}")) {
                    review["sourceVml"] =
                        json!(attrs(shape, &[("type", Some(format!("#{target_id}")))])?);
                }
            }
        }
        inner.push_str(&shape(&review, a, id)?);
        ids.insert(format!("_x0000_s{id}"));
    }
    pkg.set(&vml, replace_inner(&xml, dom.root_element(), &inner));
    let rid = attach(
        pkg,
        path,
        &vml,
        &format!("{R}/vmlDrawing"),
        "application/vnd.openxmlformats-officedocument.vmlDrawing",
    )?;
    let xml = pkg.text(path)?.to_string();
    let dom = parse(&xml)?;
    let root = dom.root_element();
    if child(root, "legacyDrawing").is_none() {
        let element = format!("<legacyDrawing xmlns=\"{S}\" xmlns:r=\"{R}\" r:id=\"{rid}\"/>");
        let later = [
            "legacyDrawingHF",
            "picture",
            "oleObjects",
            "controls",
            "webPublishItems",
            "tableParts",
            "extLst",
        ];
        let out = if let Some(at) = root
            .children()
            .find(|n| n.is_element() && later.contains(&n.tag_name().name()))
            .map(|n| n.range().start)
        {
            replace_ranges(&xml, vec![(at..at, element)])
        } else {
            let inner = root.children().map(|n| raw(&xml, n)).collect::<String>();
            replace_inner(&xml, root, &(inner + &element))
        };
        pkg.set(path, out);
    }
    Ok(())
}
