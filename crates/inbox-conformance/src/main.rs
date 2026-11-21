//! `inbox-conformance run --suite <suite> --results <results> [--target core|serve] [--inbox
//! <binary>]`: runs the suite against a target and writes the `ess-conformance-results/1`
//! document; exit 0 when it is written, whatever the scenarios answered. `core` (the default) is
//! `inbox-core` in process. `serve` is `<binary> serve`, started afresh for each scenario and
//! driven over HTTP (`--inbox` is then required): its key file and standard error are written
//! into the directory `serve` beside the results, and a service that does not start makes each
//! scenario it should have answered an error.
//!
//! `inbox-conformance gate --suite <suite> --report <report> --unsupported <unsupported.tsv>
//! [--results <results>] [--implementation <name>]`: prints the result line and the
//! `conformance_status` of the report/2 ESS wrote from those results, and exits 1 unless the
//! report is for that suite (its `suite.digest`, and each of its scenario ids named once in the
//! outcome lists, no other), for that implementation where `--implementation` is given, its
//! counts add up, 0 scenarios failed, 0 errored, 0 were skipped, the unsupported ones are exactly
//! those the file lists, each with the reason the run gave it, and its `conformance_status` is the
//! one such a run carries. A report/2 carries no reasons: they are read from `--results`, the
//! results document of the same suite the report was written from; without it, a listed
//! scenario has no reason of the run and fails the gate. The results are held to the report
//! too: each scenario id with the status of the list the report puts it in, no id more or less,
//! and the same `completed_at` (ESS copies it from the results into the report).
//!
//! Exit 2: an argument, a file or a document this command cannot read, among them a suite with a
//! top-level or `provenance` key the runner does not know or a member stated twice.

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use inbox_conformance::{CoreTarget, Target, WireTarget, gate, results, run_suite, suite};

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
    "usage: inbox-conformance run --suite <suite> --results <results> [--target core|serve] [--inbox <binary>] | gate --suite <suite> --report <report> --unsupported <unsupported.tsv> [--results <results>] [--implementation <name>]".to_owned()
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
    let (target, arguments) = optional(arguments, "--target")?;
    let (binary, arguments) = optional(&arguments, "--inbox")?;
    let [suite_path, results_path] = options(&arguments, ["--suite", "--results"])?;
    let mut target: Box<dyn Target> = match (target.as_deref(), binary) {
        (None | Some("core"), None) => Box::new(CoreTarget::new()),
        (None | Some("core"), Some(_)) => {
            return Err("`--inbox` names the binary of `--target serve`".to_owned());
        }
        (Some("serve"), Some(binary)) => {
            let work = std::path::Path::new(&results_path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("serve");
            Box::new(WireTarget::new(binary, work)?)
        }
        (Some("serve"), None) => {
            return Err(format!(
                "`--target serve` needs `--inbox <binary>`; {}",
                usage()
            ));
        }
        (Some(other), _) => {
            return Err(format!("unknown target `{other}`; {}", usage()));
        }
    };
    let text = read(&suite_path)?;
    let suite = suite::read(&text).map_err(|reason| format!("{suite_path}: {reason}"))?;
    let outcomes = run_suite(&suite, target.as_mut());
    drop(target);
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

/// Takes the optional `<option> <value>` out of `arguments`: its value, and the rest.
fn optional(arguments: &[String], option: &str) -> Result<(Option<String>, Vec<String>), String> {
    let mut rest = arguments.to_vec();
    let Some(at) = rest.iter().position(|name| name == option) else {
        return Ok((None, rest));
    };
    if at + 1 >= rest.len() {
        return Err(format!("`{option}` needs a value"));
    }
    let value = rest.remove(at + 1);
    rest.remove(at);
    if rest.iter().any(|other| other == option) {
        return Err(format!("`{option}` is given twice"));
    }
    Ok((Some(value), rest))
}

fn check(arguments: &[String]) -> Result<ExitCode, String> {
    let (implementation, arguments) = optional(arguments, "--implementation")?;
    let (results_path, arguments) = optional(&arguments, "--results")?;
    let [suite_path, report_path, listed_path] =
        options(&arguments, ["--suite", "--report", "--unsupported"])?;
    let suite = std::fs::read(&suite_path).map_err(|error| format!("{suite_path}: {error}"))?;
    let suite_text =
        std::str::from_utf8(&suite).map_err(|error| format!("{suite_path}: {error}"))?;
    let scenarios: Vec<String> = suite::read(suite_text)
        .map_err(|reason| format!("{suite_path}: {reason}"))?
        .scenarios
        .into_iter()
        .map(|scenario| scenario.id)
        .collect();
    let text = read(&report_path)?;
    let mut report =
        gate::read_report(&text).map_err(|reason| format!("{report_path}: {reason}"))?;
    let binding = gate::read_binding(&text).map_err(|reason| format!("{report_path}: {reason}"))?;
    let mut disagreements = Vec::new();
    if let Some(results_path) = &results_path {
        let run = gate::read_results(&read(results_path)?, &results::suite_digest(&suite))
            .map_err(|reason| format!("{results_path}: {reason}"))?;
        disagreements = gate::agrees(&binding, &run);
        report.reasons = run.reasons;
    }
    let listed = gate::read_listed(&read(&listed_path)?)
        .map_err(|reason| format!("{listed_path}: {reason}"))?;
    let mut verdict = gate::verdict(&report, &listed);
    verdict.problems.extend(disagreements);
    verdict.problems.extend(gate::bound(
        &report,
        &binding,
        &results::suite_digest(&suite),
    ));
    verdict
        .problems
        .extend(gate::accounts(&binding, &scenarios));
    if let Some(expected) = &implementation {
        verdict
            .problems
            .extend(gate::implemented(&binding, expected));
    }
    println!("  {}", verdict.line);
    println!("  conformance_status: {}", binding.conformance_status);
    for problem in &verdict.problems {
        println!("  {problem}");
    }
    Ok(if verdict.passes() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
