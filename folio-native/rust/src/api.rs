use crate::workspace::Core;
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
static CORE: OnceLock<Mutex<Core>> = OnceLock::new();
// flutter_rust_bridge runs this on its worker pool. No calculation or file IO on Flutter's UI thread.
pub fn dispatch(request: String) -> String {
    let result = (|| {
        let r: Value = serde_json::from_str(&request).map_err(|e| e.to_string())?;
        // Proofreading has its own lock; typing, saving and calculation need not
        // wait for language analysis. The bridge executes this off the UI thread.
        if r["op"] == "proofread" {
            return crate::proofreading::check(&r);
        }
        let mut core = CORE
            .get_or_init(|| Mutex::new(Core::new()))
            .lock()
            .map_err(|_| "Workspace lock unavailable")?;
        core.dispatch(r)
    })();
    match result {
        Ok(v) => json!({"ok":true,"value":v}).to_string(),
        Err(e) => json!({"ok":false,"error":e}).to_string(),
    }
}
