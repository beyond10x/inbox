//! `.cargo/config.toml`, the build settings every `cargo` call in the tree reads, locally and in
//! CI. The test step of the gate has a bound of 120 s on its run: with the dependencies built
//! unoptimized, the cases that sign, verify, hash and parse took 108 s for 633 cases at a load
//! average of 134 to 146 on 20 CPUs, and 39 s with the dependencies at `opt-level = 2`. The crates
//! of the workspace stay unoptimized, so a change of one of them builds as fast as before.

use std::collections::BTreeMap;
use std::fs;

use super::repository;

/// The keys of each table of the file, as written: `table -> key -> value`.
fn tables() -> BTreeMap<String, BTreeMap<String, String>> {
    let text = fs::read_to_string(repository().join(".cargo/config.toml")).unwrap();
    let mut tables: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut table = String::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            table = name.to_owned();
            tables.entry(table.clone()).or_default();
        } else {
            let (key, value) = line.split_once('=').expect("a key and a value");
            tables
                .entry(table.clone())
                .or_default()
                .insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    tables
}

#[test]
fn test_builds_optimize_the_dependencies_and_not_the_workspace() {
    let tables = tables();
    assert_eq!(
        tables.get("profile.dev.package.\"*\""),
        Some(&BTreeMap::from([("opt-level".to_owned(), "2".to_owned())])),
        "every dependency is built at opt-level 2 in the dev profile, which test builds use"
    );
    assert_eq!(
        tables.get("profile.dev"),
        Some(&BTreeMap::from([("debug".to_owned(), "false".to_owned())])),
        "the crates of the workspace keep the profile's opt-level, and no build carries debug \
         information"
    );
    assert_eq!(
        tables.len(),
        2,
        "no other table: a setting of the wrapper, the jobs or another profile is the host's"
    );
}
