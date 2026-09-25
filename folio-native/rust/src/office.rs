//! Office packages stay in memory unchanged. Imports produce editable projections;
//! exports patch those projections back into the original package, retaining parts
//! (images, themes, charts, headers, etc.) that the native editor does not model.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub const MAX_PACKAGE: usize = 32 * 1024 * 1024;
/// Largest single part, and all parts together, once inflated.
const MAX_PART: usize = 64 * 1024 * 1024;
const MAX_EXPANDED: u64 = 128 * 1024 * 1024;
#[derive(Clone, Default)]
pub struct Package {
    pub entries: BTreeMap<String, Vec<u8>>,
    pub original: Vec<u8>,
}
impl Package {
    pub fn read(path: &str) -> Result<Self, String> {
        let meta = fs::metadata(path).map_err(|e| e.to_string())?;
        if meta.len() > MAX_PACKAGE as u64 {
            return Err("Office files are limited to 32 MB in this build.".into());
        }
        Self::from_bytes(fs::read(path).map_err(|e| e.to_string())?)
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > MAX_PACKAGE {
            return Err("Office package is too large".into());
        }
        let mut zip = ZipArchive::new(Cursor::new(&bytes))
            .map_err(|_| "This file is not an unencrypted DOCX/XLSX Office package.")?;
        if zip.len() > 10000 {
            return Err("Office package has too many parts".into());
        }
        let mut entries = BTreeMap::new();
        let mut total = 0;
        for i in 0..zip.len() {
            let file = zip.by_index(i).map_err(|e| e.to_string())?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_string();
            if name.starts_with('/')
                || name.split('/').any(|p| p == "..")
                || file.size() > MAX_PART as u64
            {
                return Err("Invalid or oversized Office package part".into());
            }
            let declared = file.size();
            if total + declared > MAX_EXPANDED {
                return Err("Expanded Office package exceeds 128 MB".into());
            }
            // A part's declared size can lie, so the limits apply to what
            // actually inflates, and a mismatch means a damaged package.
            let mut data = Vec::new();
            file.take((MAX_PART as u64).min(MAX_EXPANDED - total) + 1)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())?;
            if data.len() as u64 != declared {
                return Err("Office package part is damaged or larger than it declares".into());
            }
            total += declared;
            if entries.insert(name, data).is_some() {
                return Err("Duplicate Office package part".into());
            }
        }
        Ok(Self {
            entries,
            original: bytes,
        })
    }
    pub fn text(&self, name: &str) -> Result<&str, String> {
        std::str::from_utf8(
            self.entries
                .get(name)
                .ok_or(format!("Missing Office part: {name}"))?,
        )
        .map_err(|e| e.to_string())
    }
    pub fn set(&mut self, name: &str, xml: String) {
        self.entries.insert(name.into(), xml.into_bytes());
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        let mut out = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in &self.entries {
            out.start_file(
                name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(|e| e.to_string())?;
            out.write_all(data).map_err(|e| e.to_string())?
        }
        Ok(out.finish().map_err(|e| e.to_string())?.into_inner())
    }
    pub fn encoded(&self) -> String {
        STANDARD.encode(&self.original)
    }
    pub fn decode(text: &str) -> Result<Self, String> {
        Self::from_bytes(STANDARD.decode(text).map_err(|e| e.to_string())?)
    }
}
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn attr<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value())
}
pub fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}
pub fn xml_text(node: roxmltree::Node) -> String {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "t")
        .filter_map(|n| n.text())
        .collect()
}
/// Locate the opening tag's end without mistaking a legal `>` inside a quoted
/// attribute for the delimiter. Input here is already parsed Office XML.
pub fn opening_tag_end(xml: &str) -> Option<usize> {
    let mut quote = None;
    for (i, byte) in xml.bytes().enumerate() {
        match (quote, byte) {
            (Some(q), b) if q == b => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Some(i),
            _ => (),
        }
    }
    None
}
pub fn raw<'a>(xml: &'a str, node: roxmltree::Node) -> &'a str {
    &xml[node.range()]
}
/// Copy an element with its full namespace scope, including declarations on
/// ancestors. The record can then be edited or moved without losing prefixes.
pub fn scoped_record(xml: &str, node: roxmltree::Node) -> String {
    let source = raw(xml, node);
    let tag = source[1..]
        .split(|c: char| c.is_whitespace() || matches!(c, '/' | '>'))
        .next()
        .unwrap();
    let mut declarations = String::new();
    for ns in node.namespaces() {
        let suffix = ns.name().map(|n| format!(":{n}")).unwrap_or_default();
        declarations.push_str(&format!(" xmlns{suffix}=\"{}\"", esc(ns.uri())));
    }
    let mut attrs = String::new();
    for a in node.attributes() {
        let prefix = a
            .namespace()
            .and_then(|uri| {
                if uri == "http://www.w3.org/XML/1998/namespace" {
                    Some("xml")
                } else {
                    node.namespaces()
                        .find(|ns| ns.uri() == uri && ns.name().is_some())
                        .and_then(|ns| ns.name())
                }
            })
            .map(|p| format!("{p}:"))
            .unwrap_or_default();
        attrs.push_str(&format!(" {prefix}{}=\"{}\"", a.name(), esc(a.value())));
    }
    let inner = node.children().map(|n| raw(xml, n)).collect::<String>();
    format!("<{tag}{declarations}{attrs}>{inner}</{tag}>")
}
/// Deepest element nesting accepted. Real documents stay far below it (a
/// table in a table in a table is about ten levels). roxmltree and the
/// importers both recurse per level, so the limit is checked on the raw text
/// before parsing: a hostile file could otherwise overflow the stack, which
/// aborts the process rather than returning an error.
pub const MAX_XML_DEPTH: usize = 256;
/// Whether `xml`'s element nesting exceeds `limit`. Comments, CDATA,
/// processing instructions and quoted attribute values are skipped, so a
/// `<` or `>` inside them is not counted. Malformed text may be miscounted,
/// but the parser rejects it anyway.
fn nested_deeper_than(xml: &str, limit: usize) -> bool {
    let b = xml.as_bytes();
    let skip_to = |from: usize, end: &[u8]| {
        b[from..]
            .windows(end.len())
            .position(|w| w == end)
            .map_or(b.len(), |i| from + i + end.len())
    };
    let mut depth = 0usize;
    let mut i = 0;
    while let Some(offset) = b[i..].iter().position(|&c| c == b'<') {
        i += offset;
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = skip_to(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_to(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_to(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            i = skip_to(i + 2, b">");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = skip_to(i + 2, b">");
        } else {
            // A start tag: find its `>` outside quoted attribute values.
            let mut j = i + 1;
            let mut quote = None;
            while j < b.len() {
                match (quote, b[j]) {
                    (None, b'"' | b'\'') => quote = Some(b[j]),
                    (Some(q), c) if c == q => quote = None,
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            if j >= b.len() {
                return false;
            }
            if b[j - 1] != b'/' {
                depth += 1;
                if depth > limit {
                    return true;
                }
            }
            i = j + 1;
        }
    }
    false
}
pub fn parse(xml: &str) -> Result<roxmltree::Document<'_>, String> {
    if nested_deeper_than(xml, MAX_XML_DEPTH) {
        return Err("Office XML is nested too deeply".into());
    }
    roxmltree::Document::parse(xml).map_err(|e| format!("Invalid Office XML: {e}"))
}
pub fn replace_ranges(
    xml: &str,
    mut replacements: Vec<(std::ops::Range<usize>, String)>,
) -> String {
    replacements.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    let mut out = xml.to_string();
    for (r, s) in replacements {
        out.replace_range(r, &s)
    }
    out
}
pub fn replace_inner(xml: &str, node: roxmltree::Node, content: &str) -> String {
    let r = node.range();
    let fragment = &xml[r.clone()];
    let Some(open) = fragment.find('>') else {
        return xml.into();
    };
    if fragment[..open].ends_with('/') {
        let opening = fragment[..open - 1].to_string();
        let tag = fragment[1..].split([' ', '/', '>']).next().unwrap();
        return replace_ranges(xml, vec![(r, format!("{opening}>{content}</{tag}>"))]);
    }
    let close = fragment.rfind("</").unwrap();
    replace_ranges(
        xml,
        vec![(r.start + open + 1..r.start + close, content.into())],
    )
}
pub fn write_bytes(path: &str, data: &[u8]) -> Result<(), String> {
    let p = Path::new(path);
    let parent = p.parent().ok_or("Invalid save path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    atomic_write(p, data)
}

pub fn fingerprint(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}

pub fn file_fingerprint(path: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Check again after preparing the bytes, immediately before atomic replacement.
/// The returned fingerprint describes our bytes, not a later external rewrite.
pub fn write_checked_bytes(
    path: &str,
    data: &[u8],
    expected: Option<&str>,
) -> Result<String, String> {
    if let Some(expected) = expected {
        if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid saved-file fingerprint".into());
        }
        if file_fingerprint(path)? != expected {
            return Err(
                "This file changed outside Folio. Use Save As to keep both versions.".into(),
            );
        }
    }
    write_bytes(path, data)?;
    Ok(fingerprint(data))
}

#[cfg(target_os = "macos")]
fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    // A Save panel authorizes its chosen file, not arbitrary sibling paths.
    // Foundation manages the temporary location and atomic replacement under
    // that grant. Do not fall back to truncating the original on a failed save.
    use objc2_foundation::{NSData, NSDataWritingOptions, NSString, NSURL};
    objc2::rc::autoreleasepool(|_| {
        let path = NSString::from_str(path.to_str().ok_or("Invalid save path")?);
        let url = NSURL::fileURLWithPath(&path);
        NSData::with_bytes(data)
            .writeToURL_options_error(&url, NSDataWritingOptions::Atomic)
            .map_err(|error| error.localizedDescription().to_string())
    })
}

#[cfg(not(target_os = "macos"))]
fn atomic_write(p: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = p.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&tmp, p)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result.map_err(|e| e.to_string())
}
pub fn report(kind: &str, features: Vec<String>, preserved: usize) -> Value {
    json!({"format":kind,"mode":"preserve-original-package","preservedParts":preserved,"warnings":features,"fullFidelity":false})
}
pub const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub const XML_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";

#[cfg(test)]
mod save_tests {
    use super::*;

    #[test]
    fn atomic_saves_replace_bytes_and_preserve_existing_data_on_failure() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("folio-atomic-save-{stamp}"));
        let file = dir.join("nested/Résumé 📝.docx");
        let path = file.to_str().unwrap();
        write_bytes(path, b"original bytes").unwrap();
        write_bytes(path, &[0, 1, 255, 0, 42]).unwrap();
        assert_eq!(fs::read(&file).unwrap(), [0, 1, 255, 0, 42]);
        write_bytes(path, b"").unwrap();
        assert!(fs::read(&file).unwrap().is_empty());

        let occupied = dir.join("occupied");
        fs::create_dir(&occupied).unwrap();
        fs::write(occupied.join("sentinel"), b"keep this").unwrap();
        assert!(write_bytes(occupied.to_str().unwrap(), b"cannot replace a directory").is_err());
        assert_eq!(fs::read(occupied.join("sentinel")).unwrap(), b"keep this");
        assert!(write_bytes(
            file.join("child").to_str().unwrap(),
            b"cannot traverse a file"
        )
        .is_err());
        assert!(fs::read(&file).unwrap().is_empty());
        let names: Vec<_> = fs::read_dir(file.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(names.len(), 1);
        // macOS may decompose Unicode filenames on disk.
        assert_eq!(
            names[0].canonicalize().unwrap(),
            file.canonicalize().unwrap()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}

/// Make a parsed XML subtree independently parseable while retaining every
/// attribute and all child XML. Namespace declarations are reconstructed from
/// the parser, so whitespace around '=' cannot introduce duplicate bindings.
pub(crate) fn standalone_xml(xml: &str, n: roxmltree::Node) -> String {
    let source = raw(xml, n);
    let name = source
        .trim_start_matches('<')
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap();
    let mut attrs = n
        .namespaces()
        .map(|ns| {
            format!(
                " xmlns{}=\"{}\"",
                ns.name().map(|s| format!(":{s}")).unwrap_or_default(),
                esc(ns.uri())
            )
        })
        .collect::<String>();
    for a in n.attributes() {
        let name = a
            .namespace()
            .and_then(|uri| n.lookup_prefix(uri))
            .map(|p| format!("{p}:{}", a.name()))
            .unwrap_or(a.name().into());
        attrs.push_str(&format!(" {name}=\"{}\"", esc(a.value())));
    }
    let inner = n.children().map(|n| raw(xml, n)).collect::<String>();
    format!("<{name}{attrs}>{inner}</{name}>")
}

#[cfg(test)]
mod working_file_save_tests {
    use super::*;
    use crate::workspace::Core;

    #[test]
    fn guarded_workspace_and_office_saves_preserve_outside_changes() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("folio-working-files-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        let mut core = Core::new();
        for (format, op) in [
            ("folio", "save"),
            ("docx", "exportOffice"),
            ("xlsx", "exportOffice"),
        ] {
            let path = dir.join(format!("working.{format}"));
            let base = json!({"op":op,"format":format,"path":path});
            let first = core.dispatch(base.clone()).unwrap();
            let original = fs::read(&path).unwrap();
            assert_eq!(first["fingerprint"], fingerprint(&original));
            let mut request = base.clone();
            request["expectedFingerprint"] = first["fingerprint"].clone();
            let second = core.dispatch(request.clone()).unwrap();
            assert_eq!(
                second["fingerprint"],
                fingerprint(&fs::read(&path).unwrap())
            );
            let mut outside = fs::read(&path).unwrap();
            outside.extend_from_slice(b"outside edit");
            fs::write(&path, &outside).unwrap();
            request["expectedFingerprint"] = second["fingerprint"].clone();
            let workspace = core.workspace.clone();
            assert!(core
                .dispatch(request.clone())
                .unwrap_err()
                .contains("changed outside"));
            assert_eq!(fs::read(&path).unwrap(), outside);
            assert_eq!(core.workspace, workspace);
            request["expectedFingerprint"] = json!("malformed");
            assert!(core.dispatch(request.clone()).is_err());
            assert_eq!(fs::read(&path).unwrap(), outside);
            fs::remove_file(&path).unwrap();
            request["expectedFingerprint"] = first["fingerprint"].clone();
            assert!(core.dispatch(request).is_err());
            assert!(!path.exists());
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
