//! `inbox-xtask docs-words`: an end-user page names no part of the plan and no count of the checks.

use std::fs;
use std::path::PathBuf;

use super::{tree, xtask};

const ARGS: [&str; 3] = ["docs-words", "website/docs", "website/product.json"];

fn pages(name: &str) -> PathBuf {
    tree(name, &["website/docs", "website/product.json"])
}

#[test]
fn the_committed_pages_name_nothing_of_the_plan() {
    let root = pages("docs-words-clean");
    let run = xtask(&root, &ARGS);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        ["docs-words: 8 file(s), none names the plan or a count of the checks"]
    );
}

#[test]
fn each_line_that_names_the_plan_is_printed_with_its_match() {
    let root = pages("docs-words-planted");
    let status = root.join("website/docs/status.md");
    let text = fs::read_to_string(&status).unwrap();
    assert!(text.ends_with('\n'));
    let first = text.lines().count() + 1;
    fs::write(
        &status,
        format!(
            "{text}This page names wave 3.\nAnd 38 authored scenarios.\nAnd 2 of the 5 limits.\n"
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("website/docs/sub")).unwrap();
    fs::write(
        root.join("website/docs/sub/page.mdx"),
        "Clean line.\r\nThe Epic: a draft Stories list.\n  The ADVERSARY found 12 generated scenarios.  \n",
    )
    .unwrap();
    fs::write(root.join("website/docs/notes.txt"), "wave\n").unwrap();
    let run = xtask(&root, &ARGS);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        [
            format!("website/docs/status.md:{first}: `wave`: This page names wave 3."),
            format!(
                "website/docs/status.md:{}: `38 authored scenarios`: And 38 authored scenarios.",
                first + 1
            ),
            format!(
                "website/docs/status.md:{}: `2 of the 5`: And 2 of the 5 limits.",
                first + 2
            ),
            "website/docs/sub/page.mdx:2: `Epic:`: The Epic: a draft Stories list.".to_string(),
            "website/docs/sub/page.mdx:3: `ADVERSARY`: The ADVERSARY found 12 generated scenarios."
                .to_string(),
        ]
    );
    assert_eq!(
        run.stderr.lines().collect::<Vec<_>>(),
        [
            "docs-words: 5 line(s) name the plan or a count of the checks; move them to docs/checks.md"
        ]
    );
}

#[test]
fn a_long_line_is_cut_at_120_characters() {
    let root = pages("docs-words-long");
    let line = format!("{} stage {}", "ü".repeat(60), "x".repeat(100));
    fs::write(root.join("website/docs/long.md"), format!("{line}\n")).unwrap();
    let run = xtask(&root, &ARGS);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    let cut: String = line.chars().take(120).collect();
    assert_eq!(
        run.stdout_lines(),
        [format!("website/docs/long.md:1: `stage`: {cut}")]
    );
}

#[test]
fn no_file_to_read_is_a_usage_error() {
    let root = pages("docs-words-none");
    fs::create_dir_all(root.join("website/empty")).unwrap();
    let run = xtask(&root, &["docs-words", "website/empty"]);
    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stderr.lines().collect::<Vec<_>>(),
        ["docs-words: no file to read"]
    );
}
