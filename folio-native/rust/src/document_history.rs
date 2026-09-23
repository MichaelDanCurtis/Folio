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

pub struct Entry {
    pub workspace: Value,
    pub document: Option<Journal>,
}
impl Entry {
    pub fn bytes(&self) -> usize {
        self.workspace.to_string().len() + self.document.as_ref().map_or(0, Journal::bytes)
    }
}
