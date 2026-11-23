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
    write_to_reader(&visible(text))
}

/// [`print_document`], answering whether the document was written, as [`try_print_text`] does.
///
/// # Errors
///
/// The error of [`reader`] or of the write.
pub fn try_print_document(document: &str) -> std::io::Result<()> {
    write_to_reader(&plain_document(document))
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

/// Writes `line` and a line end through [`reader`], in one `write_all`.
fn write_to_reader(line: &str) -> std::io::Result<()> {
    let mut file = reader()?;
    let mut bytes = String::with_capacity(line.len() + 1);
    bytes.push_str(line);
    bytes.push('\n');
    file.write_all(bytes.as_bytes())?;
    file.flush()
}

fn print_checked(line: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}")?;
    stdout.flush()
}

/// `text` with each control character but the line end (C0 with the tab and the carriage
/// return, DEL, C1) written as its escape `\u{1b}`: a value another principal or the service set
/// reaches the terminal as visible text, never as a control sequence. Text without control
/// characters is unchanged.
pub fn visible(text: &str) -> Cow<'_, str> {
    escaped(text, |character| {
        character.is_control() && character != '\n'
    })
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
/// refusal).
pub fn rows_text(answer: &Value) -> String {
    let rows = rows(answer);
    if rows.is_empty() {
        return "no messages".to_owned();
    }
    let mut out = String::new();
    for row in rows {
        let _ = write!(
            out,
            "{} {} {} from {} to {} thread {} expires {}",
            text(row, "message_id"),
            text(row, "kind"),
            text(row, "state"),
            text(row, "from"),
            text(row, "to"),
            text(row, "thread"),
            text(row, "expires"),
        );
        if text(row, "on_behalf_of") != text(row, "from") {
            let _ = write!(out, " on behalf of {}", text(row, "on_behalf_of"));
        }
        out.push('\n');
        indent(&mut out, text(row, "body"));
        if let Some(Value::Object(_)) = row.member("decision") {
            let decision = row.member("decision").unwrap_or(&Value::Null);
            if let Some(Value::Array(options)) = decision.member("options") {
                for option in options {
                    let _ = writeln!(
                        out,
                        "  {}) {}",
                        text(option, "letter"),
                        text(option, "text")
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
            text(reply, "reply_id"),
            text(reply, "from"),
            if option.is_empty() {
                String::new()
            } else {
                format!(", option {option}")
            }
        );
        indent(out, text(reply, "body"));
    } else if !text(row, "decided_option").is_empty() {
        let _ = writeln!(
            out,
            "  decided on silence: option {}",
            text(row, "decided_option")
        );
    }
    if !text(row, "refusal").is_empty() {
        let _ = writeln!(out, "  refused: {}", text(row, "refusal"));
    }
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

    use super::{document, plain_document, visible};

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
