//! The gate on the report ESS writes from the results: 0 failed, 0 errored, 0 skipped, and the
//! unsupported scenarios exactly those `unsupported.tsv` names; the report bound to the suite
//! (its digest, each of its scenario ids once) and, where one is given, to the implementation.

use std::collections::BTreeMap;

use inbox::server::json::{self, Value};

/// The scenarios the runner is known not to interpret, by id, each with its reason.
pub type Listed = BTreeMap<String, String>;

/// Reads `unsupported.tsv`: one line per scenario, its id, a tab, and the reason. An empty file
/// lists none.
///
/// # Errors
///
/// A line with no tab, an empty id or reason, or an id named twice.
pub fn read_listed(text: &str) -> Result<Listed, String> {
    let mut listed = Listed::new();
    for (at, line) in text.lines().enumerate() {
        let number = at + 1;
        let Some((id, reason)) = line.split_once('\t') else {
            return Err(format!(
                "line {number}: expected `<scenario id>\\t<reason>`, found {line:?}"
            ));
        };
        if id.trim().is_empty() || reason.trim().is_empty() {
            return Err(format!(
                "line {number}: the id and the reason are both required"
            ));
        }
        if listed.insert(id.to_owned(), reason.to_owned()).is_some() {
            return Err(format!("line {number}: `{id}` is listed twice"));
        }
    }
    Ok(listed)
}

/// What a report/2 says, as far as the gate reads it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// `counts.total`.
    pub total: u64,
    /// `counts.passed`.
    pub passed: u64,
    /// `outcomes.failed`.
    pub failed: Vec<String>,
    /// `outcomes.error`.
    pub error: Vec<String>,
    /// `outcomes.skipped`.
    pub skipped: Vec<String>,
    /// `outcomes.unsupported`.
    pub unsupported: Vec<String>,
    /// The reason the run gave each unsupported scenario, by id. A report/2 carries none: they
    /// are read from the results document the report was written from ([`read_reasons`]).
    pub reasons: BTreeMap<String, String>,
}

/// The `ess-conformance-results/1` document `inbox-conformance run` wrote, as far as the gate
/// holds it to the report written from it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Results {
    /// `completed_at`, as spelled.
    pub completed_at: String,
    /// Each result's `scenario_id` and `status`, in the document's order.
    pub statuses: Vec<(String, String)>,
    /// The `message` of each result whose `status` is `unsupported`, by id.
    pub reasons: BTreeMap<String, String>,
}

/// Reads the results document of the suite whose bytes have `suite_digest`.
///
/// # Errors
///
/// A document that is not JSON, not a results/1, for another suite, without `completed_at`, with
/// a result without its id or status, an id named twice, or an unsupported result without its
/// message.
pub fn read_results(text: &str, suite_digest: &str) -> Result<Results, String> {
    let reasons = read_reasons(text, suite_digest)?;
    let document = json::parse_written(text).map_err(|error| format!("the results: {error}"))?;
    let completed_at = match document.member("completed_at") {
        Some(Value::Number(spelling)) => spelling.clone(),
        _ => return Err("the results have no `completed_at`".to_owned()),
    };
    let Some(Value::Array(items)) = document.member("results") else {
        return Err("the results have no `results` list".to_owned());
    };
    let mut statuses: Vec<(String, String)> = Vec::new();
    for item in items {
        let (Some(Value::Text(id)), Some(Value::Text(status))) =
            (item.member("scenario_id"), item.member("status"))
        else {
            return Err("the results hold a result without its id and its status".to_owned());
        };
        if statuses.iter().any(|(named, _)| named == id) {
            return Err(format!("the results name `{id}` twice"));
        }
        statuses.push((id.clone(), status.clone()));
    }
    Ok(Results {
        completed_at,
        statuses,
        reasons,
    })
}

/// Holds `results` to the report they were turned into: each id the report lists with the
/// status the results give it, the same ids in both, and the same `completed_at` (ESS 0.96.0
/// copies it from the results into the report). One problem per id that differs.
pub fn agrees(binding: &Binding, results: &Results) -> Vec<String> {
    let mut problems = Vec::new();
    if binding.completed_at.as_deref() != Some(results.completed_at.as_str()) {
        problems.push(format!(
            "the results were completed at {}, and the report at {}",
            results.completed_at,
            binding.completed_at.as_deref().unwrap_or("no instant")
        ));
    }
    for (list, id) in &binding.listed {
        match results.statuses.iter().find(|(named, _)| named == id) {
            Some((_, status)) if status == list => {}
            Some((_, status)) => problems.push(format!(
                "the results give {id} the status `{status}`, and the report lists it as `{list}`"
            )),
            None => problems.push(format!(
                "the results do not name {id}, which the report lists as `{list}`"
            )),
        }
    }
    problems.extend(
        results
            .statuses
            .iter()
            .filter(|(id, _)| !binding.listed.iter().any(|(_, listed)| listed == id))
            .map(|(id, _)| format!("the results name {id}, which the report does not list")),
    );
    problems
}

/// The reason the run gave each unsupported scenario, by id, read from the
/// `ess-conformance-results/1` document `inbox-conformance run` wrote (the `message` of each
/// result whose `status` is `unsupported`), held to the suite whose bytes have `suite_digest`.
///
/// # Errors
///
/// A document that is not JSON, not a results/1, for another suite, or with an unsupported
/// result without its id or its message.
pub fn read_reasons(text: &str, suite_digest: &str) -> Result<BTreeMap<String, String>, String> {
    let document = json::parse_written(text).map_err(|error| format!("the results: {error}"))?;
    match document.member("format") {
        Some(Value::Text(format)) if format == crate::results::RESULTS_FORMAT => {}
        _ => {
            return Err(format!(
                "the results are not an `{}`",
                crate::results::RESULTS_FORMAT
            ));
        }
    }
    match document.member("suite_digest") {
        Some(Value::Text(digest)) if digest == suite_digest => {}
        _ => {
            return Err(format!(
                "the results are not those of the suite {suite_digest}"
            ));
        }
    }
    let Some(Value::Array(results)) = document.member("results") else {
        return Err("the results have no `results` list".to_owned());
    };
    let mut reasons = BTreeMap::new();
    for result in results {
        if !matches!(result.member("status"), Some(Value::Text(status)) if status == "unsupported")
        {
            continue;
        }
        match (result.member("scenario_id"), result.member("message")) {
            (Some(Value::Text(id)), Some(Value::Text(message))) => {
                if reasons.insert(id.clone(), message.clone()).is_some() {
                    return Err(format!("the results name `{id}` twice"));
                }
            }
            _ => {
                return Err(
                    "the results hold an unsupported scenario without its id and its message"
                        .to_owned(),
                );
            }
        }
    }
    Ok(reasons)
}

/// Reads an `ess-conformance-report/2`.
///
/// # Errors
///
/// A document that is not JSON, not a report/2, whose counts disagree with its lists (passed
/// included), or whose total is not the sum of its outcomes.
pub fn read_report(text: &str) -> Result<Report, String> {
    let document = json::parse_written(text).map_err(|error| format!("the report: {error}"))?;
    match document.member("format") {
        Some(Value::Text(format)) if format == "ess-conformance-report/2" => {}
        _ => return Err("the report is not an `ess-conformance-report/2`".to_owned()),
    }
    let counts = document
        .member("counts")
        .ok_or("the report has no `counts`")?;
    let outcomes = document
        .member("outcomes")
        .ok_or("the report has no `outcomes`")?;
    let count = |name: &str| -> Result<u64, String> {
        match counts.member(name) {
            Some(Value::Number(spelling)) => spelling
                .parse::<u64>()
                .map_err(|_| format!("`counts.{name}` is not a count")),
            _ => Err(format!("the report has no `counts.{name}`")),
        }
    };
    let ids = |name: &str| -> Result<Vec<String>, String> {
        let listed = match outcomes.member(name) {
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| match item {
                    Value::Text(id) => Ok(id.clone()),
                    _ => Err(format!("`outcomes.{name}` holds a value that is no id")),
                })
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
            Some(_) => return Err(format!("`outcomes.{name}` is not a list")),
        };
        if listed.len() as u64 != count(name)? {
            return Err(format!(
                "the report counts {} `{name}` and lists {}",
                count(name)?,
                listed.len()
            ));
        }
        Ok(listed)
    };
    let passed = ids("passed")?;
    let report = Report {
        total: count("total")?,
        passed: passed.len() as u64,
        failed: ids("failed")?,
        error: ids("error")?,
        skipped: ids("skipped")?,
        unsupported: ids("unsupported")?,
        reasons: BTreeMap::new(),
    };
    if report.total != accounted(&report) {
        return Err(format!(
            "the report counts {} scenarios and accounts for {}",
            report.total,
            accounted(&report)
        ));
    }
    Ok(report)
}

/// The scenarios a report accounts for: its passed count and every listed id.
fn accounted(report: &Report) -> u64 {
    report.passed
        + (report.failed.len()
            + report.error.len()
            + report.skipped.len()
            + report.unsupported.len()) as u64
}

/// What binds a report/2 to a run: its verdict and the suite it was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// `conformance_status`.
    pub conformance_status: String,
    /// `suite.digest`: `sha256:<hex>` of the suite's bytes.
    pub suite_digest: String,
    /// `implementation`, where the report names one.
    pub implementation: Option<String>,
    /// Every id of the outcome lists (passed, failed, error, skipped, unsupported), in that
    /// order, as often as the lists name it.
    pub ids: Vec<String>,
    /// Each id of the outcome lists with the list that names it (`passed`, `failed`, `error`,
    /// `skipped`, `unsupported`), in the order of [`Binding::ids`].
    pub listed: Vec<(String, String)>,
    /// `completed_at`, as spelled, where the report carries it.
    pub completed_at: Option<String>,
}

/// The outcome lists of a report/2, in the order [`Binding::ids`] reads them.
const OUTCOMES: [&str; 5] = ["passed", "failed", "error", "skipped", "unsupported"];

/// Reads the [`Binding`] of an `ess-conformance-report/2`.
///
/// # Errors
///
/// A document that is not JSON, carries no `conformance_status` or `suite.digest`, an
/// `implementation` that is not text, or an outcome list holding something other than ids.
pub fn read_binding(text: &str) -> Result<Binding, String> {
    let document = json::parse_written(text).map_err(|error| format!("the report: {error}"))?;
    let Some(Value::Text(conformance_status)) = document.member("conformance_status") else {
        return Err("the report has no `conformance_status`".to_owned());
    };
    let Some(Value::Text(suite_digest)) = document
        .member("suite")
        .and_then(|suite| suite.member("digest"))
    else {
        return Err("the report has no `suite.digest`".to_owned());
    };
    let implementation = match document.member("implementation") {
        None => None,
        Some(Value::Text(name)) => Some(name.clone()),
        Some(_) => return Err("the report's `implementation` is not text".to_owned()),
    };
    let mut ids = Vec::new();
    let mut by_list = Vec::new();
    for name in OUTCOMES {
        let listed = document
            .member("outcomes")
            .and_then(|outcomes| outcomes.member(name));
        match listed {
            None => {}
            Some(Value::Array(items)) => {
                for item in items {
                    let Value::Text(id) = item else {
                        return Err(format!("`outcomes.{name}` holds a value that is no id"));
                    };
                    ids.push(id.clone());
                    by_list.push((name.to_owned(), id.clone()));
                }
            }
            Some(_) => return Err(format!("`outcomes.{name}` is not a list")),
        }
    }
    Ok(Binding {
        conformance_status: conformance_status.clone(),
        suite_digest: suite_digest.clone(),
        implementation,
        ids,
        listed: by_list,
        completed_at: match document.member("completed_at") {
            Some(Value::Number(spelling)) => Some(spelling.clone()),
            _ => None,
        },
    })
}

/// Holds the ids a report accounts for to the scenario ids of its suite: each of `scenarios`
/// named exactly once, and no other. One problem per id that breaks it.
pub fn accounts(binding: &Binding, scenarios: &[String]) -> Vec<String> {
    let mut named: BTreeMap<&str, usize> = BTreeMap::new();
    for id in &binding.ids {
        *named.entry(id.as_str()).or_default() += 1;
    }
    let mut problems = Vec::new();
    for scenario in scenarios {
        match named.get(scenario.as_str()) {
            None => problems.push(format!("the report does not account for {scenario}")),
            Some(1) => {}
            Some(times) => problems.push(format!("the report names {scenario} {times} times")),
        }
    }
    problems.extend(
        named
            .keys()
            .filter(|id| !scenarios.iter().any(|scenario| scenario == *id))
            .map(|id| format!("the report names {id}, which is no scenario of the suite")),
    );
    problems
}

/// Holds a report to the implementation it should be written for: a problem where its
/// `implementation` is another or absent.
pub fn implemented(binding: &Binding, expected: &str) -> Option<String> {
    match &binding.implementation {
        Some(name) if name == expected => None,
        Some(name) => Some(format!(
            "the report is for the implementation `{name}`, not `{expected}`"
        )),
        None => Some(format!(
            "the report names no implementation; `{expected}` was expected"
        )),
    }
}

/// The `conformance_status` ESS 0.96.0 gives a run of the coverage suite this gate passes:
/// `passed` where every scenario passed (the suite declares its coverage), and `failed` where the
/// only scenarios that did not pass are unsupported ones the list names (ESS fails any
/// unsupported scenario). `inconclusive`, the status of a suite without declared coverage, fails
/// the gate.
pub fn passing_status(report: &Report) -> &'static str {
    if report.unsupported.is_empty() {
        "passed"
    } else {
        "failed"
    }
}

/// Holds a report's binding to the suite the task synthesized (`suite_digest`) and to the status
/// a passing run carries; one problem per mismatch.
pub fn bound(report: &Report, binding: &Binding, suite_digest: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if binding.suite_digest != suite_digest {
        problems.push(format!(
            "the report is for the suite {}, not for {suite_digest}, the suite this run synthesized",
            binding.suite_digest
        ));
    }
    let expected = passing_status(report);
    if binding.conformance_status != expected {
        problems.push(format!(
            "conformance_status is `{}`; a passing run carries `{expected}`",
            binding.conformance_status
        ));
    }
    problems
}

/// The gate's answer: the result line, and every reason it fails; none where it passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// `<total> scenarios: <passed> passed, <failed> failed, <error> error, <unsupported>
    /// unsupported`, the shape of the interpreted run's line.
    pub line: String,
    /// Why the gate fails, one entry per scenario or condition.
    pub problems: Vec<String>,
}

impl Verdict {
    /// Whether the gate passes.
    pub fn passes(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Holds `report` to `listed`: no scenario failed, errored or skipped, something ran and passed,
/// the unsupported scenarios are exactly the listed ones, and each listed reason is the reason
/// the run gave that scenario ([`Report::reasons`]); one problem per scenario whose reason
/// differs or is unknown.
pub fn verdict(report: &Report, listed: &Listed) -> Verdict {
    let line = format!(
        "{} scenarios: {} passed, {} failed, {} error, {} unsupported",
        report.total,
        report.passed,
        report.failed.len(),
        report.error.len(),
        report.unsupported.len()
    );
    let mut problems = Vec::new();
    if report.total != accounted(report) {
        problems.push(format!(
            "the report counts {} scenarios and accounts for {}",
            report.total,
            accounted(report)
        ));
    }
    if report.total == 0 || report.passed == 0 {
        problems.push("the run passed no scenario: the suite did not run".to_owned());
    }
    problems.extend(report.failed.iter().map(|id| format!("failed: {id}")));
    problems.extend(report.error.iter().map(|id| format!("error: {id}")));
    problems.extend(report.skipped.iter().map(|id| format!("skipped: {id}")));
    problems.extend(
        report
            .unsupported
            .iter()
            .filter(|id| !listed.contains_key(*id))
            .map(|id| format!("unsupported and not listed in unsupported.tsv: {id}")),
    );
    problems.extend(
        listed
            .keys()
            .filter(|id| !report.unsupported.contains(id))
            .map(|id| format!("listed in unsupported.tsv and not unsupported in the run: {id}")),
    );
    for id in &report.unsupported {
        let Some(listed_reason) = listed.get(id) else {
            continue;
        };
        match report.reasons.get(id) {
            Some(reason) if reason == listed_reason => {}
            Some(reason) => problems.push(format!(
                "the reason unsupported.tsv lists for {id} is not the reason of the run: listed `{listed_reason}`, the run gave `{reason}`"
            )),
            None => problems.push(format!(
                "the run gave no reason for {id}, which unsupported.tsv lists with `{listed_reason}` (`gate --results` names the run's results)"
            )),
        }
    }
    Verdict { line, problems }
}
