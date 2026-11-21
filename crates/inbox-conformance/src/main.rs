//! `inbox-conformance run --suite <suite> --results <results>`: runs the suite against
//! `inbox-core` and writes the `ess-conformance-results/1` document; exit 0 when it is written,
//! whatever the scenarios answered.
//!
//! `inbox-conformance gate --report <report> --unsupported <unsupported.tsv>`: prints the result
//! line of the report/2 ESS wrote from those results, and exits 1 unless 0 scenarios failed, 0
//! errored, 0 were skipped and the unsupported ones are exactly those the file lists.
//!
//! Exit 2: an argument, a file or a document this command cannot read.

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use inbox_conformance::{CoreTarget, gate, results, run_suite, suite};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match arguments.first().map(String::as_str) {
        Some("run") => run(&arguments[1..]),
        Some("gate") => check(&arguments[1..]),
        _ => Err(usage()),
    };
    match outcome {
        Ok(code) => code,
        Err(reason) => {
            eprintln!("inbox-conformance: {reason}");
            ExitCode::from(2)
        }
    }
}

fn usage() -> String {
    "usage: inbox-conformance run --suite <suite> --results <results> | gate --report <report> --unsupported <unsupported.tsv>".to_owned()
}

/// The value of each named option, in the order of `names`.
fn options<const N: usize>(arguments: &[String], names: [&str; N]) -> Result<[String; N], String> {
    let mut values: [Option<String>; N] = std::array::from_fn(|_| None);
    let mut rest = arguments.iter();
    while let Some(name) = rest.next() {
        let Some(at) = names.iter().position(|known| name == known) else {
            return Err(format!("unknown argument `{name}`; {}", usage()));
        };
        let value = rest
            .next()
            .ok_or_else(|| format!("`{name}` needs a value"))?;
        if values[at].replace(value.clone()).is_some() {
            return Err(format!("`{name}` is given twice"));
        }
    }
    let mut missing = names
        .iter()
        .zip(&values)
        .filter(|(_, value)| value.is_none());
    if let Some((name, _)) = missing.next() {
        return Err(format!("`{name}` is required; {}", usage()));
    }
    Ok(values.map(Option::unwrap_or_default))
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
}

fn run(arguments: &[String]) -> Result<ExitCode, String> {
    let [suite_path, results_path] = options(arguments, ["--suite", "--results"])?;
    let text = read(&suite_path)?;
    let suite = suite::read(&text).map_err(|reason| format!("{suite_path}: {reason}"))?;
    let outcomes = run_suite(&suite, &mut CoreTarget::new());
    for result in &outcomes {
        match &result.message {
            Some(message) => println!("  {} {}: {message}", result.status.name(), result.id),
            None => println!("  {} {}", result.status.name(), result.id),
        }
    }
    let completed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("the clock: {error}"))?
        .as_millis();
    let document = results::document(
        &results::suite_digest(text.as_bytes()),
        completed_at,
        &outcomes,
    );
    std::fs::write(&results_path, document).map_err(|error| format!("{results_path}: {error}"))?;
    Ok(ExitCode::SUCCESS)
}

fn check(arguments: &[String]) -> Result<ExitCode, String> {
    let [report_path, listed_path] = options(arguments, ["--report", "--unsupported"])?;
    let report = gate::read_report(&read(&report_path)?)
        .map_err(|reason| format!("{report_path}: {reason}"))?;
    let listed = gate::read_listed(&read(&listed_path)?)
        .map_err(|reason| format!("{listed_path}: {reason}"))?;
    let verdict = gate::verdict(&report, &listed);
    println!("  {}", verdict.line);
    for problem in &verdict.problems {
        println!("  {problem}");
    }
    Ok(if verdict.passes() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
