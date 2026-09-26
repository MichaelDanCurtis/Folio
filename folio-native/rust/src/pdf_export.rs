//! PDF export (D05.04, D05.05): writes the pages the app laid out, as
//! recorded drawing operations, into a PDF with embedded font subsets,
//! pictures, links, an outline and a tagged structure (headings,
//! paragraphs, figures with alt text; headers and footers as artifacts).
//!
//! Coordinates arrive in logical pixels (96 per inch) from the top left.
//! Text arrives as positioned fragments with the width the layout gave
//! them; each is written in the font the layout asked for (or a
//! substitute on this machine), scaled horizontally to that width so the
//! PDF matches the preview.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::{write::ZlibEncoder, Compression};
use pdf_writer::types::{
    ActionType, AnnotationType, CidFontType, FontFlags, StructRole, SystemInfo, TextRenderingMode,
    UnicodeCmap,
};
use pdf_writer::writers::StructTreeRoot;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Points per logical pixel.
const PT: f32 = 0.75;

// ── Fonts ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct FaceKey {
    path: PathBuf,
    index: u32,
}

#[derive(Clone, Debug)]
struct FaceInfo {
    key: FaceKey,
    family: String,
    bold: bool,
    italic: bool,
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join("Library/Fonts"));
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
    }
    // Microsoft Office installs its fonts inside the app bundles.
    for app in ["Microsoft Word", "Microsoft Excel"] {
        dirs.push(PathBuf::from(format!(
            "/Applications/{app}.app/Contents/Resources/DFonts"
        )));
    }
    if let Some(extra) = std::env::var_os("FOLIO_FONT_DIR") {
        dirs.insert(0, PathBuf::from(extra));
    }
    dirs
}

fn scan(dir: &std::path::Path, out: &mut Vec<FaceInfo>, depth: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < 6 {
                scan(&path, out, depth + 1);
            }
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "ttf" | "ttc" | "otf") {
            continue;
        }
        // Very large collections (full CJK sets) cost seconds to read; the
        // fallback list names the ones worth it.
        if entry.metadata().is_ok_and(|m| m.len() > 80 * 1024 * 1024) {
            continue;
        }
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
        for index in 0..count.min(64) {
            let Ok(face) = ttf_parser::Face::parse(&data, index) else {
                continue;
            };
            // Only TrueType outlines are embedded (FontFile2).
            if face.tables().glyf.is_none() {
                continue;
            }
            let family = face
                .names()
                .into_iter()
                .filter(|n| n.name_id == 16 || n.name_id == 1)
                .filter(|n| n.is_unicode())
                .find_map(|n| n.to_string())
                .unwrap_or_default();
            if family.is_empty() {
                continue;
            }
            out.push(FaceInfo {
                key: FaceKey {
                    path: path.clone(),
                    index,
                },
                family,
                bold: face.is_bold(),
                italic: face.is_italic() || face.is_oblique(),
            });
        }
    }
}

fn library() -> &'static Vec<FaceInfo> {
    static LIBRARY: OnceLock<Vec<FaceInfo>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let mut out = vec![];
        for dir in font_dirs() {
            scan(&dir, &mut out, 0);
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    })
}

const SANS: [&str; 9] = [
    "Aptos",
    "Calibri",
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Liberation Sans",
    "DejaVu Sans",
    "FreeSans",
    "Noto Sans",
];
const SERIF: [&str; 7] = [
    "Cambria",
    "Times New Roman",
    "Times",
    "Georgia",
    "Liberation Serif",
    "DejaVu Serif",
    "FreeSerif",
];
const MONO: [&str; 7] = [
    "Menlo",
    "Consolas",
    "Courier New",
    "Courier",
    "Liberation Mono",
    "DejaVu Sans Mono",
    "FreeMono",
];
/// Fonts that cover scripts the others don't.
const FALLBACK: [&str; 10] = [
    "Arial Unicode MS",
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Noto Sans CJK SC",
    "WenQuanYi Zen Hei",
    "DejaVu Sans",
    "FreeSerif",
    "FreeSans",
    "Liberation Sans",
];

fn generic(family: &str) -> &'static [&'static str] {
    let f = family.to_ascii_lowercase();
    if ["mono", "courier", "consolas", "menlo", "code"]
        .iter()
        .any(|k| f.contains(k))
    {
        &MONO
    } else if ["times", "cambria", "georgia", "serif", "garamond", "book"]
        .iter()
        .any(|k| f.contains(k))
        && !f.contains("sans")
    {
        &SERIF
    } else {
        &SANS
    }
}

/// A face of exactly this family, closest to the style.
fn family_face(name: &str, bold: bool, italic: bool) -> Option<(FaceKey, bool, bool)> {
    let lib = library();
    let faces: Vec<&FaceInfo> = lib
        .iter()
        .filter(|f| f.family.eq_ignore_ascii_case(name))
        .collect();
    if faces.is_empty() {
        return None;
    }
    let exact = faces.iter().find(|f| f.bold == bold && f.italic == italic);
    let face = exact
        .or_else(|| faces.iter().find(|f| f.bold == bold && !f.italic))
        .or_else(|| faces.iter().find(|f| !f.bold && f.italic == italic))
        .or_else(|| faces.iter().find(|f| !f.bold && !f.italic))
        .unwrap_or(&faces[0]);
    Some((face.key.clone(), bold && !face.bold, italic && !face.italic))
}

/// The best face for a family and style, with its synthesized styles.
fn find(family: &str, bold: bool, italic: bool) -> Option<(FaceKey, bool, bool)> {
    let lib = library();
    let pick = |name: &str| -> Option<(FaceKey, bool, bool)> {
        let faces: Vec<&FaceInfo> = lib
            .iter()
            .filter(|f| f.family.eq_ignore_ascii_case(name))
            .collect();
        if faces.is_empty() {
            return None;
        }
        let exact = faces.iter().find(|f| f.bold == bold && f.italic == italic);
        let face = exact
            .or_else(|| faces.iter().find(|f| f.bold == bold && !f.italic))
            .or_else(|| faces.iter().find(|f| !f.bold && f.italic == italic))
            .or_else(|| faces.iter().find(|f| !f.bold && !f.italic))
            .unwrap_or(&faces[0]);
        Some((face.key.clone(), bold && !face.bold, italic && !face.italic))
    };
    std::iter::once(family)
        .filter(|f| !f.is_empty())
        .chain(generic(family).iter().copied())
        .chain(FALLBACK.iter().copied())
        .find_map(pick)
        .or_else(|| lib.first().map(|f| (f.key.clone(), bold, italic)))
}

struct LoadedFace {
    data: Vec<u8>,
    index: u32,
}

impl LoadedFace {
    fn face(&self) -> Option<ttf_parser::Face<'_>> {
        ttf_parser::Face::parse(&self.data, self.index).ok()
    }
}

/// A face used in the document, with the glyphs to keep.
struct UsedFace {
    loaded: LoadedFace,
    remapper: subsetter::GlyphRemapper,
    unicode: BTreeMap<u16, String>,
    name: String,
}

#[derive(Default)]
struct Fonts {
    faces: Vec<(FaceKey, UsedFace)>,
    resolved: HashMap<(String, bool, bool), Option<(usize, bool, bool)>>,
    covered: HashMap<char, Option<usize>>,
}

impl Fonts {
    fn load(&mut self, key: &FaceKey) -> Option<usize> {
        if let Some(i) = self.faces.iter().position(|(k, _)| k == key) {
            return Some(i);
        }
        let data = std::fs::read(&key.path).ok()?;
        let loaded = LoadedFace {
            data,
            index: key.index,
        };
        let name = {
            let face = loaded.face()?;
            let named = |id: u16| {
                face.names()
                    .into_iter()
                    .filter(|n| n.name_id == id)
                    .find_map(|n| {
                        n.to_string().or_else(|| {
                            // Mac Roman names are single-byte.
                            (n.platform_id == ttf_parser::PlatformId::Macintosh)
                                .then(|| n.name.iter().map(|b| *b as char).collect())
                        })
                    })
            };
            let name: String = named(6)
                .or_else(|| named(4))
                .or_else(|| named(1))
                .unwrap_or_else(|| "Font".into())
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if name.is_empty() {
                format!("Font{}", self.faces.len())
            } else {
                name
            }
        };
        let mut remapper = subsetter::GlyphRemapper::new();
        remapper.remap(0);
        self.faces.push((
            key.clone(),
            UsedFace {
                loaded,
                remapper,
                unicode: BTreeMap::new(),
                name,
            },
        ));
        Some(self.faces.len() - 1)
    }

    fn resolve(&mut self, family: &str, bold: bool, italic: bool) -> Option<(usize, bool, bool)> {
        let k = (family.to_string(), bold, italic);
        if let Some(r) = self.resolved.get(&k) {
            return *r;
        }
        let r =
            find(family, bold, italic).and_then(|(key, sb, si)| Some((self.load(&key)?, sb, si)));
        self.resolved.insert(k, r);
        r
    }

    /// A face with a glyph for `c`: the chosen one, a fallback, or any
    /// font on this machine that has it.
    fn covering(&mut self, primary: usize, c: char) -> usize {
        let covers = |fonts: &Fonts, i: usize| {
            fonts.faces[i]
                .1
                .loaded
                .face()
                .is_some_and(|f| f.glyph_index(c).is_some())
        };
        if covers(self, primary) {
            return primary;
        }
        if let Some(found) = self.covered.get(&c) {
            return found.unwrap_or(primary);
        }
        let mut keys: Vec<FaceKey> = FALLBACK
            .iter()
            .filter_map(|name| family_face(name, false, false).map(|f| f.0))
            .collect();
        keys.extend(library().iter().map(|f| f.key.clone()));
        let mut found = None;
        for key in keys {
            let loaded_before = self.faces.len();
            let Some(i) = self.load(&key) else { continue };
            if covers(self, i) {
                found = Some(i);
                break;
            }
            // Don't keep faces that were only opened to look.
            if self.faces.len() > loaded_before {
                self.faces.pop();
            }
        }
        self.covered.insert(c, found);
        found.unwrap_or(primary)
    }
}

/// A run of text in one face: its new glyph ids and natural width in
/// text-space units (thousandths of the font size).
struct Run {
    face: usize,
    glyphs: Vec<u16>,
    width: f32,
}

fn shape(fonts: &mut Fonts, primary: usize, text: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = vec![];
    for c in text.chars() {
        if c.is_control() {
            continue;
        }
        let i = fonts.covering(primary, c);
        let used = &mut fonts.faces[i].1;
        let Some(face) = used.loaded.face() else {
            continue;
        };
        let old = face.glyph_index(c).map(|g| g.0).unwrap_or(0);
        let advance = face
            .glyph_hor_advance(ttf_parser::GlyphId(old))
            .unwrap_or(0) as f32
            * 1000.0
            / face.units_per_em() as f32;
        let new = used.remapper.remap(old);
        used.unicode.entry(new).or_default();
        if used.unicode.get(&new).is_some_and(String::is_empty) {
            used.unicode.insert(new, c.to_string());
        }
        match runs.last_mut() {
            Some(run) if run.face == i => {
                run.glyphs.push(new);
                run.width += advance;
            }
            _ => runs.push(Run {
                face: i,
                glyphs: vec![new],
                width: advance,
            }),
        }
    }
    runs
}

// ── Input ──────────────────────────────────────────────────────────────────

fn num(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

fn rect(v: &Value) -> (f32, f32, f32, f32) {
    (num(&v[0]), num(&v[1]), num(&v[2]), num(&v[3]))
}

fn color(v: &Value) -> Option<(f32, f32, f32)> {
    let s = v.as_str()?.trim_start_matches('#');
    let n = u32::from_str_radix(s.get(s.len().saturating_sub(6)..)?, 16).ok()?;
    Some((
        ((n >> 16) & 255) as f32 / 255.0,
        ((n >> 8) & 255) as f32 / 255.0,
        (n & 255) as f32 / 255.0,
    ))
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

fn role(tag: &str) -> StructRole {
    match tag {
        "H1" => StructRole::H1,
        "H2" => StructRole::H2,
        "H3" => StructRole::H3,
        "H4" => StructRole::H4,
        "H5" => StructRole::H5,
        "H6" => StructRole::H6,
        "Figure" => StructRole::Figure,
        "Table" => StructRole::Table,
        "TR" => StructRole::TR,
        "TD" => StructRole::TD,
        "L" => StructRole::L,
        "LI" => StructRole::LI,
        "Link" => StructRole::Link,
        _ => StructRole::P,
    }
}

struct Ids(i32);
impl Ids {
    fn next(&mut self) -> Ref {
        self.0 += 1;
        Ref::new(self.0)
    }
}

struct Tagged {
    role: StructRole,
    alt: Option<String>,
    page: usize,
    mcids: Vec<i32>,
}

/// Writes the PDF for `request` (see the module docs): `pages`, `images`,
/// `outline`, `title` and `lang`.
pub fn write(request: &Value) -> Result<Vec<u8>, String> {
    let pages = request["pages"].as_array().ok_or("No pages to write")?;
    if pages.is_empty() {
        return Err("No pages to write".into());
    }
    let mut ids = Ids(0);
    let catalog = ids.next();
    let tree = ids.next();
    let struct_root = ids.next();
    let document_element = ids.next();
    let info = ids.next();
    let page_refs: Vec<Ref> = pages.iter().map(|_| ids.next()).collect();
    let content_refs: Vec<Ref> = pages.iter().map(|_| ids.next()).collect();
    let mut pdf = Pdf::new();
    let mut fonts = Fonts::default();
    let mut images: HashMap<String, Ref> = HashMap::new();
    let mut tagged: Vec<Tagged> = vec![];
    let mut links: Vec<Vec<(Rect, String)>> = vec![];
    let mut page_heights = vec![];

    // Pictures: RGB with an alpha mask when any pixel is translucent.
    for (key, image) in request["images"].as_object().into_iter().flatten() {
        let (w, h) = (
            image["width"].as_u64().unwrap_or(0) as u32,
            image["height"].as_u64().unwrap_or(0) as u32,
        );
        let Ok(rgba) = STANDARD.decode(image["rgba"].as_str().unwrap_or("")) else {
            continue;
        };
        if w == 0 || h == 0 || rgba.len() != (w * h * 4) as usize {
            continue;
        }
        let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        let alpha: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();
        let id = ids.next();
        let mask = alpha.iter().any(|a| *a < 255).then(|| ids.next());
        let data = deflate(&rgb);
        let mut x = pdf.image_xobject(id, &data);
        x.filter(Filter::FlateDecode);
        x.width(w as i32);
        x.height(h as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        if let Some(m) = mask {
            x.s_mask(m);
        }
        x.finish();
        if let Some(m) = mask {
            let data = deflate(&alpha);
            let mut x = pdf.image_xobject(m, &data);
            x.filter(Filter::FlateDecode);
            x.width(w as i32);
            x.height(h as i32);
            x.color_space().device_gray();
            x.bits_per_component(8);
            x.finish();
        }
        images.insert(key.clone(), id);
    }

    let mut contents: Vec<(Vec<u8>, Vec<usize>, Vec<String>)> = vec![];
    for (p, page) in pages.iter().enumerate() {
        let height = num(&page["height"]) * PT;
        page_heights.push(height);
        let y = |v: f32| height - v * PT;
        let mut c = Content::new();
        let mut used_faces: Vec<usize> = vec![];
        let mut used_images: Vec<String> = vec![];
        let mut page_links = vec![];
        let mut mcid = 0;
        let mut open: Vec<Option<usize>> = vec![];
        for op in page["ops"].as_array().into_iter().flatten() {
            match op["op"].as_str().unwrap_or("") {
                "save" => {
                    c.save_state();
                }
                "restore" => {
                    c.restore_state();
                }
                "clip" => {
                    let (x0, y0, w, h) = rect(&op["rect"]);
                    c.rect(x0 * PT, y(y0 + h), w * PT, h * PT);
                    c.clip_nonzero();
                    c.end_path();
                }
                "rect" => {
                    let (x0, y0, w, h) = rect(&op["rect"]);
                    let fill = color(&op["fill"]);
                    let stroke = color(&op["stroke"]);
                    if fill.is_none() && stroke.is_none() {
                        continue;
                    }
                    c.save_state();
                    if let Some((r, g, b)) = fill {
                        c.set_fill_rgb(r, g, b);
                    }
                    if let Some((r, g, b)) = stroke {
                        c.set_stroke_rgb(r, g, b);
                        c.set_line_width(num(&op["width"]).max(0.1) * PT);
                    }
                    c.rect(x0 * PT, y(y0 + h), w * PT, h * PT);
                    match (fill.is_some(), stroke.is_some()) {
                        (true, true) => c.fill_nonzero_and_stroke(),
                        (true, false) => c.fill_nonzero(),
                        _ => c.stroke(),
                    };
                    c.restore_state();
                }
                "line" => {
                    let (r, g, b) = color(&op["color"]).unwrap_or((0.0, 0.0, 0.0));
                    c.save_state();
                    c.set_stroke_rgb(r, g, b);
                    c.set_line_width(num(&op["width"]).max(0.1) * PT);
                    c.move_to(num(&op["from"][0]) * PT, y(num(&op["from"][1])));
                    c.line_to(num(&op["to"][0]) * PT, y(num(&op["to"][1])));
                    c.stroke();
                    c.restore_state();
                }
                "image" => {
                    let key = op["image"].as_str().unwrap_or("");
                    if !images.contains_key(key) {
                        continue;
                    }
                    let (x0, y0, w, h) = rect(&op["rect"]);
                    let n = used_images
                        .iter()
                        .position(|k| k == key)
                        .unwrap_or_else(|| {
                            used_images.push(key.to_string());
                            used_images.len() - 1
                        });
                    c.save_state();
                    c.transform([w * PT, 0.0, 0.0, h * PT, x0 * PT, y(y0 + h)]);
                    c.x_object(Name(format!("Im{n}").as_bytes()));
                    c.restore_state();
                }
                "text" => {
                    let text = op["text"].as_str().unwrap_or("");
                    if text.trim().is_empty() {
                        continue;
                    }
                    let size = num(&op["size"]).max(1.0) * PT;
                    let bold = op["bold"] == true;
                    let italic = op["italic"] == true;
                    let Some((primary, fake_bold, fake_italic)) =
                        fonts.resolve(op["family"].as_str().unwrap_or(""), bold, italic)
                    else {
                        continue;
                    };
                    let runs = shape(&mut fonts, primary, text);
                    let natural: f32 = runs.iter().map(|r| r.width).sum::<f32>() * size / 1000.0;
                    let target = num(&op["width"]) * PT;
                    let scale = if natural > 0.0 && target > 0.0 {
                        (target / natural * 100.0).clamp(50.0, 200.0)
                    } else {
                        100.0
                    };
                    let (r, g, b) = color(&op["color"]).unwrap_or((0.0, 0.0, 0.0));
                    let x0 = num(&op["x"]) * PT;
                    let base = y(num(&op["y"]));
                    c.begin_text();
                    c.set_fill_rgb(r, g, b);
                    if fake_bold {
                        c.set_stroke_rgb(r, g, b);
                        c.set_line_width(size * 0.03);
                        c.set_text_rendering_mode(TextRenderingMode::FillStroke);
                    }
                    c.set_horizontal_scaling(scale);
                    let skew = if fake_italic { 0.2 } else { 0.0 };
                    c.set_text_matrix([1.0, 0.0, skew, 1.0, x0, base]);
                    for run in &runs {
                        let n = used_faces
                            .iter()
                            .position(|f| *f == run.face)
                            .unwrap_or_else(|| {
                                used_faces.push(run.face);
                                used_faces.len() - 1
                            });
                        c.set_font(Name(format!("F{n}").as_bytes()), size);
                        let bytes: Vec<u8> =
                            run.glyphs.iter().flat_map(|g| g.to_be_bytes()).collect();
                        c.show(Str(&bytes));
                    }
                    c.end_text();
                    if let Some(href) = op["link"].as_str() {
                        if href.starts_with("http://")
                            || href.starts_with("https://")
                            || href.starts_with("mailto:")
                        {
                            page_links.push((
                                Rect::new(
                                    x0,
                                    base - size * 0.25,
                                    x0 + target.max(natural),
                                    base + size * 0.9,
                                ),
                                href.to_string(),
                            ));
                        }
                    }
                }
                "begin" => {
                    let tag = op["tag"].as_str().unwrap_or("P");
                    if tag == "Artifact" {
                        c.begin_marked_content(Name(b"Artifact"));
                        open.push(None);
                    } else {
                        let mut props =
                            c.begin_marked_content_with_properties(Name(if tag.starts_with('H') {
                                b"H"
                            } else {
                                b"P"
                            }));
                        props.properties().identify(mcid);
                        drop(props);
                        tagged.push(Tagged {
                            role: role(tag),
                            alt: op["alt"].as_str().map(String::from),
                            page: p,
                            mcids: vec![mcid],
                        });
                        mcid += 1;
                        open.push(Some(tagged.len() - 1));
                    }
                }
                "end" => {
                    if open.pop().is_some() {
                        c.end_marked_content();
                    }
                }
                _ => {}
            }
        }
        while open.pop().is_some() {
            c.end_marked_content();
        }
        links.push(page_links);
        contents.push((c.finish().into_vec(), used_faces, used_images));
    }

    // Fonts: a Type0 font with an Identity-H CID font over each subset.
    let mut font_refs = vec![];
    for (_, used) in &fonts.faces {
        let Some(face) = used.loaded.face() else {
            font_refs.push(None);
            continue;
        };
        let type0 = ids.next();
        let cid = ids.next();
        let descriptor = ids.next();
        let file = ids.next();
        let cmap = ids.next();
        let units = face.units_per_em() as f32;
        let scale = |v: f32| v * 1000.0 / units;
        let subset = subsetter::subset(&used.loaded.data, used.loaded.index, &used.remapper)
            .map_err(|e| format!("A font could not be embedded: {e:?}"))?;
        let tag: String = {
            let mut h: u32 = 2166136261;
            for b in used
                .name
                .bytes()
                .chain(used.loaded.index.to_le_bytes())
                .chain(used.remapper.num_gids().to_le_bytes())
            {
                h = (h ^ b as u32).wrapping_mul(16777619);
            }
            (0..6)
                .map(|i| (b'A' + ((h >> (i * 5)) % 26) as u8) as char)
                .collect()
        };
        let base = format!("{tag}+{}", used.name);
        pdf.type0_font(type0)
            .base_font(Name(base.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid)
            .to_unicode(cmap);
        let widths: Vec<f32> = used
            .remapper
            .remapped_gids()
            .map(|old| {
                scale(
                    face.glyph_hor_advance(ttf_parser::GlyphId(old))
                        .unwrap_or(0) as f32,
                )
            })
            .collect();
        let mut cid_font = pdf.cid_font(cid);
        cid_font
            .subtype(CidFontType::Type2)
            .base_font(Name(base.as_bytes()))
            .system_info(SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .font_descriptor(descriptor)
            .default_width(0.0)
            .cid_to_gid_map_predefined(Name(b"Identity"));
        cid_font.widths().consecutive(0, widths);
        cid_font.finish();
        let bbox = face.global_bounding_box();
        let mut flags = FontFlags::SYMBOLIC;
        if face.is_italic() {
            flags |= FontFlags::ITALIC;
        }
        if face.is_monospaced() {
            flags |= FontFlags::FIXED_PITCH;
        }
        pdf.font_descriptor(descriptor)
            .name(Name(base.as_bytes()))
            .flags(flags)
            .bbox(Rect::new(
                scale(bbox.x_min as f32),
                scale(bbox.y_min as f32),
                scale(bbox.x_max as f32),
                scale(bbox.y_max as f32),
            ))
            .italic_angle(face.italic_angle())
            .ascent(scale(face.ascender() as f32))
            .descent(scale(face.descender() as f32))
            .cap_height(scale(
                face.capital_height().unwrap_or(face.ascender()) as f32
            ))
            .stem_v(80.0)
            .font_file2(file);
        let data = deflate(&subset);
        pdf.stream(file, &data).filter(Filter::FlateDecode);
        let mut map = UnicodeCmap::new(
            Name(b"Custom"),
            SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"UCS"),
                supplement: 0,
            },
        );
        for (gid, text) in &used.unicode {
            for c in text.chars().take(1) {
                map.pair(*gid, c);
            }
        }
        let map = map.finish();
        pdf.cmap(cmap, &map);
        font_refs.push(Some(type0));
    }

    // Pages, their annotations and structure parents.
    let mut annotation_refs: Vec<Vec<Ref>> = vec![];
    for page_links in &links {
        annotation_refs.push(page_links.iter().map(|_| ids.next()).collect());
    }
    for (p, page) in pages.iter().enumerate() {
        let (width, height) = (num(&page["width"]) * PT, page_heights[p]);
        let (data, used_faces, used_images) = &contents[p];
        let compressed = deflate(data);
        pdf.stream(content_refs[p], &compressed)
            .filter(Filter::FlateDecode);
        let mut page_writer = pdf.page(page_refs[p]);
        page_writer
            .parent(tree)
            .media_box(Rect::new(0.0, 0.0, width, height))
            .contents(content_refs[p])
            .struct_parents(p as i32);
        page_writer.tab_order(pdf_writer::types::TabOrder::StructureOrder);
        if !annotation_refs[p].is_empty() {
            page_writer.annotations(annotation_refs[p].iter().copied());
        }
        let mut resources = page_writer.resources();
        let mut font_dict = resources.fonts();
        for (n, face) in used_faces.iter().enumerate() {
            if let Some(Some(r)) = font_refs.get(*face) {
                font_dict.pair(Name(format!("F{n}").as_bytes()), *r);
            }
        }
        font_dict.finish();
        let mut xobjects = resources.x_objects();
        for (n, key) in used_images.iter().enumerate() {
            if let Some(r) = images.get(key) {
                xobjects.pair(Name(format!("Im{n}").as_bytes()), *r);
            }
        }
        xobjects.finish();
        resources.finish();
        page_writer.finish();
        for ((area, href), id) in links[p].iter().zip(&annotation_refs[p]) {
            let mut a = pdf.annotation(*id);
            a.subtype(AnnotationType::Link)
                .rect(*area)
                .border(0.0, 0.0, 0.0, None);
            a.action()
                .action_type(ActionType::Uri)
                .uri(Str(href.as_bytes()));
            a.finish();
        }
    }
    pdf.pages(tree)
        .kids(page_refs.iter().copied())
        .count(page_refs.len() as i32);

    // Structure: Document > H1…H6 / P / Figure, each over its marked content.
    let element_refs: Vec<Ref> = tagged.iter().map(|_| ids.next()).collect();
    for (t, id) in tagged.iter().zip(&element_refs) {
        let mut e = pdf.struct_element(*id);
        e.kind(t.role)
            .parent(document_element)
            .page(page_refs[t.page]);
        if let Some(alt) = &t.alt {
            e.alt(TextStr(alt));
        }
        let mut kids = e.children();
        for m in &t.mcids {
            kids.marked_content_id(*m);
        }
        kids.finish();
        e.finish();
    }
    let mut document = pdf.struct_element(document_element);
    document.kind(StructRole::Document).parent(struct_root);
    let mut kids = document.children();
    for r in &element_refs {
        kids.struct_element(*r);
    }
    kids.finish();
    document.finish();
    // The parent tree: each page's marked content, in MCID order.
    let parent_arrays: Vec<Ref> = pages.iter().map(|_| ids.next()).collect();
    for (p, id) in parent_arrays.iter().enumerate() {
        let refs: Vec<Ref> = tagged
            .iter()
            .zip(&element_refs)
            .filter(|(t, _)| t.page == p)
            .map(|(_, r)| *r)
            .collect();
        pdf.indirect(*id).array().items(refs);
    }
    let mut root = pdf.indirect(struct_root).start::<StructTreeRoot>();
    root.child(document_element);
    let mut parents = root.parent_tree();
    let mut nums = parents.nums();
    for (p, id) in parent_arrays.iter().enumerate() {
        nums.insert(p as i32, *id);
    }
    nums.finish();
    parents.finish();
    root.parent_tree_next_key(pages.len() as i32);
    root.finish();

    // The outline: headings, as bookmarks to their page.
    let outline = request["outline"].as_array().cloned().unwrap_or_default();
    let outline_root = (!outline.is_empty()).then(|| ids.next());
    if let Some(root) = outline_root {
        let items: Vec<Ref> = outline.iter().map(|_| ids.next()).collect();
        for (i, (item, id)) in outline.iter().zip(&items).enumerate() {
            let page = (item["page"].as_u64().unwrap_or(0) as usize).min(page_refs.len() - 1);
            let top = page_heights[page] - num(&item["y"]) * PT;
            let mut o = pdf.outline_item(*id);
            o.title(TextStr(item["title"].as_str().unwrap_or("")))
                .parent(root);
            if i > 0 {
                o.prev(items[i - 1]);
            }
            if i + 1 < items.len() {
                o.next(items[i + 1]);
            }
            o.dest().page(page_refs[page]).xyz(0.0, top, None);
            o.finish();
        }
        pdf.outline(root)
            .first(items[0])
            .last(*items.last().unwrap())
            .count(items.len() as i32);
    }

    let title = request["title"].as_str().unwrap_or("Document");
    pdf.document_info(info)
        .title(TextStr(title))
        .producer(TextStr("Folio"));
    let mut cat = pdf.catalog(catalog);
    cat.pages(tree);
    cat.lang(TextStr(request["lang"].as_str().unwrap_or("en")));
    cat.mark_info().marked(true);
    cat.viewer_preferences().display_doc_title(true);
    if let Some(root) = outline_root {
        cat.outlines(root);
    }
    cat.pair(Name(b"StructTreeRoot"), struct_root);
    cat.finish();
    Ok(pdf.finish())
}
