//! What a command prints: one JSON document with `--json`, text for a person without.

use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::Write as _;

use inbox::server::json::{self, Value};

/// What a finished command prints.
#[derive(Debug)]
pub struct Done {
    /// The JSON document, printed with `--json`.
    pub document: String,
    /// The text, printed without it.
    pub text: String,
}

/// Writes one line to standard error, each control character but the line end in its visible
/// form ([`visible`]): the line may repeat text of the service or of another principal. A failed
/// write (a reader that is gone) is ignored, never a panic, as `eprintln!` would.
pub fn warn(line: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{}", visible(line));
}

/// Writes text for a person to standard output, each control character but the line end in its
/// visible form ([`visible`]). A failed write (a reader that is gone) is not a panic: the exit
/// status still says what happened.
pub fn print_text(text: &str) {
    print(&visible(text));
}

/// Writes a JSON document to standard output with no raw control character in it
/// ([`plain_document`]).
pub fn print_document(document: &str) {
    print(&plain_document(document));
}

fn print(line: &str) {
    let _ = print_checked(line);
}

/// [`print_text`], answering whether the text was written: `inbox wake` sends no `MarkDelivered`
/// after a write that failed. It writes through [`reader`], so a write the kernel refuses (a
/// reader that is gone, a descriptor 1 open only for reading) is an error; the bytes are those
/// [`print_text`] writes.
///
/// # Errors
///
/// The error of [`reader`] or of the write.
pub fn try_print_text(text: &str) -> std::io::Result<()> {
    write_to_reader(&visible(text), false)
}

/// The terminal's bell, the client's own byte: [`try_print_round`] writes it after the rows of a
/// round of `inbox wake --follow`, never through [`visible`], which shows a BEL of another
/// principal as its escape `\u{7}`.
const BEL: char = '\u{7}';

/// [`try_print_text`] for a round of `inbox wake --follow`: the same bytes, then one [`BEL`] when
/// standard output is a terminal, and none when it is not (a pipe or a file gets the text only).
/// One write, so the bell comes after the text.
///
/// # Errors
///
/// The error of [`reader`] or of the write.
pub fn try_print_round(text: &str) -> std::io::Result<()> {
    write_to_reader(&visible(text), true)
}

/// [`print_document`], answering whether the document was written, as [`try_print_text`] does.
///
/// # Errors
///
/// The error of [`reader`] or of the write.
pub fn try_print_document(document: &str) -> std::io::Result<()> {
    write_to_reader(&plain_document(document), false)
}

/// Whether standard output can reach a reader ([`reader`]); `inbox wake` asks it before its lock
/// and its first request, and again before it writes its rows.
///
/// # Errors
///
/// The error of [`reader`].
pub fn reaches_a_reader() -> std::io::Result<()> {
    reader().map(drop)
}

/// A file made from a duplicate of descriptor 1, when it can reach a reader: descriptor 1 is open
/// and is not `/dev/null`. A process started with descriptor 1 closed (`inbox wake >&-`) finds
/// `/dev/null` there, which the standard library's start of the program opens in place of a
/// closed standard descriptor; a write to it succeeds and reaches no one. A write through the
/// file goes to the kernel unbuffered, and its error is not swallowed as the standard library's
/// standard output swallows `EBADF`.
///
/// # Errors
///
/// An error that says so when descriptor 1 is closed or is `/dev/null`.
fn reader() -> std::io::Result<std::fs::File> {
    use std::os::fd::AsFd;
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let nobody = || std::io::Error::other("it is closed or /dev/null, which no one reads");
    let held = std::io::stdout()
        .as_fd()
        .try_clone_to_owned()
        .map_err(|_| nobody())?;
    let file = std::fs::File::from(held);
    let found = file.metadata().map_err(|_| nobody())?;
    let null = std::fs::metadata("/dev/null").ok();
    if null.is_some_and(|null| found.file_type().is_char_device() && found.rdev() == null.rdev()) {
        return Err(nobody());
    }
    Ok(file)
}

/// Writes `line` and a line end through [`reader`], in one `write_all`; with `bell`, then a
/// [`BEL`] when that is a terminal ([`rings`]).
fn write_to_reader(line: &str, bell: bool) -> std::io::Result<()> {
    let mut file = reader()?;
    let mut bytes = String::with_capacity(line.len() + 2);
    bytes.push_str(line);
    bytes.push('\n');
    if bell && rings(&file) {
        bytes.push(BEL);
    }
    file.write_all(bytes.as_bytes())?;
    file.flush()
}

/// Whether the bell is written to `file`: only when it is a terminal.
fn rings(file: &std::fs::File) -> bool {
    use std::io::IsTerminal as _;
    file.is_terminal()
}

fn print_checked(line: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}")?;
    stdout.flush()
}

/// `text` with each control character but the line end (C0 with the tab and the carriage
/// return, DEL, C1), and the Unicode line and paragraph separators (U+2028, U+2029, which a
/// reader may show as a line end), and each bidirectional control ([`bidirectional`]), written
/// as its escape `\u{1b}`: a value another principal or the service set reaches the terminal as
/// visible text, never as a control sequence, a line of its own or text shown in another order.
/// Text without such characters is unchanged.
pub fn visible(text: &str) -> Cow<'_, str> {
    escaped(text, |character| {
        (character.is_control() && character != '\n')
            || matches!(character, '\u{2028}' | '\u{2029}')
            || bidirectional(character)
    })
}

/// The bidirectional controls of Unicode, which make a terminal show the text after them in
/// another order: the Arabic letter mark (U+061C), the left-to-right and right-to-left marks
/// (U+200E, U+200F), the embeddings, overrides and their end (U+202A to U+202E), and the
/// isolates and their end (U+2066 to U+2069).
fn bidirectional(character: char) -> bool {
    matches!(
        character,
        '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

/// `document` with each control character a JSON writer may leave raw inside a string (DEL and
/// C1; a C0 character in a string is escaped already) written as its `\u` escape: a JSON reader
/// gets the same value, and no control sequence reaches a terminal. JSON's white space outside a
/// string (line end, carriage return, tab) is kept.
pub fn plain_document(document: &str) -> Cow<'_, str> {
    if !document.chars().any(raw_in_document) {
        return Cow::Borrowed(document);
    }
    let mut out = String::with_capacity(document.len() + 8);
    for character in document.chars() {
        if raw_in_document(character) {
            let _ = write!(out, "\\u{:04x}", u32::from(character));
        } else {
            out.push(character);
        }
    }
    Cow::Owned(out)
}

fn raw_in_document(character: char) -> bool {
    character.is_control() && !matches!(character, '\n' | '\r' | '\t')
}

fn escaped(text: &str, escape: impl Fn(char) -> bool) -> Cow<'_, str> {
    if !text.chars().any(&escape) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for character in text.chars() {
        if escape(character) {
            out.extend(character.escape_unicode());
        } else {
            out.push(character);
        }
    }
    Cow::Owned(out)
}

/// Appends `value` as JSON. Numbers keep the spelling they arrived in.
pub fn write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => json::push_bool(out, *flag),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => json::push_text(out, text),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (name, member) in members {
                json::member(out, name);
                write(member, out);
            }
            out.push('}');
        }
    }
}

/// `value` as a JSON document.
pub fn document(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

/// `document`, a JSON object this client wrote, with the member `name` set to the JSON
/// document `member` after its own members. A document that does not read as an object, or a
/// member that does not read as JSON, leaves it as it is.
pub fn with_member<'a>(document: &'a str, name: &str, member: &str) -> Cow<'a, str> {
    match (json::parse(document), json::parse(member)) {
        (Ok(Value::Object(mut members)), Ok(member)) => {
            members.retain(|(held, _)| held != name);
            members.push((name.to_owned(), member));
            Cow::Owned(self::document(&Value::Object(members)))
        }
        _ => Cow::Borrowed(document),
    }
}

/// The text member `name` of `value`, or an empty text.
pub fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    match value.member(name) {
        Some(Value::Text(text)) => text,
        _ => "",
    }
}

/// The rows of a view answer.
pub fn rows(value: &Value) -> &[Value] {
    match value.member("rows") {
        Some(Value::Array(rows)) => rows,
        _ => &[],
    }
}

/// The rows of a view answer, for a person: a head line per message, its body indented under
/// it, then what the message carries beyond the body (the options of a decision, its reply, its
/// refusal). A message has one line that is not indented, its head line, whatever its sender
/// wrote: a member printed inside a line shows a line end as `\u{a}` ([`inline`]), and the lines
/// of a text after its first are indented by 4 spaces ([`indent`], [`first_and_rest`]).
pub fn rows_text(answer: &Value) -> String {
    let rows = rows(answer);
    if rows.is_empty() {
        return "no messages".to_owned();
    }
    let mut out = String::new();
    for row in rows {
        let field = |name: &str| inline(text(row, name));
        let _ = write!(
            out,
            "{} {} {} from {} to {} thread {} expires {}",
            field("message_id"),
            field("kind"),
            field("state"),
            field("from"),
            field("to"),
            field("thread"),
            field("expires"),
        );
        if text(row, "on_behalf_of") != text(row, "from") {
            let _ = write!(out, " on behalf of {}", field("on_behalf_of"));
        }
        out.push('\n');
        indent(&mut out, text(row, "body"));
        if let Some(Value::Object(_)) = row.member("decision") {
            let decision = row.member("decision").unwrap_or(&Value::Null);
            if let Some(Value::Array(options)) = decision.member("options") {
                for option in options {
                    first_and_rest(
                        &mut out,
                        &format!("  {}) ", inline(text(option, "letter"))),
                        text(option, "text"),
                    );
                }
            }
        }
        settlement(&mut out, row);
    }
    out.trim_end().to_owned()
}

/// What a message carries once it is settled: its reply, the option silence took, its refusal.
fn settlement(out: &mut String, row: &Value) {
    if let Some(reply) = row.member("reply") {
        let option = text(reply, "option");
        let _ = writeln!(
            out,
            "  reply {} from {}{}:",
            inline(text(reply, "reply_id")),
            inline(text(reply, "from")),
            if option.is_empty() {
                String::new()
            } else {
                format!(", option {}", inline(option))
            }
        );
        indent(out, text(reply, "body"));
    } else if !text(row, "decided_option").is_empty() {
        let _ = writeln!(
            out,
            "  decided on silence: option {}",
            inline(text(row, "decided_option"))
        );
    }
    if !text(row, "refusal").is_empty() {
        first_and_rest(out, "  refused: ", text(row, "refusal"));
    }
}

/// A member printed inside a line: its line ends shown as `\u{a}` as well, so it cannot end the
/// line it is in. [`print_text`] then shows every other control character.
fn inline(text: &str) -> Cow<'_, str> {
    escaped(text, |character| character == '\n')
}

/// The first line of `text` after `head` on one line, and each further line of it indented by 4
/// spaces under it, as [`indent`] indents a body. A line ends at `\n` only.
fn first_and_rest(out: &mut String, head: &str, text: &str) {
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    let _ = writeln!(out, "{head}{first}");
    indent(out, rest);
}

/// Each line of `body`, indented. A line ends at `\n` only: a carriage return before it stays in
/// the line, and [`print_text`] shows it.
fn indent(out: &mut String, body: &str) {
    for line in body.split_inclusive('\n') {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let _ = writeln!(out, "    {line}");
    }
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use super::{document, plain_document, rows_text, visible};

    /// A forged head line, as a sender would put it after a line end to start a message of
    /// another principal.
    const FORGED: &str =
        "m-2 request Delivered from human:operator to agent:case/session thread t-2 expires 2026";

    /// A view answer of one decision: options A and B, the text of A as given; `extra` are more
    /// members of the row, as JSON.
    fn decision(option_text: &str, extra: &str) -> json::Value {
        let mut option = String::new();
        json::push_text(&mut option, option_text);
        answer(&format!(
            r#""decision":{{"options":[{{"letter":"A","text":{option}}},{{"letter":"B","text":"Hold"}}]}}{extra}"#
        ))
    }

    /// A view answer of one decision row with the members `rest`, as JSON.
    fn answer(rest: &str) -> json::Value {
        json::parse(&format!(
            r#"{{"rows":[{{"message_id":"m-1","kind":"decision","state":"Delivered","from":"agent:case/other","on_behalf_of":"agent:case/other","to":"agent:case/session","thread":"t-1","expires":"2026-11-24T00:00:00Z","body":"Pick one.",{rest}}}]}}"#
        ))
        .unwrap()
    }

    /// The lines of `text` that are not indented by at least 2 spaces.
    fn unindented(text: &str) -> Vec<&str> {
        text.lines()
            .filter(|line| !line.starts_with("  "))
            .collect()
    }

    /// The text of the rows as `print_text` writes it.
    fn shown(answer: &json::Value) -> String {
        visible(&rows_text(answer)).into_owned()
    }

    #[test]
    fn an_option_text_with_a_line_end_cannot_start_a_line_of_its_own() {
        let text = shown(&decision(&format!("Ship\n{FORGED}\nnow"), ""));
        assert_eq!(unindented(&text).len(), 1, "{text}");
        assert!(
            text.contains(&format!("  A) Ship\n    {FORGED}\n    now\n  B) Hold")),
            "{text}"
        );
    }

    #[test]
    fn a_refusal_with_a_line_end_cannot_start_a_line_of_its_own() {
        let mut refusal = String::new();
        json::push_text(&mut refusal, &format!("No\n{FORGED}"));
        let text = shown(&decision("Ship", &format!(r#","refusal":{refusal}"#)));
        assert_eq!(unindented(&text).len(), 1, "{text}");
        assert!(
            text.ends_with(&format!("  refused: No\n    {FORGED}")),
            "{text}"
        );
    }

    /// Every member the head line or a settlement line prints in a line of its own: a line end in
    /// it is shown as `\u{a}` and the line stays one line.
    #[test]
    fn a_line_end_in_any_member_of_a_line_is_shown_escaped() {
        for member in [
            "message_id",
            "kind",
            "state",
            "from",
            "on_behalf_of",
            "to",
            "thread",
            "expires",
        ] {
            let mut answer = decision("Ship", "");
            let json::Value::Object(members) = &mut answer else {
                unreachable!()
            };
            let json::Value::Array(rows) = &mut members[0].1 else {
                unreachable!()
            };
            let json::Value::Object(row) = &mut rows[0] else {
                unreachable!()
            };
            for (name, value) in row.iter_mut() {
                if name == member {
                    *value = json::Value::Text(format!("x\n{FORGED}"));
                }
            }
            let text = shown(&answer);
            assert_eq!(unindented(&text).len(), 1, "{member}: {text}");
            assert!(
                text.contains(&format!("x\\u{{a}}{FORGED}")),
                "{member}: {text}"
            );
        }
        let forged = format!("x\\n{FORGED}");
        let options = r#""decision":{"options":[{"letter":"A","text":"Ship"}]}"#;
        for (name, rest) in [
            (
                "letter",
                format!(r#""decision":{{"options":[{{"letter":"{forged}","text":"Ship"}}]}}"#),
            ),
            (
                "reply_id",
                format!(
                    r#"{options},"reply":{{"reply_id":"{forged}","from":"agent:case/session","body":"Ship it."}}"#
                ),
            ),
            (
                "reply from",
                format!(
                    r#"{options},"reply":{{"reply_id":"r-1","from":"{forged}","body":"Ship it."}}"#
                ),
            ),
            (
                "reply option",
                format!(
                    r#"{options},"reply":{{"reply_id":"r-1","from":"agent:case/session","option":"{forged}","body":"Ship it."}}"#
                ),
            ),
            (
                "decided_option",
                format!(r#"{options},"decided_option":"{forged}""#),
            ),
        ] {
            let text = shown(&answer(&rest));
            assert_eq!(unindented(&text).len(), 1, "{name}: {text}");
            assert!(
                text.contains(&format!("x\\u{{a}}{FORGED}")),
                "{name}: {text}"
            );
        }
    }

    #[test]
    fn a_unicode_line_or_paragraph_separator_is_shown_escaped() {
        assert_eq!(visible("a\u{2028}b\u{2029}c"), "a\\u{2028}b\\u{2029}c");
        let text = shown(&decision(&format!("Ship\u{2028}{FORGED}"), ""));
        assert!(
            text.contains(&format!("  A) Ship\\u{{2028}}{FORGED}")),
            "{text}"
        );
    }

    /// Every bidirectional control (the Arabic letter mark, the left-to-right and right-to-left
    /// marks, the embeddings, overrides and their end, the isolates and their end) is shown as
    /// its escape: a sender cannot reorder what the owner's terminal shows.
    #[test]
    fn every_bidirectional_control_is_shown_escaped() {
        let controls = [
            '\u{61c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
            '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
        ];
        for control in controls {
            let text = format!("pay {control}0001 now");
            assert_eq!(
                visible(&text),
                format!("pay \\u{{{:x}}}0001 now", u32::from(control)),
                "U+{:04X}",
                u32::from(control)
            );
        }
        let text = shown(&decision("Ship \u{202e}dloH", ""));
        assert!(text.contains("  A) Ship \\u{202e}dloH"), "{text}");
        assert!(
            !text.chars().any(|character| controls.contains(&character)),
            "{text}"
        );
    }

    /// The neighbours of the bidirectional controls are not escaped: the zero width non-joiner
    /// and joiner, which a script needs, the letters of a right-to-left script, U+202F and
    /// U+2065.
    #[test]
    fn a_character_beside_the_bidirectional_controls_is_printed_as_written() {
        let plain = "a\u{200c}b\u{200d}c \u{5e9}\u{5dc}\u{5d5}\u{5dd} \u{645}\u{631}\u{62d}\u{628}\u{627} \u{202f}\u{2065}";
        assert!(matches!(visible(plain), std::borrow::Cow::Borrowed(text) if text == plain));
    }

    /// `--json` is unchanged: a bidirectional control stays a value of the document as it was
    /// sent.
    #[test]
    fn a_document_keeps_a_bidirectional_control_as_it_was_sent() {
        let value = json::Value::Object(vec![(
            "body".to_owned(),
            json::Value::Text("pay \u{202e}0001".to_owned()),
        )]);
        let written = document(&value);
        assert_eq!(plain_document(&written), written);
        assert_eq!(json::parse(&written).unwrap(), value);
    }

    #[test]
    fn a_document_is_written_as_it_was_read() {
        let text = r#"{"rows":[{"a":"x\"y\n","n":1.50,"b":true,"c":null,"d":[]}],"e":{}}"#;
        assert_eq!(document(&json::parse(text).unwrap()), text);
    }

    #[test]
    fn every_control_character_but_the_line_end_is_shown_escaped() {
        assert_eq!(
            visible("a\u{1b}[2J\u{7}\r\n\t\u{7f}\u{9b}b"),
            "a\\u{1b}[2J\\u{7}\\u{d}\n\\u{9}\\u{7f}\\u{9b}b"
        );
        let plain = "Grüße, 世界\n\\u{1b} stays\n";
        assert!(matches!(visible(plain), std::borrow::Cow::Borrowed(text) if text == plain));
    }

    #[test]
    fn a_document_holds_no_raw_control_character_and_reads_as_the_same_value() {
        let value = json::Value::Object(vec![(
            "body".to_owned(),
            json::Value::Text("a\u{1b}\u{7f}\u{80}\u{9b}\u{9f}\u{a0}\r\n".to_owned()),
        )]);
        let written = plain_document(&document(&value)).into_owned();
        assert!(!written.chars().any(char::is_control), "{written:?}");
        assert_eq!(json::parse(&written).unwrap(), value);
    }

    /// Every write to standard output or standard error of the client goes through `print_text`,
    /// `print_document` or `warn` of this file, which hold it to the rule of [`visible`]. A file
    /// of `src/` that writes there itself fails here, naming the file and what it wrote with.
    #[test]
    fn only_this_file_writes_to_standard_output_or_standard_error() {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut wrong = Vec::new();
        for entry in std::fs::read_dir(&source).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|name| name == "output.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for writer in [
                "stdout(",
                "stderr(",
                "print!(",
                "println!(",
                "eprint!(",
                "eprintln!(",
                "dbg!(",
            ] {
                if text.contains(writer) {
                    wrong.push(format!("{}: {writer}", path.display()));
                }
            }
        }
        assert!(wrong.is_empty(), "{wrong:?}");
    }
}
