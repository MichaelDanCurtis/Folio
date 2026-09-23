use crate::{office::Package, office_templates, workspace::Core};
use serde_json::json;
use std::{fs, path::PathBuf};

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/templates")
}
fn output(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("folio-templates-{name}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn starter_templates_preview_create_save_and_recover_without_changing_sources() {
    let mut c = Core::new();
    let directory = output("starters");
    for name in [
        "letter.dotx",
        "report.dotx",
        "minutes.dotx",
        "budget.xltx",
        "invoice.xltx",
        "tracker.xltx",
    ] {
        let path = assets().join(name);
        let bytes = fs::read(&path).unwrap();
        let current = c.workspace.clone();
        let preview = c
            .dispatch(json!({"op":"inspectTemplate","path":path}))
            .unwrap();
        assert_eq!(current, c.workspace);
        let created = c.dispatch(json!({"op":"newFromTemplate","path":path,"expectedFingerprint":preview["fingerprint"]})).unwrap();
        assert_ne!(current["id"], created["workspace"]["id"]);
        assert_eq!(created["canUndo"], false);
        let kind = if name.ends_with("dotx") {
            "docx"
        } else {
            "xlsx"
        };
        assert_eq!(created["newFileKind"], kind);
        assert_ne!(
            created["fileStates"][kind],
            created["initialFileStates"][kind]
        );
        let untouched = if kind == "docx" { "xlsx" } else { "docx" };
        assert_eq!(
            created["fileStates"][untouched],
            created["initialFileStates"][untouched]
        );
        if name == "budget.xltx" {
            assert_eq!(created["workspace"]["sheets"][0]["cells"]["D2"], "=B2-C2");
            assert!(preview["sample"].to_string().contains("Personnel"));
            assert_eq!(preview["sample"][1][1], "$0.00");
            assert_eq!(preview["sample"][1][3], "$0.00");
        }
        let saved = directory.join(name.replace("dotx", "docx").replace("xltx", "xlsx"));
        c.dispatch(json!({"op":"exportOffice","format":kind,"path":saved}))
            .unwrap();
        let package = Package::read(saved.to_str().unwrap()).unwrap();
        assert!(!package
            .text("[Content_Types].xml")
            .unwrap()
            .contains(".template.main+xml"));
        c.dispatch(json!({"op":"save","path":directory.join("workspace.folio")}))
            .unwrap();
        let before = c.snapshot()["fileStates"].clone();
        c.dispatch(json!({"op":"load","path":directory.join("workspace.folio")}))
            .unwrap();
        assert_eq!(before, c.snapshot()["fileStates"]);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn template_export_and_edit_preserve_office_parts_and_dirty_identity() {
    let directory = output("parts");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    for (name, format) in [
        ("word-outline-levels.docx", "dotx"),
        ("structure.xlsx", "xltx"),
    ] {
        let mut c = Core::new();
        c.dispatch(json!({"op":"importOffice","path":fixtures.join(name)}))
            .unwrap();
        let kind = office_templates::document_format(format).unwrap();
        let plain = directory.join(format!("plain.{kind}"));
        c.dispatch(json!({"op":"exportOffice","format":kind,"path":plain}))
            .unwrap();
        let path = directory.join(format!("saved.{format}"));
        let result = c
            .dispatch(json!({"op":"exportOffice","format":format,"path":path}))
            .unwrap();
        assert_eq!(result["contentSignature"], c.snapshot()["fileStates"][kind]);
        let original = Package::read(plain.to_str().unwrap()).unwrap();
        let template = Package::read(path.to_str().unwrap()).unwrap();
        assert!(template
            .text("[Content_Types].xml")
            .unwrap()
            .contains(".template.main+xml"));
        for (name, value) in &original.entries {
            if name != "[Content_Types].xml" {
                assert_eq!(template.entries.get(name), Some(value), "{name}");
            }
        }
        c.dispatch(json!({"op":"importOffice","path":path}))
            .unwrap();
        let report = c.snapshot()["fileStates"][kind].clone();
        let editing = directory.join(format!("edited.{format}"));
        c.dispatch(json!({"op":"exportOffice","format":format,"path":editing}))
            .unwrap();
        assert_eq!(report, c.snapshot()["fileStates"][kind]);
        assert_eq!(
            template.entries,
            Package::read(editing.to_str().unwrap()).unwrap().entries
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn stale_corrupt_or_macro_templates_cannot_replace_current_work() {
    let directory = output("invalid");
    let mut c = Core::new();
    let current = c.workspace.clone();
    let path = assets().join("letter.dotx");
    assert!(c
        .dispatch(json!({"op":"newFromTemplate","path":path,"expectedFingerprint":"stale"}))
        .is_err());
    assert_eq!(c.workspace, current);
    let mut package = Package::read(path.to_str().unwrap()).unwrap();
    package
        .entries
        .insert("word/vbaProject.bin".into(), vec![1, 2, 3]);
    let macro_file = directory.join("renamed.dotx");
    fs::write(&macro_file, package.bytes().unwrap()).unwrap();
    assert!(c
        .dispatch(json!({"op":"newFromTemplate","path":macro_file}))
        .unwrap_err()
        .contains("Macro-enabled"));
    assert_eq!(c.workspace, current);
    fs::write(&macro_file, b"broken").unwrap();
    assert!(c
        .dispatch(json!({"op":"newFromTemplate","path":macro_file}))
        .is_err());
    assert_eq!(c.workspace, current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn content_type_conversion_is_namespace_aware_and_rejects_ambiguity() {
    let mut package = Package::read(assets().join("letter.dotx").to_str().unwrap()).unwrap();
    let xml = package
        .text("[Content_Types].xml")
        .unwrap()
        .replace("<Types xmlns=", "<ct:Types xmlns:ct=")
        .replace("</Types>", "</ct:Types>")
        .replace("<Default ", "<ct:Default ")
        .replace("<Override ", "<ct:Override ");
    package.set("[Content_Types].xml", xml.clone());
    package.original = package.bytes().unwrap();
    let converted = office_templates::convert(package.clone(), "docx").unwrap();
    assert!(converted
        .text("[Content_Types].xml")
        .unwrap()
        .contains("<ct:Types"));
    assert!(converted
        .text("[Content_Types].xml")
        .unwrap()
        .contains("wordprocessingml.document.main+xml"));
    let duplicate = xml.replace("</ct:Types>", "<ct:Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml\"/></ct:Types>");
    package.set("[Content_Types].xml", duplicate);
    assert!(office_templates::convert(package, "docx").is_err());
    assert_eq!(office_templates::format("Report.DoTx").unwrap(), "dotx");
}
