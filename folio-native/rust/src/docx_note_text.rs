//! The text Word Count can include (W12.09): footnotes, endnotes and text
//! boxes, which Folio keeps in the source package.
use crate::office::{parse, Package};
use serde_json::{json, Value};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// Each paragraph's text, from its runs.
fn paragraphs(node: roxmltree::Node) -> String {
    node.descendants()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            p.descendants()
                .filter_map(|n| match n.tag_name() {
                    t if t.namespace() == Some(W) && t.name() == "t" => {
                        Some(n.text().unwrap_or("").to_string())
                    }
                    t if t.namespace() == Some(W) && t.name() == "tab" => Some("\t".into()),
                    _ => None,
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn notes(pkg: &Package, part: &str, tag: &str) -> Result<Vec<String>, String> {
    let Ok(xml) = pkg.text(part) else {
        return Ok(vec![]);
    };
    let doc = parse(xml)?;
    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name((W, tag)))
        // Separators are Word's own, not the author's notes.
        .filter(|n| n.attribute((W, "type")).is_none_or(|t| t == "normal"))
        .map(paragraphs)
        .collect())
}

fn text_boxes(pkg: &Package) -> Result<Vec<String>, String> {
    let Ok(xml) = pkg.text("word/document.xml") else {
        return Ok(vec![]);
    };
    let doc = parse(xml)?;
    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name((W, "txbxContent")))
        // A DrawingML box repeats its text in the VML fallback.
        .filter(|n| !n.ancestors().any(|a| a.has_tag_name((MC, "Fallback"))))
        .map(paragraphs)
        .collect())
}

pub fn read(pkg: Option<&Package>) -> Result<Value, String> {
    let Some(pkg) = pkg else {
        return Ok(json!({"footnotes":[],"endnotes":[],"textBoxes":[]}));
    };
    Ok(json!({
        "footnotes": notes(pkg, "word/footnotes.xml", "footnote")?,
        "endnotes": notes(pkg, "word/endnotes.xml", "endnote")?,
        "textBoxes": text_boxes(pkg)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_and_boxes_skip_separators_and_fallbacks() {
        let mut pkg = Package::default();
        pkg.set(
            "word/footnotes.xml",
            format!(
                r#"<w:footnotes xmlns:w="{W}"><w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote><w:footnote w:id="1"><w:p><w:r><w:t>First note</w:t></w:r></w:p><w:p><w:r><w:t>more</w:t></w:r></w:p></w:footnote></w:footnotes>"#
            ),
        );
        pkg.set(
            "word/document.xml",
            format!(
                r#"<w:document xmlns:w="{W}" xmlns:mc="{MC}"><w:body><w:p><w:r><mc:AlternateContent><mc:Choice><w:txbxContent><w:p><w:r><w:t>Boxed</w:t><w:tab/><w:t>text</w:t></w:r></w:p></w:txbxContent></mc:Choice><mc:Fallback><w:txbxContent><w:p><w:r><w:t>Boxed</w:t></w:r></w:p></w:txbxContent></mc:Fallback></mc:AlternateContent></w:r></w:p></w:body></w:document>"#
            ),
        );
        assert_eq!(
            read(Some(&pkg)).unwrap(),
            json!({"footnotes":["First note\nmore"],"endnotes":[],"textBoxes":["Boxed\ttext"]})
        );
        assert_eq!(read(None).unwrap()["footnotes"], json!([]));
    }
}
