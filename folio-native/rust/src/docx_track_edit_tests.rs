use crate::{docx, docx_revisions as review, docx_track_edit as track, office::*};
use serde_json::json;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn p(text: &str) -> String {
    format!(
        "<x:p xmlns:x=\"{W}\"><x:r><x:t xml:space=\"preserve\">{}</x:t></x:r></x:p>",
        esc(text)
    )
}
fn author(name: &str) -> track::Author {
    track::Author {
        name: name.into(),
        date: "2026-09-20T18:00:00Z".into(),
        enabled: true,
        formatting: true,
    }
}
fn edit(before: &str, after: &str, start: usize, end: usize, text: &str, name: &str) -> String {
    track::record(
        before,
        after,
        &json!([{"start":start,"end":end,"text":text}]),
        &author(name),
        before,
    )
    .unwrap()
}
fn pkg(xml: &str) -> Package {
    let mut p = docx::new_package();
    p.set(
        "word/document.xml",
        format!("<w:document xmlns:w=\"{W}\"><w:body>{xml}<w:sectPr/></w:body></w:document>"),
    );
    p.original = p.bytes().unwrap();
    p
}
fn reviewed(xml: &str, action: &str) -> String {
    let p = pkg(xml);
    let list = review::list(&p).unwrap();
    let keys = list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].clone())
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return contents(xml);
    }
    let result = review::resolve(&p, &json!({"action":action,"keys":keys})).unwrap();
    let xml = result.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    d.descendants()
        .filter(|n| n.has_tag_name((W, "t")))
        .filter_map(|n| n.text())
        .collect()
}
fn contents(xml: &str) -> String {
    let x = track::current_view(xml).unwrap();
    let d = parse(&x).unwrap();
    d.descendants()
        .filter(|n| n.has_tag_name((W, "t")))
        .filter_map(|n| n.text())
        .collect()
}
fn save(xml: &str, name: &str) {
    std::fs::write(
        std::env::temp_dir().join(format!("folio-track-edit-{name}.docx")),
        pkg(xml).original,
    )
    .unwrap();
}
#[test]
fn authored_replacement_repeated_text_unicode_and_review() {
    let before = p("aaaa😀a & end");
    let next = p("aaXa😀a & end");
    let result = edit(&before, &next, 2, 3, "X", "Alex");
    assert_eq!(contents(&result), "aaXa😀a & end");
    assert_eq!(reviewed(&result, "accept"), "aaXa😀a & end");
    assert_eq!(reviewed(&result, "reject"), "aaaa😀a & end");
    assert!(track::record(
        &before,
        &p("bad"),
        &json!([{"start":5,"end":6,"text":"x"}]),
        &author("Alex"),
        &before
    )
    .unwrap_err()
    .contains("Unicode"));
    assert!(track::record(
        &before,
        &next,
        &json!([{"start":0,"end":1,"text":"X"}]),
        &author("Alex"),
        &before
    )
    .is_err());
    save(&result, "replacement");
}
#[test]
fn own_insertions_extend_edit_and_cancel_without_phantom_deletions() {
    let first = edit(&p("Hello "), &p("Hello new"), 6, 6, "new", "Alex");
    let second = edit(&first, &p("Hello newer"), 9, 9, "er", "Alex");
    assert_eq!(
        review::list(&pkg(&second)).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let third = edit(&second, &p("Hello NEWer"), 6, 9, "NEW", "Alex");
    assert_eq!(reviewed(&third, "reject"), "Hello ");
    assert!(!parse(&third)
        .unwrap()
        .descendants()
        .any(|n| n.has_tag_name((W, "del"))));
    let removed = edit(&third, &p("Hello "), 6, 11, "", "Alex");
    assert!(review::list(&pkg(&removed)).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .is_empty());
    save(&third, "own-insertion");
}
#[test]
fn different_authors_retain_attribution_and_nested_deletion() {
    let a = edit(&p("A"), &p("ABC"), 1, 1, "BC", "Alex");
    let b = edit(&a, &p("AC"), 1, 2, "", "Morgan");
    let list = review::list(&pkg(&b)).unwrap();
    assert_eq!(list["revisions"][0]["author"], "Alex");
    assert_eq!(list["revisions"][1]["author"], "Morgan");
    assert_eq!(contents(&b), "AC");
    assert_eq!(reviewed(&b, "accept"), "AC");
    assert_eq!(reviewed(&b, "reject"), "A");
    save(&b, "authors");
}
#[test]
fn pending_deletions_survive_later_edits_and_tracking_off() {
    let first = edit(&p("Original tail"), &p("New tail"), 0, 8, "New", "Alex");
    let second = edit(&first, &p("New tail!"), 8, 8, "!", "Morgan");
    assert_eq!(reviewed(&second, "reject"), "Original tail");
    let mut off = author("");
    off.enabled = false;
    let off = track::record(
        &second,
        &p("New TAIL!"),
        &json!([{"start":4,"end":8,"text":"TAIL"}]),
        &off,
        &second,
    )
    .unwrap();
    assert_eq!(contents(&off), "New TAIL!");
    assert_eq!(reviewed(&off, "reject"), "Original TAIL");
    save(&off, "tracking-off");
}
#[test]
fn rich_formatting_changes_group_runs_and_keep_prior_snapshot() {
    let before=format!("<x:p xmlns:x=\"{W}\"><x:pPr><x:jc x:val=\"left\"/></x:pPr><x:r x:rsidR=\"12345678\"><x:rPr><x:i/><x:smallCaps/></x:rPr><x:t>Formatted</x:t></x:r></x:p>");
    let after = before
        .replace("left", "right")
        .replace("<x:i/>", "<x:b/><x:i/>");
    let result = track::record(&before, &after, &json!([]), &author("Alex"), &before).unwrap();
    let d = parse(&result).unwrap();
    assert_eq!(
        d.descendants()
            .filter(|n| n.has_tag_name((W, "rPrChange")))
            .count(),
        1
    );
    assert_eq!(
        d.descendants()
            .find(|n| n.has_tag_name((W, "r")))
            .unwrap()
            .attribute((W, "rsidR")),
        Some("12345678")
    );
    assert_eq!(
        d.descendants()
            .filter(|n| n.has_tag_name((W, "pPrChange")))
            .count(),
        1
    );
    let newer = after.replace("<x:smallCaps/>", "<x:smallCaps/><x:u x:val=\"single\"/>");
    let second = track::record(&result, &newer, &json!([]), &author("Alex"), &result).unwrap();
    let old = pkg(&second);
    let list = review::list(&old).unwrap();
    let ids = list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].clone())
        .collect::<Vec<_>>();
    let rejected = review::resolve(&old, &json!({"action":"reject","keys":ids})).unwrap();
    let d = parse(rejected.text("word/document.xml").unwrap()).unwrap();
    assert!(d.descendants().any(|n| n.has_tag_name((W, "smallCaps"))));
    assert!(!d
        .descendants()
        .any(|n| n.has_tag_name((W, "b")) || n.has_tag_name((W, "u"))));
    save(&second, "formatting");
}
#[test]
fn exact_disjoint_edits_empty_runs_tabs_and_line_breaks() {
    let result=track::record(&p("0123456789"),&p("0AB345Z789!"),&json!([{"start":1,"end":3,"text":"AB"},{"start":6,"end":7,"text":"Z"},{"start":10,"end":10,"text":"!"}]),&author("Alex"),&p("0123456789")).unwrap();
    assert_eq!(reviewed(&result, "accept"), "0AB345Z789!");
    assert_eq!(reviewed(&result, "reject"), "0123456789");
    let empty = edit(&p(""), &p("😀"), 0, 0, "😀", "Alex");
    assert_eq!(reviewed(&empty, "reject"), "");
    let next =
        format!("<x:p xmlns:x=\"{W}\"><x:r><x:t>A</x:t><x:tab/><x:br/><x:t>B</x:t></x:r></x:p>");
    let rich = edit(&p("AB"), &next, 1, 1, "\t\n", "Alex");
    let current = track::current_view(&rich).unwrap();
    let d = parse(&current).unwrap();
    assert!(d.descendants().any(|n| n.has_tag_name((W, "tab"))));
    assert!(d.descendants().any(|n| n.has_tag_name((W, "br"))));
    save(&rich, "breaks");
}
#[test]
fn invalid_journals_authors_and_unsupported_objects_fail_without_output() {
    let before = p("abc");
    let mut bad = author(" ");
    assert!(track::record(
        &before,
        &p("ab"),
        &json!([{"start":2,"end":3,"text":""}]),
        &bad,
        &before
    )
    .is_err());
    bad = author("Alex");
    bad.date = "yesterday".into();
    assert!(track::record(&before, &before, &json!([]), &bad, &before).is_err());
    assert!(track::record(
        &before,
        &before,
        &json!([{"start":0,"end":2,"text":"ab"},{"start":1,"end":2,"text":"b"}]),
        &author("Alex"),
        &before
    )
    .is_err());
    let field = before.replace("<x:t", "<x:fldChar x:fldCharType=\"begin\"/><x:t");
    assert!(track::current_view(&field).is_err());
    let source=format!("<x:p xmlns:x=\"{W}\"><x:pPr><x:rPr><x:del x:id=\"6\" x:author=\"Alex\"/></x:rPr></x:pPr><x:r><x:t>abc</x:t></x:r></x:p>");
    assert!(
        track::record(&source, &before, &json!([]), &author("Alex"), &source)
            .unwrap_err()
            .contains("structural")
    );
}
fn story_text(p: &[String]) -> String {
    track::current_story_view(p)
        .unwrap()
        .iter()
        .map(|x| format!("{}\n", contents(x)))
        .collect()
}
fn story_edit(
    before: &[String],
    after: &[String],
    start: usize,
    end: usize,
    text: &str,
    name: &str,
) -> Vec<String> {
    track::record_story(
        before,
        after,
        &json!([{"start":start,"end":end,"text":text}]),
        &author(name),
        &format!("<root>{}</root>", before.concat()),
    )
    .unwrap()
}
#[test]
fn authored_enter_backspace_cross_paragraph_replacement_and_review() {
    let before = vec![p("First second")];
    let split = story_edit(&before, &[p("First "), p("second")], 6, 6, "\n", "Alex");
    assert_eq!(story_text(&split), "First \nsecond\n");
    assert_eq!(reviewed(&split.concat(), "reject"), "First second");
    let joined = story_edit(&split, &[p("First second")], 6, 7, "", "Alex");
    assert_eq!(joined.len(), 1);
    assert!(review::list(&pkg(&joined.concat())).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .is_empty());
    let original = vec![p("First"), p("Second"), p("Third")];
    let joined = story_edit(
        &original,
        &[p("FirstSecond"), p("Third")],
        5,
        6,
        "",
        "Morgan",
    );
    assert_eq!(story_text(&joined), "FirstSecond\nThird\n");
    assert_eq!(joined.len(), 3);
    let replaced = story_edit(
        &joined,
        &[p("Fione"), p("twoird")],
        2,
        14,
        "one\ntwo",
        "Morgan",
    );
    assert_eq!(story_text(&replaced), "Fione\ntwoird\n");
    assert_eq!(reviewed(&replaced.concat(), "reject"), "FirstSecondThird");
    save(&split.concat(), "paragraph-split");
    save(&joined.concat(), "paragraph-join");
    save(&replaced.concat(), "paragraph-replacement");
}
#[test]
fn multiline_insertion_allocates_unique_revision_ids_and_last_mark_survives() {
    let old = vec![p("Base")];
    let result = story_edit(
        &old,
        &[p("Base one"), p("two"), p("three")],
        4,
        4,
        " one\ntwo\nthree",
        "Alex",
    );
    let xml = format!("<root>{}</root>", result.concat());
    let d = parse(&xml).unwrap();
    let ids = d
        .descendants()
        .filter(|n| n.has_tag_name((W, "ins")))
        .filter_map(|n| n.attribute((W, "id")))
        .collect::<Vec<_>>();
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::HashSet<_>>().len()
    );
    assert_eq!(reviewed(&result.concat(), "reject"), "Base");
    assert_eq!(story_text(&result), "Base one\ntwo\nthree\n");
    assert!(track::record_story(
        &old,
        &[p("")],
        &json!([{"start":0,"end":5,"text":"\n"}]),
        &author("Alex"),
        &old[0]
    )
    .is_err());
    save(&result.concat(), "multiline");
}

fn projected(
    source: &[String],
    before: &[String],
    after: &[String],
    edits: serde_json::Value,
) -> Vec<String> {
    track::record_projected_story(
        source,
        before,
        after,
        &edits,
        &author("Alex"),
        &format!("<root>{}</root>", source.concat()),
    )
    .unwrap()
}
#[test]
fn editor_projection_preserves_unmodeled_properties_in_edited_and_inserted_runs() {
    let source=format!("<x:p xmlns:x=\"{W}\" x:rsidR=\"12345678\"><x:pPr><x:keepNext/><x:spacing x:before=\"240\" x:after=\"120\"/><x:jc x:val=\"left\"/></x:pPr><x:r><x:rPr><x:rFonts x:ascii=\"Aptos\" x:hAnsi=\"Aptos\" x:eastAsia=\"Yu Gothic\"/><x:smallCaps/><x:lang x:val=\"en-US\" x:eastAsia=\"ja-JP\"/></x:rPr><x:t>hello</x:t></x:r></x:p>");
    let model=format!("<w:p xmlns:w=\"{W}\"><w:pPr><w:spacing w:before=\"240\"/><w:jc w:val=\"left\"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii=\"Aptos\" w:hAnsi=\"Aptos\"/></w:rPr><w:t>hello</w:t></w:r></w:p>");
    let after = model
        .replace("Aptos", "Arial")
        .replace("left", "right")
        .replace("240", "360")
        .replace("</w:rPr>", "<w:b/></w:rPr>")
        .replace("hello", "hello!");
    let result = projected(
        &[source],
        &[model],
        &[after],
        json!([{"start":5,"end":5,"text":"!"}]),
    );
    let current = track::current_story_view(&result).unwrap().concat();
    let d = parse(&current).unwrap();
    let runs = d
        .descendants()
        .filter(|n| n.has_tag_name((W, "r")))
        .collect::<Vec<_>>();
    assert!(!runs.is_empty());
    for r in runs {
        assert!(r.descendants().any(|n| n.has_tag_name((W, "smallCaps"))));
        assert!(r.descendants().any(|n| n.has_tag_name((W, "b"))));
        let f = r
            .descendants()
            .find(|n| n.has_tag_name((W, "rFonts")))
            .unwrap();
        assert_eq!(f.attribute((W, "eastAsia")), Some("Yu Gothic"));
        assert_eq!(f.attribute((W, "ascii")), Some("Arial"));
        assert!(r.descendants().any(|n| n.has_tag_name((W, "lang"))));
    }
    let spacing = d
        .descendants()
        .find(|n| n.has_tag_name((W, "spacing")))
        .unwrap();
    assert_eq!(spacing.attribute((W, "after")), Some("120"));
    assert_eq!(spacing.attribute((W, "before")), Some("360"));
    assert!(d.descendants().any(|n| n.has_tag_name((W, "keepNext"))));
    assert_eq!(reviewed(&result.concat(), "reject"), "hello");
    save(&result.concat(), "projected-properties");
}
#[test]
fn unchanged_projection_does_not_create_phantom_formatting_revisions() {
    let source = p("same").replace("<x:r>", "<x:r><x:rPr><x:smallCaps/></x:rPr>");
    let output = projected(
        &[source],
        &[p("same")],
        &[p("same!")],
        json!([{"start":4,"end":4,"text":"!"}]),
    );
    let xml = output.concat();
    let d = parse(&xml).unwrap();
    assert!(!d.descendants().any(|n| n.has_tag_name((W, "rPrChange"))));
    let wrong = track::record_projected_story(
        &output,
        &[p("stale")],
        &[p("wrong")],
        &json!([]),
        &author("Alex"),
        &xml,
    );
    assert!(wrong.unwrap_err().contains("projection"));
    save(&xml, "projected-typing");
}
#[test]
fn replacement_preview_orders_old_content_before_new_content() {
    let result = edit(&p("one old two"), &p("one new two"), 4, 7, "new", "Alex");
    let d = parse(&result).unwrap();
    let preview = review::preview(d.root_element());
    let texts = preview
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["text"].as_str().unwrap())
        .collect::<String>();
    assert_eq!(texts, "one oldnew two");
}
#[test]
fn paragraph_marks_with_existing_font_and_property_changes_stay_valid() {
    let before = p("First second").replace("<x:r>", "<x:pPr><x:rPr><x:b/></x:rPr></x:pPr><x:r>");
    let next = vec![
        p("First ").replace("<x:r>", "<x:pPr><x:rPr><x:b/></x:rPr></x:pPr><x:r>"),
        p("second"),
    ];
    let split = story_edit(&[before], &next, 6, 6, "\n", "Alex");
    assert_eq!(story_text(&split), "First \nsecond\n");
    save(&split.concat(), "paragraph-mark-properties");
}

#[test]
fn projected_empty_paragraph_typing_inherits_mark_format_and_font_overrides_theme() {
    let source = format!(
        "<x:p xmlns:x=\"{W}\"><x:pPr><x:rPr><x:b/><x:lang x:val=\"fr-FR\"/></x:rPr></x:pPr></x:p>"
    );
    let typed = projected(
        &[source],
        &[p("")],
        &[p("Hi")],
        json!([{"start":0,"end":0,"text":"Hi"}]),
    );
    let current = track::current_story_view(&typed).unwrap().concat();
    let d = parse(&current).unwrap();
    let run = d.descendants().find(|n| n.has_tag_name((W, "r"))).unwrap();
    assert!(run.descendants().any(|n| n.has_tag_name((W, "b"))));
    assert!(run
        .descendants()
        .any(|n| n.has_tag_name((W, "lang")) && n.attribute((W, "val")) == Some("fr-FR")));
    let source=format!("<x:rPr xmlns:x=\"{W}\" xmlns:w=\"urn:extension\"><x:rFonts x:ascii=\"Aptos\" x:asciiTheme=\"minorHAnsi\" x:eastAsia=\"Yu Gothic\"/></x:rPr>");
    let before = format!("<x:rPr xmlns:x=\"{W}\"><x:rFonts x:ascii=\"Aptos\"/></x:rPr>");
    let after = before
        .replace("Aptos", "Arial")
        .replace(" x:ascii=", " x:hAnsi=\"Arial\" x:ascii=");
    let result = crate::docx_track_properties::overlay(&source, &before, &after, "rPr").unwrap();
    let d = parse(&result).unwrap();
    let fonts = d
        .descendants()
        .find(|n| n.has_tag_name((W, "rFonts")))
        .unwrap();
    assert_eq!(fonts.attribute((W, "ascii")), Some("Arial"));
    assert_eq!(fonts.attribute((W, "hAnsi")), Some("Arial"));
    assert_eq!(fonts.attribute((W, "asciiTheme")), None);
    assert_eq!(fonts.attribute((W, "eastAsia")), Some("Yu Gothic"));
    save(&typed.concat(), "empty-paragraph-format");
}
