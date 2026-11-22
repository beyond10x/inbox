//! The authored scenarios of the specification (`ess/scenarios/*.yaml`, written by hand) and the
//! manifest that lists them (`ess/ess-inputs.yaml`, written from the files by `task scenarios`).
//!
//! Every file is checked before anything is written or compared, so a failed check changes no
//! file:
//!
//! - its `scenario:` is the file's name without `.yaml`;
//! - its `summary:` has at most 200 characters;
//! - every value at a key `message_id`, `reply_id`, `in_reply_to` or `thread` is a message id: 26
//!   characters of the Crockford alphabet;
//! - no generated scenario id is claimed by two `covers:` lists (one claimant per id);
//! - a claim of a type invariant at a view field is of a form this check reads, and rests on a row
//!   its scenario reads in that view (`assert`, `contains`) with a value at the field's path: every
//!   value there, through each element of a list, is present and not null.
//!
//! The manifest names the files of `ess/scenarios/`, each once, sorted, after a fixed head.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use yaml_rust2::{Yaml, YamlLoader};

use crate::Answer;

/// The directory of the scenario files, relative to the repository root.
pub const SCENARIOS: &str = "ess/scenarios";
/// The manifest, relative to the repository root.
pub const MANIFEST: &str = "ess/ess-inputs.yaml";
const FORMAT: &str = "ess-inputs/2";
const REQUIRES: &str = "ess 0.97.1";
const SPECIFICATION: [&str; 3] = ["system.yaml", "components.yaml", "domains/messaging.yaml"];
const DOMAIN: &str = "inbox.messaging.";
const SUMMARY_LIMIT: usize = 200;
const ID_KEYS: [&str; 4] = ["message_id", "reply_id", "in_reply_to", "thread"];
const ID_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ID_LENGTH: usize = 26;

/// A type's invariant read off a view field reached through an `Optional`, a struct or a list,
/// which synthesis names by the digest of its path: (type, field, digest) and the path a
/// `contains` row must hold a value at for the claim to rest on it. A path runs through every
/// element of a list. `reply` is claimed whole in a row: inbox-conformance does not read a dotted
/// row path such as `reply.body`.
const WRAPPED: [(&str, &str, &str, &[&str]); 7] = [
    (
        "Body",
        "reply",
        "6f225dcb81a1756258e5c80e589dcff62c29e6e19a7a5f6ccdce295dc91c5680",
        &["reply", "body"],
    ),
    (
        "MessageId",
        "reply",
        "2a59efac4c3ac205ca937cd02a066886cbd43648835cc56ca35246776dabd9bb",
        &["reply", "reply_id"],
    ),
    (
        "OptionLetter",
        "reply",
        "86a88b3f3da7e2830b7ed86ff27a235c8c2aa1b8b1ec906e031cf4f3f20680cc",
        &["reply", "option"],
    ),
    (
        "OptionLetter",
        "decided_option",
        "5a5de3e7ecffe323b37c180756920d594f1df48d3d00a5289072b4349e37fef4",
        &["decided_option"],
    ),
    (
        "OptionLetter",
        "default_on_silence",
        "5a5de3e7ecffe323b37c180756920d594f1df48d3d00a5289072b4349e37fef4",
        &["default_on_silence"],
    ),
    (
        "OptionLetter",
        "decision",
        "5fc40af03c03b2190846fd86e4dc7b3694f364228b490834f133211ec5ee2e25",
        &["decision", "options", "letter"],
    ),
    (
        "OptionLetter",
        "decision",
        "c9e38459e74269aef672d56a96584e08784681471855f791bed107fca2f65578",
        &["decision", "recommended_option"],
    ),
];

static INVARIANT_AT: LazyLock<Regex> = LazyLock::new(|| {
    let domain = regex::escape(DOMAIN);
    Regex::new(&format!(
        r"^{domain}(\w+)/invariant(-wrapped)?/at/{domain}(\w+)/(\w+)(?:/([0-9a-f]{{64}}))?$"
    ))
    .expect("the invariant pattern compiles")
});

/// One scenario file, read.
struct Scenario {
    name: String,
    covers: Vec<String>,
    asserts: Vec<Yaml>,
}

/// Check the scenario files and, with `check`, compare the manifest with them (one line per path
/// that differs); without, rewrite the manifest from them.
pub fn scenarios(root: &Path, check: bool) -> Result<Answer, String> {
    let dir = root.join(SCENARIOS);
    let mut names = Vec::new();
    let mut strays = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        match scenario_name(&file) {
            Some(name) if entry.path().is_file() => names.push(name.to_string()),
            _ => strays.push(file),
        }
    }
    names.sort();
    strays.sort();

    let mut failures = Vec::new();
    let mut read = Vec::new();
    for name in &names {
        if let Some(scenario) = read_scenario(root, name, &mut failures) {
            read.push(scenario);
        }
    }
    check_claims(&read, &mut failures);
    if !failures.is_empty() {
        let mut stderr: Vec<String> = failures.iter().map(|f| format!("scenarios: {f}")).collect();
        stderr.push(format!(
            "scenarios: {} check(s) failed; nothing written",
            failures.len()
        ));
        return Ok(Answer {
            stderr,
            code: 1,
            ..Answer::default()
        });
    }

    if check {
        let mut stdout = manifest_differences(root, &names, &strays);
        let differ = stdout.len();
        stdout.push(format!(
            "{} scenarios, {differ} path(s) differ from {MANIFEST}",
            names.len()
        ));
        return Ok(Answer {
            stdout,
            code: u8::from(differ > 0),
            ..Answer::default()
        });
    }
    if let Some(stray) = strays.first() {
        return Ok(Answer {
            stderr: vec![format!(
                "scenarios: {SCENARIOS}/{stray}: no scenario file (a name of a-z, 0-9 and -, ending in .yaml); nothing written"
            )],
            code: 1,
            ..Answer::default()
        });
    }
    let path = root.join(MANIFEST);
    fs::write(&path, render_manifest(&names))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(Answer {
        stdout: vec![format!("{} scenarios", names.len())],
        ..Answer::default()
    })
}

/// The scenario name of a file name: `<name>.yaml`, the name of a-z, 0-9 and `-`, not starting
/// with `-`.
fn scenario_name(file: &str) -> Option<&str> {
    let name = file.strip_suffix(".yaml")?;
    let plain = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (plain && !name.is_empty() && !name.starts_with('-')).then_some(name)
}

/// The manifest that lists `names`, as `task scenarios` writes it.
pub fn render_manifest(names: &[String]) -> String {
    let mut text = format!("format: {FORMAT}\nrequires: {REQUIRES}\nspecification:\n");
    for file in SPECIFICATION {
        text.push_str(&format!("- {file}\n"));
    }
    if names.is_empty() {
        text.push_str("scenarios: []\n");
    } else {
        text.push_str("scenarios:\n");
        for name in names {
            text.push_str(&format!("- scenarios/{name}.yaml\n"));
        }
    }
    text
}

/// One line per path that keeps the manifest from being the one `names` make: `missing:` for the
/// manifest itself or a file it lists that does not exist, `not in the manifest:` for an entry of
/// `ess/scenarios/` it does not list, and `edited:` for a manifest that is not in its form (the
/// head, each line a scenario file, sorted, each once). No line means it is the one `names` make.
fn manifest_differences(root: &Path, names: &[String], strays: &[String]) -> Vec<String> {
    let path = root.join(MANIFEST);
    let Ok(text) = fs::read(&path) else {
        return vec![if path.exists() {
            format!("edited: {MANIFEST}")
        } else {
            format!("missing: {MANIFEST}")
        }];
    };
    let listed = String::from_utf8(text.clone())
        .ok()
        .and_then(|text| listed_names(&text));
    let Some(listed) = listed else {
        return vec![format!("edited: {MANIFEST}")];
    };
    let mut lines = Vec::new();
    let mut canonical: Vec<String> = listed.iter().flatten().cloned().collect();
    canonical.sort();
    canonical.dedup();
    if listed.iter().any(Option::is_none) || render_manifest(&canonical).as_bytes() != text {
        lines.push(format!("edited: {MANIFEST}"));
    }
    for name in &canonical {
        if !names.contains(name) {
            lines.push(format!("missing: {SCENARIOS}/{name}.yaml"));
        }
    }
    for name in names {
        if !canonical.contains(name) {
            lines.push(format!("not in the manifest: {SCENARIOS}/{name}.yaml"));
        }
    }
    for stray in strays {
        lines.push(format!("not in the manifest: {SCENARIOS}/{stray}"));
    }
    lines
}

/// The entries of the manifest's `scenarios:` list: the scenario name of each
/// `scenarios/<name>.yaml`, `None` for an entry of another form. `None` for a manifest that is no
/// YAML mapping with such a list.
fn listed_names(text: &str) -> Option<Vec<Option<String>>> {
    let documents = YamlLoader::load_from_str(text).ok()?;
    let [document] = documents.as_slice() else {
        return None;
    };
    let list = document["scenarios"].as_vec()?;
    Some(
        list.iter()
            .map(|entry| {
                entry
                    .as_str()
                    .and_then(|s| s.strip_prefix("scenarios/"))
                    .and_then(scenario_name)
                    .map(str::to_string)
            })
            .collect(),
    )
}

fn read_scenario(root: &Path, name: &str, failures: &mut Vec<String>) -> Option<Scenario> {
    let file = format!("{SCENARIOS}/{name}.yaml");
    let path = root.join(&file);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            failures.push(format!("{file}: {e}"));
            return None;
        }
    };
    let documents = match YamlLoader::load_from_str(&text) {
        Ok(documents) => documents,
        Err(e) => {
            failures.push(format!("{file}: {e}"));
            return None;
        }
    };
    let [document] = documents.as_slice() else {
        failures.push(format!(
            "{file}: holds {} YAML documents, not one",
            documents.len()
        ));
        return None;
    };
    if document.as_hash().is_none() {
        failures.push(format!("{file}: is no YAML mapping"));
        return None;
    }
    match document["scenario"].as_str() {
        Some(scenario) if scenario == name => {}
        _ => failures.push(format!(
            "{file}: `scenario:` is `{}`, not the file's name `{name}`",
            describe(&document["scenario"])
        )),
    }
    match document["summary"].as_str() {
        Some(summary) if summary.chars().count() <= SUMMARY_LIMIT => {}
        Some(summary) => failures.push(format!(
            "{file}: the summary has {} characters, more than {SUMMARY_LIMIT}",
            summary.chars().count()
        )),
        None => failures.push(format!("{file}: has no `summary:` text")),
    }
    check_ids(&file, document, failures);
    let covers = match &document["covers"] {
        Yaml::BadValue => Vec::new(),
        Yaml::Array(items) if items.iter().all(|i| i.as_str().is_some()) => items
            .iter()
            .filter_map(|i| i.as_str().map(str::to_string))
            .collect(),
        _ => {
            failures.push(format!("{file}: `covers:` is no list of ids"));
            Vec::new()
        }
    };
    let asserts = document["assert"].as_vec().cloned().unwrap_or_default();
    Some(Scenario {
        name: name.to_string(),
        covers,
        asserts,
    })
}

/// Every value at a key of `ID_KEYS`, anywhere in the document, is a message id.
fn check_ids(file: &str, value: &Yaml, failures: &mut Vec<String>) {
    match value {
        Yaml::Hash(hash) => {
            for (key, value) in hash {
                if let Some(key) = key.as_str().filter(|k| ID_KEYS.contains(k)) {
                    let valid = value.as_str().is_some_and(|id| {
                        id.chars().count() == ID_LENGTH
                            && id.chars().all(|c| ID_ALPHABET.contains(c))
                    });
                    if !valid {
                        failures.push(format!(
                            "{file}: `{key}` is `{}`, not a message id ({ID_LENGTH} characters of 0-9 and A-Z without I, L, O, U)",
                            describe(value)
                        ));
                    }
                }
                check_ids(file, value, failures);
            }
        }
        Yaml::Array(items) => {
            for item in items {
                check_ids(file, item, failures);
            }
        }
        _ => {}
    }
}

fn describe(value: &Yaml) -> String {
    match value {
        Yaml::String(s) | Yaml::Real(s) => s.clone(),
        Yaml::Integer(i) => i.to_string(),
        Yaml::Boolean(b) => b.to_string(),
        Yaml::Null => "null".to_string(),
        Yaml::BadValue => "absent".to_string(),
        Yaml::Array(_) | Yaml::Hash(_) | Yaml::Alias(_) => "not a text".to_string(),
    }
}

/// One claimant per id, and each claim of a type invariant at a view field read and resting on a
/// row.
fn check_claims(scenarios: &[Scenario], failures: &mut Vec<String>) {
    let mut claimants: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for scenario in scenarios {
        for claim in &scenario.covers {
            claimants.entry(claim).or_default().push(&scenario.name);
        }
    }
    for (claim, names) in &claimants {
        if names.len() > 1 {
            failures.push(format!(
                "an id is claimed twice: {claim} ({})",
                names.join(", ")
            ));
        }
    }
    for scenario in scenarios {
        for claim in &scenario.covers {
            match reads(claim) {
                Ok(None) => {}
                Err(verdict) => {
                    failures.push(format!("{}: {claim}: {verdict}", scenario.name));
                }
                Ok(Some((view, path))) => {
                    let wanted = format!("{DOMAIN}{view}");
                    let rests = scenario
                        .asserts
                        .iter()
                        .filter(|a| a["view"].as_str() == Some(wanted.as_str()))
                        .filter(|a| !a["contains"].is_badvalue())
                        .any(|a| {
                            values_at(Some(&a["contains"]), &path)
                                .iter()
                                .all(|v| v.is_some_and(|v| !v.is_null()))
                        });
                    if !rests {
                        failures.push(format!(
                            "{} claims {claim} and reads no value at `{}` in a row of {view}",
                            scenario.name,
                            path.join(".")
                        ));
                    }
                }
            }
        }
    }
}

/// The view and the row path a claim of a type invariant at a view field rests on, read off the
/// id itself; `None` for an id of another form.
fn reads(claim: &str) -> Result<Option<(&str, Vec<&str>)>, &'static str> {
    if !claim.contains("/invariant") {
        return Ok(None);
    }
    let Some(found) = INVARIANT_AT.captures(claim) else {
        return Err("an invariant id of a shape this check does not read");
    };
    let part = |n: usize| found.get(n).map(|m| m.as_str());
    let (type_, view, field) = (
        part(1).unwrap_or(""),
        part(3).unwrap_or(""),
        part(4).unwrap_or(""),
    );
    let digest = part(5);
    if part(2).is_none() {
        return match digest {
            None => Ok(Some((view, vec![field]))),
            Some(_) => Err("an invariant id that is not wrapped carries a digest"),
        };
    }
    WRAPPED
        .iter()
        .find(|(t, f, d, _)| *t == type_ && *f == field && Some(*d) == digest)
        .map(|(_, _, _, path)| Some((view, path.to_vec())))
        .ok_or("a wrapped path this check does not know")
}

/// Every value a row holds at a path, through each element of a list; `None` where one is absent.
fn values_at<'a>(value: Option<&'a Yaml>, path: &[&str]) -> Vec<Option<&'a Yaml>> {
    let Some(value) = value.filter(|v| !v.is_badvalue()) else {
        return vec![None];
    };
    let Some((first, rest)) = path.split_first() else {
        return vec![Some(value)];
    };
    match value {
        Yaml::Array(items) if items.is_empty() => vec![None],
        Yaml::Array(items) => items
            .iter()
            .flat_map(|item| values_at(Some(item), path))
            .collect(),
        Yaml::Hash(hash) => values_at(hash.get(&Yaml::String((*first).to_string())), rest),
        _ => vec![None],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(text: &str) -> Yaml {
        YamlLoader::load_from_str(text).unwrap().remove(0)
    }

    fn present(text: &str, path: &[&str]) -> bool {
        values_at(Some(&yaml(text)), path)
            .iter()
            .all(|v| v.is_some_and(|v| !v.is_null()))
    }

    #[test]
    fn a_path_runs_through_every_element_of_a_list() {
        let path = ["decision", "options", "letter"];
        assert!(present(
            "decision: {options: [{letter: A}, {letter: B}]}",
            &path
        ));
        assert!(!present(
            "decision: {options: [{letter: A}, {text: x}]}",
            &path
        ));
        assert!(!present("decision: {options: []}", &path));
        assert!(!present("decision: {options: [{letter: null}]}", &path));
        assert!(!present("decision: 3", &path));
    }

    #[test]
    fn a_value_at_the_end_of_the_path_may_be_a_list_or_a_mapping() {
        assert!(present("reply: {body: x}", &["reply"]));
        assert!(present("options: [1]", &["options"]));
        assert!(!present("reply: ~", &["reply"]));
    }

    #[test]
    fn an_unwrapped_claim_reads_its_field() {
        assert_eq!(
            reads("inbox.messaging.Body/invariant/at/inbox.messaging.Settled/body"),
            Ok(Some(("Settled", vec!["body"])))
        );
        assert_eq!(reads("inbox.messaging.Answer/outcome/answered"), Ok(None));
    }

    #[test]
    fn the_manifest_of_no_scenario_is_still_yaml_with_a_list() {
        let text = render_manifest(&[]);
        assert_eq!(listed_names(&text), Some(Vec::new()));
    }
}
