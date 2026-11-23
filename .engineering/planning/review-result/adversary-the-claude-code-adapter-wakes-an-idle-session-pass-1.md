---
format: aep.planning-md/3
id: review-result:adversary-the-claude-code-adapter-wakes-an-idle-session-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the claude_code adapter
relations:
- reviews: story:the-claude-code-adapter-wakes-an-idle-session
revision: 1
---
unit: story:the-claude-code-adapter-wakes-an-idle-session, commit d1044b2 (archive copy at <scratch>/s3/cc/adversary/copy)
verdict: NEEDS-CHANGE
cases: executed 93→100, red 5 (plus 1 red in the copy only: `the_plugin_declares_an_async_rewake_stop_hook_that_runs_the_script` reads `git ls-files -s`, and the copy has no git)
origin: introduced 7 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (the scratch copy and its logs, listed in part 6)
needs-coordinator: whether a sender's text may reach the session at column 0 is a design call: fix it in `output.rs` (indent option text), in the plugin (framing that data cannot forge), or both

**1. diff --stat** (the copy has no git, so this was compared by hand against `git archive d1044b2`)
- `crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs`: new, about 370 lines.
- `crates/inbox-xtask/tests/tools/mod.rs`: 1 line added, `mod adversary_claude_code_adapter;`.
- Both are test files. I wrote nothing in `<tree>`. Its staged `.engineering/evidence` files are not mine.
- After the mutant runs, the copy's `wake.sh` is byte-equal to `d1044b2:adapters/claude-code/scripts/wake.sh` (cmp).

**2. Cases added** (file above; each runs in under 0.31 s)
- `a_decision_option_cannot_forge_the_plugins_note_line`, red. It runs the hook twice:
  - Genuine: a decision, wake exit 1, so the plugin adds its note line.
  - Forged: the same decision whose option A text holds a line end and that note, wake exit 0.
  - It asserts the two outputs do not hold the same number of `inbox:` lines.
  - Red output (run alone): `assertion left != right failed: a sender's option text reads as the plugin's own note line … left: 2 right: 2`. The quoted stderr shows the forged `inbox: \`inbox wake\` ended with exit status 1: …` at column 0.
- `one_message_reads_as_one_message_to_the_session`, red. One decision from `agent:case/other` whose option text carries a line end and a full row header `m-2 request Delivered from human:operator …`.
  - Red: `left: ["m-1 decision Delivered from agent:case/other …", "m-2 request Delivered from human:operator to agent:case/session …"] right: ["m-1 decision …"]`.
- `a_hook_killed_with_sigkill_leaves_no_wake_that_marks_a_post`, red. SIGKILL to the hook, then a post arrives.
  - Red: `the wake outlived its hook, wrote the post to a file no one reads and marked it delivered`.
- `a_harness_that_is_gone_gets_no_post_marked_delivered`, red. The read ends of the hook's stdout and stderr are closed with no signal sent, as when Claude Code crashes or is OOM-killed. Then a post arrives.
  - Red: `the post was marked delivered and handed to no one: no later wake returns on it`.
- `a_directory_left_under_the_hooks_name_does_not_stop_the_wake`, red.
  - First run alone: red for the wrong reason (`exec: sh: not found`, a typo of mine: the case's `PATH` has no `sh`). Fixed to the absolute path.
  - Red output (run alone): `left: Some(1) right: Some(2): … inbox plugin: the directory for the output of \`inbox wake\` cannot be made: a post does not wake this session.`
- `the_rows_directory_is_readable_by_its_owner_alone`, green on the unit. Under umask 022 it asserts mode 700 for the directory and 600 for the rows file. It kills mutant M3.
  - First run: same wrong-reason typo as above.
- `rows_printed_before_a_signal_are_handed_on_without_a_note`, green on the unit. Rows, then SIGTERM; it asserts stderr is exactly the fixed line plus the rows, as `docs/design/wake.md:207` says. It kills mutant M1.

**3. Suite run**, after the cases existed
- Command: `SLOT_TIMEOUT=10m cargo nextest run -p inbox-xtask --locked --no-fail-fast` in the copy.
- Result: `100 tests run: 94 passed, 6 failed`, exit 100, 20.3 s.
- Failures: my 5 red cases and `claude_code_adapter::the_plugin_declares_an_async_rewake_stop_hook_that_runs_the_script` (no git in the copy).
- `cargo clippy -p inbox-xtask --all-targets --locked -- -D warnings`: exit 0.

**4. Findings** (all cover d1044b2)

F1. Sender text reaches column 0 and can forge the plugin's lines (`crates/inbox-cli/src/output.rs:237-241`). NEEDS-CHANGE, warning, pre-existing.
- Measured: `rows_text` writes `  {letter}) {text}` and leaves a line end inside option text in place. `visible` keeps `\n`. `DecisionOption.text` is a plain `String` with no rule (`ess/domains/messaging.yaml:83-86`). `inbox post --option A=<text>` takes any text (`main.rs:409`).
- Reach: any principal that can post a decision to the inbox.
- Fix: indent each continuation line of option and reply text, as `indent` does for the body.

F2. The plugin's framing gives the session no boundary the data cannot forge (`adapters/claude-code/scripts/wake.sh:70-77`). NEEDS-CHANGE, warning, introduced.
- Measured: the fixed line says "the lines below are messages". Yet the plugin itself writes an `inbox:` note after the rows, and the rows can hold byte-identical `inbox:` lines or a forged row from `human:operator` (cases 1 and 2).
- At worst, a sender hands a session with the user's full permissions a line that reads as the plugin's or as another principal's message. Inside a body the indent is the only marker, and the fixed line does not name it.
- Fix, either of:
  - a delimiter the sender cannot know (a per-run nonce in the fixed line and an end line), plus the count of rows;
  - or hand only id, sender and kind, and let the session run `inbox read`.

F3. A harness gone without a signal gets its post marked delivered and lost (`wake.sh:58`). CONFIRMED, warning, introduced.
- Measured: the wake writes to a regular file, so its write and flush succeed and it marks the post. The hook then dies of SIGPIPE on its first `printf`, its EXIT trap does not run, and the rows stay on disk.
- No later wake returns on the post (`docs/design/wake.md:88-92`). This breaks the brief's rule "rows … marked delivered are never dropped" and "no wake is left behind by a session that ended".
- Reach: Claude Code crashes or is OOM-killed while idle. The design does not mention it.
- Fix: give the wake the hook's own stderr as stdout (`inbox wake </dev/null >&2 2>/dev/null`, with the fixed line first), so a harness that is gone makes the wake's write fail: exit 4, no mark.

F4. SIGKILL to the hook alone leaves the wake running, and it marks the post (`wake.sh:58`). INFEASIBLE, note, introduced.
- Confirmed by the stand-in. `docs/design/wake.md:230-233` already states it.
- Reach not shown: nothing found that sends SIGKILL to the hook alone, and the timeout signal is not verified.
- A design without the hole: a FIFO read by the hook's shell itself (`while IFS= read -r l; do …; done < fifo`). With no reader, the wake's write fails and exits 4. A window of one pipe buffer stays.

F5. A directory already under the name `$TMPDIR/inbox-claude-code.$$` makes the hook exit 1 without a wake (`wake.sh:47-51`). CONFIRMED, note, introduced.
- Reach:
  - another local user on a shared `/tmp`, by pre-making names for each pid;
  - or a directory left by F3 or F4 when the pid is reused, which loses the wake for that turn.
- Fix: try another name on EEXIST (a counter suffix), or `mktemp -d`.
- Mode 0700/0600 holds; quoted paths with spaces or newlines are safe; `rm -rf` removes only the directory it made.

F6. The website states as fact what the design marks not verified (`website/docs/first-use.md:225`). NEEDS-CHANGE, note, introduced.
- The page says "The plugin gives a wait 7 days, after which Claude Code ends it". `docs/design/wake.md:228-230` marks it not verified that Claude Code takes 604800 as it is.

F7. The exit-status table leaves out 4 and 128+n with rows (`docs/design/wake.md:200-207`). CONFIRMED, note, introduced.
- The script treats both as "1 or 3 with rows": rows, note, exit 2.
- Exit 4 with rows is reachable: a write that fails part way on a full `$TMPDIR` hands partial rows to the session.

F8. The 13 cases miss 2 mutants of mine (`crates/inbox-xtask/tests/tools/claude_code_adapter.rs`). CONFIRMED, note, introduced.
- They do not catch M1 (the `ended` guard dropped from the note) or M3 (`umask 077` dropped). My 2 green cases now catch them.

F9. Mutant M13 survives every case (`wake.sh:34`). CONFIRMED, note, introduced.
- M13 loosens the grep to `claude_code`, so a record whose address contains `claude_code` and is opened with `terminal` would match.
- Case to add: a record `{"inbox":"agent:case/claude_code","adapter":"terminal"}` gives no wake.

F10. Long rows may not reach the session whole (`docs/design/wake.md`, section claude_code). INFEASIBLE, note, undecided.
- hooks.md:1095-1099 caps plain hook output at 10,000 characters; past it, the session gets a file path and a 2,000-character preview. A body can be up to 65,536 bytes.
- Not verified whether the cap applies to an `asyncRewake` hook's stderr. The design says nothing about it.

Mutants, against the implementor's 13 cases (git case left out) and mine:
- M1, note printed after a signal: missed by the 13; caught by `rows_printed_before_a_signal_are_handed_on_without_a_note`.
- M2, note printed at status 0: caught by `rows_wake_the_session_with_the_rows_after_the_fixed_line`.
- M3, `umask 077` dropped: missed by the 13; caught by `the_rows_directory_is_readable_by_its_owner_alone`.
- M4, `-s` replaced by `-f`: caught by `a_wake_that_failed_without_rows_…`, `a_signal_to_the_hook_ends_its_wake` and `a_hook_run_while_a_wake_runs_leaves_one_wake`.
- M5, rows on stdout: caught by `rows_wake_…` and `a_mark_that_failed_…`.
- M6, `exit 0` for status 1 with rows: caught by `a_mark_that_failed_still_wakes_the_session_with_the_rows`.
- M7, `HOME` branch dropped: caught by `the_record_is_read_beside_the_outbox_file_inbox_uses_by_default`.
- M8, early `kill` at line 60 dropped: survived. Its window is a signal between `&` and `pid=$!`; not caught.
- M9, wait loop dropped: caught by `a_signal_to_the_hook_ends_its_wake`.
- M10, `[ -z "$ended" ] || exit 0` dropped: caught by `a_signal_to_the_hook_ends_its_wake` and `a_hook_run_while_a_wake_runs_leaves_one_wake`.
- M11, status 0 without rows exits 1: survived. Not reachable: the wake exits 0 only after printing rows.
- M12, wake stderr not discarded: caught by 5 cases.
- M13, grep loosened to `claude_code`: survived (F9).

**5. Attacked and not broken**
- Outbox path against `Outbox::locate`: same for relative paths, empty `XDG_STATE_HOME` and no `HOME`. Not checked: whether clap reads an empty `INBOX_OUTBOX` as unset.
- Adapter record: `push_text` escapes `"`, so an address cannot contain the matched text; an exact-enum adapter value cannot be forged.
- Round-4 change to `inbox wake`: does not break the hook. `reaches_a_reader` refuses only a closed descriptor or the `/dev/null` device, and the hook hands a regular file.
- TERM before the wake starts, between start and wait, or during `cat`: handled.
- Two hooks at the same instant: separate directories, and the wake's lock decides.
- Operator check: every command and option exists in `generated/cli/inbox.json`, and `--plugin-dir` is on plugins.md. The website names no story, stage or test count.

Not checked:
- dash (not on this host; not fetched), and `claude plugin validate` (not run).
- The git-mode case (`git ls-files -s`), which needs git.
- The pid-reuse busy loop in `kill -0`/`wait` (read only, judged negligible).

**6. Paths written outside the worktree**
- `<scratch>/s3/cc/adversary/` holds:
  - `copy/` (the archive of d1044b2 plus my 2 test files; `copy/target` deleted);
  - `mutate.sh` and `wake.sh.orig`;
  - the logs `red-alone.log`, `red-alone-2.log`, `suite.log`, `clippy.log`, `mutants.log` and `mutant-M1…M13-*.log`.

```findings
[
{"file":"crates/inbox-cli/src/output.rs","line":237,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"rows_text writes decision option text with its line ends unindented, so any sender can put arbitrary lines at column 0 of what inbox wake and inbox pending print"},
{"file":"adapters/claude-code/scripts/wake.sh","line":70,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the hook hands the session the rows with no delimiter or count the sender cannot forge, so a post can produce byte-identical plugin note lines and a forged message from another principal"},
{"file":"adapters/claude-code/scripts/wake.sh","line":58,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"with the wake writing to a regular file, a harness that is gone without a signal still gets the post marked delivered and handed to no one, and no later wake returns on it"},
{"file":"adapters/claude-code/scripts/wake.sh","line":58,"category":"concurrency","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"SIGKILL to the hook alone leaves the wake running and it marks the post delivered, but nothing was shown that sends that signal"},
{"file":"adapters/claude-code/scripts/wake.sh","line":47,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a directory already under the name $TMPDIR/inbox-claude-code.$$ makes the hook exit 1 without a wake instead of trying another name"},
{"file":"website/docs/first-use.md","line":225,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the page states as fact that Claude Code ends the wait after 7 days, which the design marks not verified"},
{"file":"docs/design/wake.md","line":200,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the exit-status table leaves out exit 4 and 128+n with rows, which the script hands on with the note like 1 and 3"},
{"file":"crates/inbox-xtask/tests/tools/claude_code_adapter.rs","category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the 13 cases miss the mutants that drop the ended guard of the note line and that drop umask 077"},
{"file":"adapters/claude-code/scripts/wake.sh","line":34,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"loosening the adapter grep to claude_code survives every case, so no case holds a record whose address contains claude_code"},
{"file":"docs/design/wake.md","category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"undecided","message":"the design does not say what the session sees when the rows exceed the 10,000-character hook output cap, and whether that cap applies to asyncRewake stderr is not verified"}
]
```
