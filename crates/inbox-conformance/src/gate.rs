//! The gate on the report ESS writes from the results: 0 failed, 0 errored, 0 skipped, and the
//! unsupported scenarios exactly those `unsupported.tsv` names.

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
}

/// Reads an `ess-conformance-report/2`.
///
/// # Errors
///
/// A document that is not JSON, not a report/2, or whose counts disagree with its lists.
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
    Ok(Report {
        total: count("total")?,
        passed: count("passed")?,
        failed: ids("failed")?,
        error: ids("error")?,
        skipped: ids("skipped")?,
        unsupported: ids("unsupported")?,
    })
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
/// and the unsupported scenarios are exactly the listed ones.
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
    Verdict { line, problems }
}
