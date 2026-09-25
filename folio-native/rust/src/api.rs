use crate::workspace::Core;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};
static CORE: OnceLock<Mutex<Core>> = OnceLock::new();
// flutter_rust_bridge runs this on its worker pool. No calculation or file IO on Flutter's UI thread.
pub fn dispatch(request: String) -> String {
    dispatch_on(CORE.get_or_init(|| Mutex::new(Core::new())), &request)
}

const INTERNAL_ERROR: &str = "Folio hit an internal error and did not finish this action. \
Please report the file or steps that caused it.";

/// Handles one request against `core`. A panic anywhere in the engine becomes
/// an error response, and the engine is re-synced so later requests work: a
/// bad file must never take the whole app down.
pub(crate) fn dispatch_on(core: &Mutex<Core>, request: &str) -> String {
    let result = (|| {
        let r: Value = serde_json::from_str(request).map_err(|e| e.to_string())?;
        // Proofreading has its own lock; typing, saving and calculation need not
        // wait for language analysis. The bridge executes this off the UI thread.
        if r["op"] == "proofread" {
            return catch_unwind(|| crate::proofreading::check(&r))
                .unwrap_or_else(|_| Err(INTERNAL_ERROR.into()));
        }
        // A panic while the lock was held poisons it; the recovery below has
        // already restored a consistent engine, so keep using it.
        let mut core = core.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match catch_unwind(AssertUnwindSafe(|| core.dispatch(r))) {
            Ok(result) => result,
            Err(_) => {
                let _ = catch_unwind(AssertUnwindSafe(|| core.recover_after_panic()));
                Err(INTERNAL_ERROR.into())
            }
        }
    })();
    match result {
        Ok(v) => json!({"ok":true,"value":v}).to_string(),
        Err(e) => json!({"ok":false,"error":e}).to_string(),
    }
}
