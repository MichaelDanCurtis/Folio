//! Word list definitions are separate from paragraph text and formatting.
use crate::office::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
pub const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
fn wc<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|n| n.has_tag_name((W, name)))
}
fn val<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<&'a str> {
    wc(n, name).and_then(|n| n.attribute((W, "val")))
}
fn level(n: roxmltree::Node) -> Value {
    let enabled = |key| {
        wc(n, key).is_some_and(|n| !matches!(n.attribute((W, "val")), Some("0" | "false" | "off")))
    };
    json!({"format":val(n,"numFmt").unwrap_or("decimal"),
        "text":val(n,"lvlText").unwrap_or(""),
        "start":val(n,"start").and_then(|s|s.parse::<u32>().ok()).unwrap_or(0),
        "restart":val(n,"lvlRestart").and_then(|s|s.parse::<u32>().ok()),
        "suffix":val(n,"suff").unwrap_or("tab"), "legal":enabled("isLgl"),
        "style":val(n,"pStyle"), "picture":wc(n,"lvlPicBulletId").is_some()})
}
#[derive(Default)]
pub struct Catalog {
    pub path: String,
    pub instances: HashMap<String, Value>,
    styles: HashMap<String, (Option<String>, Option<String>, Option<u64>)>,
    default_style: Option<String>,
}
impl Catalog {
    pub fn read(pkg: &Package) -> Result<Self, String> {
        let mut this = Self {
            path: "word/numbering.xml".into(),
            ..Self::default()
        };
        if let Ok(xml) = pkg.text("word/_rels/document.xml.rels") {
            let d = parse(xml)?;
            if let Some(rel) = d
                .root_element()
                .children()
                .find(|n| attr(*n, "Type") == Some(&format!("{R}/numbering")))
            {
                let target = attr(rel, "Target").unwrap_or("numbering.xml");
                if target.starts_with('/') {
                    this.path = target.trim_start_matches('/').into();
                } else {
                    this.path = format!("word/{target}");
                }
            }
        }
        if let Ok(xml) = pkg.text(&this.path) {
            let d = parse(xml)?;
            let root = d.root_element();
            let abstracts: HashMap<_, _> = root
                .children()
                .filter(|n| n.has_tag_name((W, "abstractNum")))
                .filter_map(|n| n.attribute((W, "abstractNumId")).map(|id| (id, n)))
                .collect();
            for n in root.children().filter(|n| n.has_tag_name((W, "num"))) {
                let Some(id) = n.attribute((W, "numId")) else {
                    continue;
                };
                let Some(a) = val(n, "abstractNumId").and_then(|id| abstracts.get(id)) else {
                    continue;
                };
                let mut levels = vec![Value::Null; 9];
                for l in a.children().filter(|l| l.has_tag_name((W, "lvl"))) {
                    if let Some(i) = l
                        .attribute((W, "ilvl"))
                        .and_then(|s| s.parse::<usize>().ok())
                        .filter(|i| *i < 9)
                    {
                        levels[i] = level(l);
                    }
                }
                for over in n.children().filter(|l| l.has_tag_name((W, "lvlOverride"))) {
                    if let Some(i) = over
                        .attribute((W, "ilvl"))
                        .and_then(|s| s.parse::<usize>().ok())
                        .filter(|i| *i < 9)
                    {
                        if let Some(l) = wc(over, "lvl") {
                            levels[i] = level(l);
                        }
                        if let Some(start) =
                            val(over, "startOverride").and_then(|s| s.parse::<u32>().ok())
                        {
                            if levels[i].is_object() {
                                levels[i]["start"] = json!(start);
                            }
                        }
                    }
                }
                this.instances.insert(id.into(), json!(levels));
            }
        }
        if let Ok(xml) = pkg.text("word/styles.xml") {
            let d = parse(xml)?;
            for n in d
                .root_element()
                .children()
                .filter(|n| n.has_tag_name((W, "style")))
            {
                let Some(id) = n.attribute((W, "styleId")) else {
                    continue;
                };
                let num = wc(n, "pPr").and_then(|p| wc(p, "numPr"));
                this.styles.insert(
                    id.into(),
                    (
                        val(n, "basedOn").map(String::from),
                        num.and_then(|p| val(p, "numId")).map(String::from),
                        num.and_then(|p| val(p, "ilvl"))
                            .and_then(|s| s.parse().ok()),
                    ),
                );
                if n.attribute((W, "type")) == Some("paragraph")
                    && matches!(n.attribute((W, "default")), Some("1" | "true" | "on"))
                {
                    this.default_style = Some(id.into());
                }
            }
        }
        Ok(this)
    }
    pub fn paragraph(&self, p: roxmltree::Node) -> Option<Value> {
        let props = wc(p, "pPr");
        let num = props.and_then(|n| wc(n, "numPr"));
        let mut id = num.and_then(|n| val(n, "numId")).map(String::from);
        let mut ilvl = num
            .and_then(|n| val(n, "ilvl"))
            .and_then(|s| s.parse::<u64>().ok());
        let style = props
            .and_then(|n| val(n, "pStyle"))
            .map(String::from)
            .or_else(|| self.default_style.clone());
        let mut current = style.clone();
        let mut seen = HashSet::new();
        while let Some(s) = current.filter(|s| seen.insert(s.clone())) {
            let Some((base, n, l)) = self.styles.get(&s) else {
                break;
            };
            if id.is_none() {
                id = n.clone();
            }
            if ilvl.is_none() {
                ilvl = *l;
            }
            current = base.clone();
        }
        let id = id?;
        if id == "0" {
            return None;
        }
        let levels = self.instances.get(&id).cloned().unwrap_or(Value::Null);
        if ilvl.is_none() {
            ilvl = levels
                .as_array()
                .and_then(|ls| {
                    ls.iter().position(|l| {
                        style
                            .as_ref()
                            .is_some_and(|s| l["style"].as_str() == Some(s))
                    })
                })
                .map(|i| i as u64);
        }
        Some(json!({"numId":id,"level":ilvl.unwrap_or(0),"levels":levels}))
    }
}

pub fn default_levels(bullet: bool, start: u64) -> Value {
    json!((0..9).map(|i|json!({"format":if bullet {"bullet"} else {match i%3 {1=>"lowerLetter",2=>"lowerRoman",_=>"decimal"}},
        "text":if bullet {"•".into()} else {format!("%{}.",i+1)},"start":if i==0 {start}else{1},
        "restart":Value::Null,"suffix":"tab","legal":false,"style":Value::Null,"picture":false})).collect::<Vec<_>>())
}
fn level_xml(i: usize, l: &Value) -> String {
    let restart = l["restart"]
        .as_u64()
        .map(|v| format!("<w:lvlRestart w:val=\"{v}\"/>"))
        .unwrap_or_default();
    let legal = if l["legal"] == true { "<w:isLgl/>" } else { "" };
    format!("<w:lvl w:ilvl=\"{i}\"><w:start w:val=\"{}\"/><w:numFmt w:val=\"{}\"/>{restart}{legal}<w:suff w:val=\"{}\"/><w:lvlText w:val=\"{}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:tabs><w:tab w:val=\"num\" w:pos=\"{}\"/></w:tabs><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",
        l["start"].as_u64().unwrap_or(1),esc(l["format"].as_str().unwrap_or("decimal")),esc(l["suffix"].as_str().unwrap_or("tab")),esc(l["text"].as_str().unwrap_or("")),720*(i+1),720*(i+1))
}
struct Writer {
    next_num: u64,
    next_abstract: u64,
    abstracts: Vec<String>,
    nums: Vec<String>,
    groups: HashMap<String, String>,
    catalog: Catalog,
}
impl Writer {
    fn allocate(&mut self, levels: &Value) -> String {
        let id = self.next_num;
        self.next_num += 1;
        let a = self.next_abstract;
        self.next_abstract += 1;
        let ls = levels
            .as_array()
            .map(|ls| {
                ls.iter()
                    .enumerate()
                    .filter(|(_, l)| l.is_object())
                    .map(|(i, l)| level_xml(i, l))
                    .collect::<String>()
            })
            .unwrap_or_default();
        self.abstracts.push(format!("<w:abstractNum xmlns:w=\"{W}\" w:abstractNumId=\"{a}\"><w:multiLevelType w:val=\"multilevel\"/>{ls}</w:abstractNum>"));
        self.nums.push(format!(
            "<w:num xmlns:w=\"{W}\" w:numId=\"{id}\"><w:abstractNumId w:val=\"{a}\"/></w:num>"
        ));
        id.to_string()
    }
    fn paragraph(&mut self, n: &mut Value) {
        let numbering = &n["attrs"]["numbering"];
        if !numbering.is_object() {
            return;
        }
        let source = numbering["numId"].as_str();
        if source.is_some_and(|id| self.catalog.instances.get(id) == Some(&numbering["levels"])) {
            return;
        }
        // Unresolved imported definitions stay attached to their source package.
        if numbering["levels"].is_null() {
            return;
        }
        let key = format!(
            "{}:{}",
            numbering["instance"].as_str().or(source).unwrap_or("new"),
            numbering["levels"]
        );
        let id = if let Some(id) = self.groups.get(&key) {
            id.clone()
        } else {
            let id = self.allocate(&numbering["levels"]);
            self.groups.insert(key, id.clone());
            id
        };
        n["attrs"]["numbering"]["numId"] = json!(id);
    }
    fn flatten(&mut self, nodes: &[Value], inherited: Option<(String, usize)>) -> Vec<Value> {
        let mut out = vec![];
        for n in nodes {
            if matches!(n["type"].as_str(), Some("bulletList" | "orderedList")) {
                let level = n["attrs"]["level"]
                    .as_u64()
                    .map(|v| v as usize)
                    .unwrap_or_else(|| inherited.as_ref().map(|(_, l)| l + 1).unwrap_or(0))
                    .min(8);
                let mut levels = default_levels(
                    n["type"] == "bulletList",
                    n["attrs"]["start"].as_u64().unwrap_or(1),
                );
                levels[level]["start"] = json!(n["attrs"]["start"].as_u64().unwrap_or(1));
                // Each explicit list is an independent instance; a nested list
                // may use a different format without changing its parent.
                let id = self.allocate(&levels);
                for item in n["content"].as_array().into_iter().flatten() {
                    let mut first = true;
                    for block in item["content"].as_array().into_iter().flatten() {
                        if matches!(block["type"].as_str(), Some("bulletList" | "orderedList")) {
                            out.extend(
                                self.flatten(
                                    std::slice::from_ref(block),
                                    Some((id.clone(), level)),
                                ),
                            );
                        } else {
                            let mut p = block.clone();
                            if first && matches!(p["type"].as_str(), Some("paragraph" | "heading"))
                            {
                                p["attrs"]["numbering"] =
                                    json!({"numId":id,"level":level,"levels":levels});
                            }
                            first = false;
                            out.push(p);
                        }
                    }
                }
            } else {
                let mut n = n.clone();
                self.paragraph(&mut n);
                if let Some(children) = n["content"].as_array() {
                    n["content"] = json!(self.flatten(children, None));
                }
                out.push(n);
            }
        }
        out
    }
}
pub fn prepare(document: &Value, pkg: &mut Package) -> Result<Value, String> {
    let catalog = Catalog::read(pkg)?;
    let path = catalog.path.clone();
    let xml = pkg
        .text(&path)
        .map(str::to_string)
        .unwrap_or_else(|_| format!("{XML_HEAD}<w:numbering xmlns:w=\"{W}\"/>"));
    let d = parse(&xml)?;
    let root = d.root_element();
    let max = |name, key| {
        root.children()
            .filter(|n| n.has_tag_name((W, name)))
            .filter_map(|n| n.attribute((W, key)).and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0)
            + 1
    };
    let mut writer = Writer {
        next_num: max("num", "numId"),
        next_abstract: max("abstractNum", "abstractNumId"),
        abstracts: vec![],
        nums: vec![],
        groups: HashMap::new(),
        catalog,
    };
    let mut result = document.clone();
    result["content"] = json!(writer.flatten(
        document["content"].as_array().ok_or("Invalid document")?,
        None
    ));
    if writer.nums.is_empty() {
        return Ok(result);
    }
    let mut inner = String::new();
    let mut inserted = false;
    let mut nums_inserted = false;
    for n in root.children() {
        if !inserted && (n.has_tag_name((W, "num")) || n.has_tag_name((W, "numIdMacAtCleanup"))) {
            inner += &writer.abstracts.join("");
            inserted = true;
        }
        if n.has_tag_name((W, "numIdMacAtCleanup")) {
            inner += &writer.nums.join("");
            nums_inserted = true;
        }
        inner += raw(&xml, n);
    }
    if !inserted {
        inner += &writer.abstracts.join("");
    }
    if !nums_inserted {
        inner += &writer.nums.join("");
    }
    pkg.set(&path, replace_inner(&xml, root, &inner));
    let relpath = "word/_rels/document.xml.rels";
    let relxml = pkg
        .text(relpath)
        .map(str::to_string)
        .unwrap_or_else(|_| format!("<Relationships xmlns=\"{REL_NS}\"/>"));
    let rels = parse(&relxml)?;
    if !rels
        .root_element()
        .children()
        .any(|n| attr(n, "Type") == Some(&format!("{R}/numbering")))
    {
        let mut id = "folioNumbering".to_string();
        while rels
            .root_element()
            .children()
            .any(|n| attr(n, "Id") == Some(&id))
        {
            id.push('N');
        }
        let inner = rels
            .root_element()
            .children()
            .map(|n| raw(&relxml, n))
            .collect::<String>();
        let new = format!(
            "<Relationship xmlns=\"{REL_NS}\" Id=\"{id}\" Type=\"{R}/numbering\" Target=\"{}\"/>",
            esc(path.strip_prefix("word/").unwrap_or(&path))
        );
        pkg.set(
            relpath,
            replace_inner(&relxml, rels.root_element(), &(inner + &new)),
        );
    }
    let ctxml = pkg.text("[Content_Types].xml")?.to_string();
    let ct = parse(&ctxml)?;
    if !ct
        .root_element()
        .children()
        .any(|n| attr(n, "PartName") == Some(&format!("/{path}")))
    {
        let inner = ct
            .root_element()
            .children()
            .map(|n| raw(&ctxml, n))
            .collect::<String>();
        let new=format!("<Override xmlns=\"{CT}\" PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/>",esc(&path));
        pkg.set(
            "[Content_Types].xml",
            replace_inner(&ctxml, ct.root_element(), &(inner + &new)),
        );
    }
    Ok(result)
}

pub fn validate_node(node: &Value) -> Result<(), String> {
    let n = &node["attrs"]["numbering"];
    if n.is_null() {
        return Ok(());
    }
    if !n.is_object() || n["level"].as_u64().filter(|v| *v < 9).is_none() {
        return Err("Invalid document list level".into());
    }
    if let Some(id) = n.get("numId") {
        if !id.is_null()
            && !id.as_str().is_some_and(|s| {
                !s.is_empty() && s.len() < 32 && s.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err("Invalid Word list identifier".into());
        }
    }
    if let Some(id) = n.get("instance") {
        if !id.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 160) {
            return Err("Invalid list instance".into());
        }
    }
    if !n["levels"].is_null() {
        let levels = n["levels"]
            .as_array()
            .filter(|a| a.len() == 9)
            .ok_or("A Word list needs nine level slots")?;
        for l in levels.iter().filter(|l| !l.is_null()) {
            if !l.is_object()
                || !l["format"].as_str().is_some_and(|s| s.len() <= 64)
                || !l["text"].as_str().is_some_and(|s| s.len() <= 1024)
                || l["start"]
                    .as_u64()
                    .filter(|v| *v <= u32::MAX as u64)
                    .is_none()
                || (!l["restart"].is_null() && l["restart"].as_u64().filter(|v| *v <= 9).is_none())
            {
                return Err("Invalid Word numbering definition".into());
            }
        }
    }
    Ok(())
}

pub fn hydrate(document: &mut Value, source: &Value) {
    fn collect(n: &Value, result: &mut HashMap<String, Value>) {
        if let Some(id) = n["attrs"]["officeId"].as_str() {
            if let Some(numbering) = n["attrs"].get("numbering") {
                result.insert(id.into(), numbering.clone());
            }
        }
        for child in n["content"].as_array().into_iter().flatten() {
            collect(child, result);
        }
    }
    fn apply(n: &mut Value, source: &HashMap<String, Value>) {
        if n["attrs"].get("numbering").is_none() {
            if let Some(value) = n["attrs"]["officeId"]
                .as_str()
                .and_then(|id| source.get(id))
            {
                n["attrs"]["numbering"] = value.clone();
            }
        }
        if let Some(children) = n.get_mut("content").and_then(Value::as_array_mut) {
            for child in children {
                apply(child, source);
            }
        }
    }
    let mut numbers = HashMap::new();
    collect(source, &mut numbers);
    apply(document, &numbers);
}
