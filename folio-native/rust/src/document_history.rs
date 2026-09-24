//! Exact logical body-story changes travel with in-memory workspace history.
//! They are independent of the UI editor and never become document metadata.
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Journal {
    pub forward: Value,
    pub reverse: Value,
}
impl Journal {
    pub fn new(
        before: &Value,
        after: &Value,
        exact: Option<&Value>,
    ) -> Result<Option<Self>, String> {
        if let Some(exact) = exact {
            let reverse = crate::document_bookmarks::inverse_edits(before, after, exact)?;
            return Ok(Some(Self {
                forward: exact.clone(),
                reverse,
            }));
        }
        if before == after
            || crate::document_bookmarks::Index::new(before).text
                == crate::document_bookmarks::Index::new(after).text
        {
            return Ok(Some(Self {
                forward: json!([]),
                reverse: json!([]),
            }));
        }
        // An imported/replaced document without a supplied journal must not be
        // presented as an exact positional edit inferred from repeated text.
        Ok(None)
    }
    pub fn bytes(&self) -> usize {
        self.forward.to_string().len() + self.reverse.to_string().len()
    }
}

/// One undo or redo step: the patch that turns the current workspace into
/// the one on the other side of the step. Its size is measured once.
pub struct Entry {
    pub patch: crate::json_patch::Patch,
    pub document: Option<Journal>,
    bytes: usize,
}
impl Entry {
    pub fn new(patch: crate::json_patch::Patch, document: Option<Journal>) -> Self {
        let bytes = patch.bytes() + document.as_ref().map_or(0, Journal::bytes);
        Self {
            patch,
            document,
            bytes,
        }
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

const MAX_UNDO: usize = 100;
const MAX_UNDO_BYTES: usize = 24_000_000;

/// Undo and redo stacks. The undo stack keeps at most `MAX_UNDO` entries
/// and `MAX_UNDO_BYTES` of patches, evicting the oldest first; the running
/// byte total makes each commit's accounting O(1).
#[derive(Default)]
pub struct History {
    undo: std::collections::VecDeque<Entry>,
    redo: Vec<Entry>,
    undo_bytes: usize,
}
impl History {
    /// A new edit: record it and discard the redo stack.
    pub fn record(&mut self, entry: Entry) {
        self.push_undo(entry);
        self.redo.clear();
    }
    pub fn push_undo(&mut self, entry: Entry) {
        self.undo_bytes += entry.bytes();
        self.undo.push_back(entry);
        while self.undo.len() > MAX_UNDO || self.undo_bytes > MAX_UNDO_BYTES {
            let Some(old) = self.undo.pop_front() else {
                break;
            };
            self.undo_bytes -= old.bytes();
        }
    }
    pub fn pop_undo(&mut self) -> Option<Entry> {
        let entry = self.undo.pop_back()?;
        self.undo_bytes -= entry.bytes();
        Some(entry)
    }
    pub fn push_redo(&mut self, entry: Entry) {
        self.redo.push(entry);
    }
    pub fn pop_redo(&mut self) -> Option<Entry> {
        self.redo.pop()
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }
    pub fn last_undo(&self) -> Option<&Entry> {
        self.undo.back()
    }
    /// The workspace changed from `before` to `after` without an edit (for
    /// example automatic row sizes). Undo and redo must still land exactly
    /// on the neighbouring workspaces, as full snapshots used to, so the two
    /// entries adjacent to the current state are re-based onto `after`.
    pub fn rebase(&mut self, before: &Value, after: &Value) -> Result<(), String> {
        let top_undo = self.undo.pop_back();
        let top_redo = self.redo.pop();
        for (entry, undo) in [(top_undo, true), (top_redo, false)] {
            let Some(entry) = entry else { continue };
            let mut target = before.clone();
            crate::json_patch::apply_inverse(&mut target, entry.patch)?;
            let patch = crate::json_patch::diff(after, &target).without_field("revision");
            let entry = Entry::new(patch, entry.document);
            if undo {
                self.push_undo(entry);
            } else {
                self.push_redo(entry);
            }
        }
        Ok(())
    }
}
