//! Content identity for working files, independent of the Undo revision counter.
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

struct DigestWriter(Sha256);
impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn digest(value: &impl Serialize) -> String {
    let mut writer = DigestWriter(Sha256::new());
    // These callers serialize JSON values, strings and byte collections only.
    serde_json::to_writer(&mut writer, value).expect("serializable file state");
    format!("{:x}", writer.0.finalize())
}

/// Package revisions change when preserved Office parts are edited. The file
/// identity must follow those edits, while a new import gets a new identity.
pub fn replace_package(workspace: &mut Value, key: &str, id: &str) {
    let kind = if key == "officeDocument" {
        "docx"
    } else {
        "xlsx"
    };
    let source = workspace[key]["sourceId"]
        .as_str()
        .or_else(|| workspace[key]["packageId"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}:{kind}", workspace["id"].as_str().unwrap()));
    workspace[key]["sourceId"] = json!(source);
    workspace[key]["packageId"] = json!(id);
}

// Length-prefix raw package parts: no Base64, ZIP rebuild, or decimal-byte JSON
// expansion on the interactive path. Cached by immutable Arc package identity.
pub fn package_digest(package: &crate::office::Package) -> String {
    let mut hasher = Sha256::new();
    for (name, bytes) in &package.entries {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}
