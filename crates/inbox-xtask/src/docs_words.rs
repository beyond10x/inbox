//! An end-user page names no wave, stage, story, epic or count of scenarios.
//!
//! The pages of the documentation site are written for a user of the tool. The plan and the
//! numbers of the gate belong in `docs/checks.md` and `docs/plan/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::Answer;

/// Case-insensitive; `\b` and `\d` are Unicode, as in the pattern this replaces.
static WORDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bwaves?\b|\bstages?\b|\bstory:|\bepic:|\bdraft stor(y|ies)\b|\badversary\b",
        r"|\b\d+ (authored |generated )?scenarios\b|\b\d+ of the \d+\b",
    ))
    .expect("the words pattern compiles")
});

/// The longest part of a matching line that is printed, in characters.
const SHOWN: usize = 120;

/// Each line of the files that names the plan or a count of the checks, as
/// `<path>:<line>: `<match>`: <the line, trimmed, at most 120 characters>`. A directory stands for
/// every `*.md` and `*.mdx` file below it. Exit 1 on a match, 2 when no file is to be read.
pub fn docs_words(arguments: &[PathBuf]) -> Result<Answer, String> {
    let mut paths = Vec::new();
    for argument in arguments {
        if argument.is_dir() {
            let mut found = Vec::new();
            pages_below(argument, &mut found)?;
            found.sort();
            paths.extend(found);
        } else {
            paths.push(argument.clone());
        }
    }
    if paths.is_empty() {
        return Ok(Answer {
            stderr: vec!["docs-words: no file to read".to_string()],
            code: 2,
            ..Answer::default()
        });
    }
    let mut stdout = Vec::new();
    for path in &paths {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("docs-words: read {}: {e}", path.display()))?;
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        for (number, line) in text.split('\n').enumerate() {
            if let Some(found) = WORDS.find(line) {
                let shown: String = line.trim().chars().take(SHOWN).collect();
                stdout.push(format!(
                    "{}:{}: `{}`: {shown}",
                    path.display(),
                    number + 1,
                    found.as_str()
                ));
            }
        }
    }
    if stdout.is_empty() {
        return Ok(Answer {
            stdout: vec![format!(
                "docs-words: {} file(s), none names the plan or a count of the checks",
                paths.len()
            )],
            ..Answer::default()
        });
    }
    let stderr = vec![format!(
        "docs-words: {} line(s) name the plan or a count of the checks; move them to docs/checks.md",
        stdout.len()
    )];
    Ok(Answer {
        stdout,
        stderr,
        code: 1,
    })
}

/// Every entry below `dir` whose extension is `md` or `mdx`, at any depth.
fn pages_below(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let listing =
        fs::read_dir(dir).map_err(|e| format!("docs-words: read {}: {e}", dir.display()))?;
    for entry in listing {
        let path = entry
            .map_err(|e| format!("docs-words: read {}: {e}", dir.display()))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "md" || extension == "mdx")
        {
            found.push(path.clone());
        }
        if path.is_dir() {
            pages_below(&path, found)?;
        }
    }
    Ok(())
}
