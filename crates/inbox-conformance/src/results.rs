//! The `ess-conformance-results/1` document `ess verify conform report` turns into a report/2.

use std::fmt::Write as _;

use inbox::server::json;
use sha2::{Digest, Sha256};

use crate::runner::ScenarioResult;

/// The format the document declares.
pub const RESULTS_FORMAT: &str = "ess-conformance-results/1";

/// The digest of the suite's bytes, as the report names the suite: `sha256:<hex>`.
pub fn suite_digest(suite: &[u8]) -> String {
    let mut digest = String::from("sha256:");
    for byte in Sha256::digest(suite) {
        let _ = write!(digest, "{byte:02x}");
    }
    digest
}

/// The results document: one terminal status per scenario, each with its message where there is
/// one, for the suite whose bytes have `suite_digest`, completed at `completed_at` (milliseconds
/// since the Unix epoch).
pub fn document(suite_digest: &str, completed_at: u128, results: &[ScenarioResult]) -> String {
    let mut out = String::from("{");
    json::member(&mut out, "format");
    json::push_text(&mut out, RESULTS_FORMAT);
    json::member(&mut out, "suite_digest");
    json::push_text(&mut out, suite_digest);
    json::member(&mut out, "completed_at");
    out.push_str(&completed_at.to_string());
    json::member(&mut out, "results");
    out.push('[');
    for (at, result) in results.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push('{');
        json::member(&mut out, "scenario_id");
        json::push_text(&mut out, &result.id);
        json::member(&mut out, "status");
        json::push_text(&mut out, result.status.name());
        if let Some(message) = &result.message {
            json::member(&mut out, "message");
            json::push_text(&mut out, message);
        }
        out.push('}');
    }
    out.push_str("]}\n");
    out
}
