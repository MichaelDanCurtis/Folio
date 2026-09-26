//! Inline pictures in Word documents (W08.01–W08.02): an inline `image` node
//! for each `w:drawing/wp:inline` picture, written back from its source XML
//! when unchanged, or as a new DrawingML inline.
use crate::office::{parse, Package};
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::HashMap;

const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
const IMAGE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// EMUs per pixel at 96 dpi.
const EMU: f64 = 9525.0;

fn blip<'a, 'i>(drawing: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    drawing.descendants().find(|n| n.has_tag_name((A, "blip")))
}

/// A drawing Folio can show as an inline picture: `wp:inline` with a picture
/// whose image is embedded in the package (not linked).
pub fn is_inline_picture(drawing: Node) -> bool {
    let Some(inline) = drawing.children().find(|n| n.is_element()) else {
        return false;
    };
    inline.has_tag_name((WP, "inline"))
        && inline.descendants().any(|n| n.has_tag_name((PIC, "pic")))
        && blip(drawing).is_some_and(|b| b.attribute((R, "embed")).is_some())
}

/// The part a relationship target names, from `word/document.xml`.
fn part_of(target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_string();
    }
    let mut parts: Vec<&str> = vec!["word"];
    for piece in target.split('/') {
        match piece {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// The `image` node for an inline picture, keeping its source XML.
pub fn read(drawing: Node, rels: &HashMap<String, String>) -> Option<Value> {
    if !is_inline_picture(drawing) {
        return None;
    }
    let inline = drawing.children().find(|n| n.is_element())?;
    let extent = inline.children().find(|n| n.has_tag_name((WP, "extent")))?;
    let cx: f64 = extent.attribute("cx")?.parse().ok()?;
    let cy: f64 = extent.attribute("cy")?.parse().ok()?;
    let rid = blip(drawing)?.attribute((R, "embed"))?;
    let target = rels.get(rid)?;
    let doc_pr = inline.children().find(|n| n.has_tag_name((WP, "docPr")));
    let alt = doc_pr
        .and_then(|d| d.attribute("descr").or_else(|| d.attribute("title")))
        .unwrap_or("");
    let xml = &drawing.document().input_text()[drawing.range()];
    Some(json!({
        "type": "image",
        "attrs": {
            "src": part_of(target),
            "rId": rid,
            "width": (cx / EMU * 100.0).round() / 100.0,
            "height": (cy / EMU * 100.0).round() / 100.0,
            "alt": alt,
            "officeXml": xml,
        }
    }))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The run for an `image` node: its source drawing when it has one (an edit
/// that changes the picture drops `officeXml`), or a new DrawingML inline.
pub fn write(node: &Value, run_props: &str, id: usize) -> String {
    let attrs = &node["attrs"];
    let width = attrs["width"].as_f64().unwrap_or(96.0);
    let height = attrs["height"].as_f64().unwrap_or(96.0);
    let alt = attrs["alt"].as_str().unwrap_or("");
    let rid = attrs["rId"].as_str().unwrap_or("");
    let drawing = match attrs["officeXml"].as_str() {
        Some(xml) => xml.to_string(),
        None => {
            let (cx, cy) = ((width * EMU).round() as i64, (height * EMU).round() as i64);
            let name = format!("Picture {id}");
            format!(
                concat!(
                    "<w:drawing><wp:inline xmlns:wp=\"{wp}\" distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">",
                    "<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{id}\" name=\"{name}\" descr=\"{alt}\"/>",
                    "<a:graphic xmlns:a=\"{a}\"><a:graphicData uri=\"{pic}\">",
                    "<pic:pic xmlns:pic=\"{pic}\"><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"{name}\" descr=\"{alt}\"/><pic:cNvPicPr/></pic:nvPicPr>",
                    "<pic:blipFill><a:blip xmlns:r=\"{r}\" r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>",
                    "<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr>",
                    "</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"
                ),
                wp = WP,
                a = A,
                pic = PIC,
                r = R,
                cx = cx,
                cy = cy,
                id = id,
                name = name,
                alt = esc(alt),
                rid = esc(rid),
            )
        }
    };
    format!("<w:r><w:rPr>{run_props}</w:rPr>{drawing}</w:r>")
}

/// Image formats Folio inserts, by their leading bytes.
pub fn image_kind(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("jpeg", "image/jpeg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("gif", "image/gif"))
    } else {
        None
    }
}

/// Pixel size from a PNG, JPEG or GIF header.
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be16 = |i: usize| Some(u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as u32);
    let be32 = |i: usize| {
        Some(u32::from_be_bytes([
            *bytes.get(i)?,
            *bytes.get(i + 1)?,
            *bytes.get(i + 2)?,
            *bytes.get(i + 3)?,
        ]))
    };
    match image_kind(bytes)?.0 {
        "png" => Some((be32(16)?, be32(20)?)),
        "gif" => Some((
            u16::from_le_bytes([*bytes.get(6)?, *bytes.get(7)?]) as u32,
            u16::from_le_bytes([*bytes.get(8)?, *bytes.get(9)?]) as u32,
        )),
        _ => {
            // JPEG: the first start-of-frame marker holds the size.
            let mut i = 2;
            while i + 9 < bytes.len() {
                if bytes[i] != 0xff {
                    i += 1;
                    continue;
                }
                let marker = bytes[i + 1];
                if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
                    return Some((be16(i + 7)?, be16(i + 5)?));
                }
                if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
                    i += 2;
                    continue;
                }
                i += 2 + be16(i + 2)? as usize;
            }
            None
        }
    }
}

/// Adds a picture to a Word package: its media part, a relationship from the
/// document and the extension's content type. Returns the part and the
/// relationship id. The same picture added twice is stored once.
pub fn add(pkg: &mut Package, bytes: &[u8]) -> Result<(String, String), String> {
    use sha2::{Digest, Sha256};
    let (ext, content_type) = image_kind(bytes).ok_or("Choose a PNG, JPEG or GIF picture.")?;
    let hash = format!("{:x}", Sha256::digest(bytes));
    let part = format!("word/media/folio-{}.{ext}", &hash[..16]);
    let target = part.trim_start_matches("word/").to_string();
    pkg.entries.insert(part.clone(), bytes.to_vec());

    let rels_name = "word/_rels/document.xml.rels";
    let rels = pkg.text(rels_name).map(String::from).unwrap_or(format!(
        "<Relationships xmlns=\"{REL_NS}\"></Relationships>"
    ));
    let existing = {
        let d = parse(&rels)?;
        let found = d
            .descendants()
            .filter(|n| n.tag_name().name() == "Relationship")
            .find(|n| n.attribute("Target") == Some(target.as_str()))
            .and_then(|n| n.attribute("Id"))
            .map(String::from);
        let ids: Vec<String> = d
            .descendants()
            .filter_map(|n| n.attribute("Id").map(String::from))
            .collect();
        (found, ids)
    };
    let rid = match existing.0 {
        Some(id) => id,
        None => {
            let mut n = existing.1.len() + 1;
            while existing.1.contains(&format!("folioImage{n}")) {
                n += 1;
            }
            let id = format!("folioImage{n}");
            let end = rels
                .rfind("</Relationships>")
                .ok_or("Invalid relationships")?;
            pkg.set(
                rels_name,
                format!(
                    "{}<Relationship Id=\"{id}\" Type=\"{IMAGE_REL}\" Target=\"{target}\"/>{}",
                    &rels[..end],
                    &rels[end..]
                ),
            );
            id
        }
    };

    let types = pkg.text("[Content_Types].xml")?.to_string();
    let has_default = {
        let d = parse(&types)?;
        d.descendants().any(|n| {
            n.has_tag_name((CT, "Default"))
                && n.attribute("Extension")
                    .is_some_and(|e| e.eq_ignore_ascii_case(ext))
        })
    };
    if !has_default {
        let end = types.rfind("</Types>").ok_or("Invalid content types")?;
        pkg.set(
            "[Content_Types].xml",
            format!(
                "{}<Default Extension=\"{ext}\" ContentType=\"{content_type}\"/>{}",
                &types[..end],
                &types[end..]
            ),
        );
    }
    // The saved `.folio` file and exports read the package's bytes.
    pkg.original = pkg.bytes()?;
    Ok((part, rid))
}
