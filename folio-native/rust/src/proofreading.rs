use harper_core::linting::{LintGroup, LintKind, Linter, Suggestion};
use harper_core::parsers::PlainEnglish;
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};

struct Engine {
    dialect: Dialect,
    linter: LintGroup,
}
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

pub fn check(request: &Value) -> Result<Value, String> {
    let text = request["text"].as_str().ok_or("Text is required")?;
    if text.encode_utf16().count() > 16000 {
        return Err("Check passages of at most 16,000 characters".into());
    }
    let dialect = match request["language"].as_str().unwrap_or("en-US") {
        "en-US" => Dialect::American,
        "en-GB" => Dialect::British,
        "en-CA" => Dialect::Canadian,
        "en-AU" => Dialect::Australian,
        "en-IN" => Dialect::Indian,
        _ => return Err("Built-in proofreading currently supports English only".into()),
    };
    let mut engine = ENGINE
        .get_or_init(|| {
            Mutex::new(Engine {
                dialect,
                linter: LintGroup::new_curated(FstDictionary::curated(), dialect),
            })
        })
        .lock()
        .map_err(|_| "Proofreading engine unavailable")?;
    if engine.dialect != dialect {
        engine.linter = LintGroup::new_curated(FstDictionary::curated(), dialect);
        engine.dialect = dialect;
    }
    let document = Document::new_curated(text, &PlainEnglish);
    let chars: Vec<char> = text.chars().collect();
    // Harper uses Unicode scalar positions. Flutter/Quill use UTF-16 positions.
    let mut offsets = vec![0];
    for c in &chars {
        offsets.push(offsets.last().unwrap() + c.len_utf16());
    }
    let mut lints = engine.linter.lint(&document);
    harper_core::remove_overlaps(&mut lints);
    lints.sort_by_key(|lint| lint.span.start);
    let truncated = lints.len() > 300;
    let issues: Vec<_> = lints
        .into_iter()
        .take(300)
        .filter_map(|lint| {
            if lint.span.end > chars.len() || lint.span.start >= lint.span.end {
                return None;
            }
            let original: String = chars[lint.span.start..lint.span.end].iter().collect();
            let suggestions: Vec<String> = lint
                .suggestions
                .iter()
                .take(6)
                .map(|s| match s {
                    Suggestion::ReplaceWith(v) => v.iter().collect(),
                    Suggestion::InsertAfter(v) => {
                        format!("{}{}", original, v.iter().collect::<String>())
                    }
                    Suggestion::Remove => String::new(),
                })
                .collect();
            let category = match lint.lint_kind {
                LintKind::Spelling => "spelling",
                LintKind::Style
                | LintKind::Enhancement
                | LintKind::Readability
                | LintKind::Redundancy => "style",
                _ => "grammar",
            };
            Some(
                json!({"start":offsets[lint.span.start],"end":offsets[lint.span.end],
            "text":original,"category":category,"rule":lint.lint_kind.to_string_key(),
            "message":lint.message,"suggestions":suggestions}),
            )
        })
        .collect();
    Ok(
        json!({"issues":issues,"truncated":truncated,"engine":"Harper 2.11.0","language":request["language"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_spelling_grammar_and_utf16_ranges() {
        let text = "😀 This is an test. We recieved the the letter.";
        let result = check(&json!({"text":text,"language":"en-US"})).unwrap();
        let issues = result["issues"].as_array().unwrap();
        assert!(issues
            .iter()
            .any(|i| i["category"] == "spelling" && i["text"] == "recieved"));
        assert!(issues.iter().any(|i| i["category"] == "grammar"));
        let units: Vec<u16> = text.encode_utf16().collect();
        for i in issues {
            let a = i["start"].as_u64().unwrap() as usize;
            let b = i["end"].as_u64().unwrap() as usize;
            assert_eq!(
                String::from_utf16(&units[a..b]).unwrap(),
                i["text"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn validates_language_limits_and_dialect() {
        assert!(check(&json!({"text":"Bonjour","language":"fr"})).is_err());
        assert!(check(&json!({"text":"a".repeat(16001)})).is_err());
        assert_eq!(check(&json!({"text":""})).unwrap()["issues"], json!([]));
        let us = check(&json!({"text":"The colour is blue.","language":"en-US"})).unwrap();
        let gb = check(&json!({"text":"The colour is blue.","language":"en-GB"})).unwrap();
        assert!(us["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["text"] == "colour"));
        assert!(!gb["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["text"] == "colour"));
    }
}
