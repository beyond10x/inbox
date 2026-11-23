# Releasing the inbox

A release is a bare version tag (`0.5.0`), annotated and written by the organization bot, on a
`main` commit whose `task check` and 3 pushed checks are green, and a GitHub Release made from
that tag. The tag is the workspace version, `[workspace.package] version` in `Cargo.toml`; the
release workflow fails on a tag that differs.

## What a release holds

No binary and no crate. Every crate of the workspace is `publish = false`, and the release
workflow builds and attaches nothing: the binary `inbox` (`crates/inbox-cli`) is built from
source. A release is the tag, its
`CHANGELOG.md` section, and a GitHub Release whose notes say what works and which limits are
open.

[`.github/workflows/release.yml`](../.github/workflows/release.yml) has one job, `Version`. On a
tag it fails unless the tag equals the workspace version, the tag is annotated, and
`CHANGELOG.md` has a section `## [<version>]`. On a manual run it checks the version and the
changelog section only. It builds nothing, publishes nothing and writes nothing to the
repository. The release is created by the organization bot, `b10x-bot[bot]`; no step uses
`github.token` or a personal account to write a release.

## Cutting a release

The commands use the gates as the bot, from a checkout of this repository:

    gates="b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/inbox"

1. **Version.** Set `version` in `[workspace.package]` of `Cargo.toml` to the new version, then
   `cargo metadata --format-version 1 > /dev/null` so `Cargo.lock` carries it. The first
   release is the version already there, `0.1.0`.
2. **Changelog.** In `CHANGELOG.md`, rename `## [Unreleased]` to `## [<version>] - <date>` and
   put an empty `## [Unreleased]` above it. Commit both and bring the commit to `main`
   (`AGENTS.md`, "Publishing"); `slot task check` is green on that commit, and so are the 3
   checks that run on it (`common / Security and privacy`, `inbox / task check`,
   `Build documentation`).
3. **Try it.** Run the workflow by hand on `main` and wait for it:

       $gates gh -- workflow run release.yml --ref main
       $gates gh -- run list --workflow release.yml --limit 1
       $gates gh -- run watch <run-id> --exit-status

4. **Tag.** Write the tag message (the version's changelog section) to a file, then:

       $gates bot -- tag -a <version> -F <message-file> <commit>
       $gates bot -- push origin <version>

5. **Check.** The push starts the workflow on the tag. Wait for that run to be green:

       $gates gh -- run list --workflow release.yml --branch <version> --limit 1
       $gates gh -- run watch <run-id> --exit-status

6. **Release.** Write the notes to a file in the short form: a lead of 1 sentence (at most 25
   words), `**Breaking**` (`- None.` when none), `**Changed**` with at most 5 bullets of one
   clause each, one of them naming what stays on the limits page, and a last line that links
   to the version's `CHANGELOG.md` section; at most 900 characters without that line. The
   annotated tag carries the whole section. Scan the file, then create the release as the bot
   and view it:

       $gates scan-text <notes-file>
       $gates gh -- release create <version> --verify-tag --title "inbox <version>" \
         --notes-file <notes-file>
       $gates gh -- release view <version> --json author,tagName,isDraft

   The release is done when it is owned by `b10x-bot[bot]`, names the tag `<version>`, and is
   not a draft. A green tag run with no release is queued, not released.

A refused scan creates no release: fix the cause on `main` and release the next patch version.
