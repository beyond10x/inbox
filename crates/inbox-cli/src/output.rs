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

/// Whether standard output can reach a reader ([`reader`]), and, when it is a pipe or a FIFO,
/// whether that still has a reader ([`reader_gone`]); `inbox wake` asks it before its lock and
/// its first request, and again after it wrote the rows of a round and before their first mark.
///
/// # Errors
///
/// The error of [`reader`], or [`NO_READER`] for a pipe or a FIFO without a reader.
pub fn reaches_a_reader() -> std::io::Result<()> {
    let file = reader()?;
    if is_a_pipe(&file) && reader_gone(&file) {
        return Err(std::io::Error::other(NO_READER));
    }
    Ok(())
}

/// What a pipe or a FIFO is whose reading end is closed.
pub const NO_READER: &str = "it is a pipe or a FIFO whose reader is gone";

/// A duplicate of descriptor 1 when it is a pipe or a FIFO that can reach a reader ([`reader`]):
/// the one kind of standard output whose reader `inbox wake` watches ([`reader_gone`]). `None`
/// for a terminal, a regular file, `/dev/null`, a socket, or a closed descriptor.
pub fn pipe() -> Option<std::fs::File> {
    reader().ok().filter(is_a_pipe)
}

/// Whether `file` is a pipe or a FIFO (both are `S_IFIFO`).
fn is_a_pipe(file: &std::fs::File) -> bool {
    use std::os::unix::fs::FileTypeExt;
    file.metadata()
        .is_ok_and(|found| found.file_type().is_fifo())
}

/// Whether the reading end of `pipe`, a pipe or a FIFO, is closed, asked without writing a byte:
/// the standard library copies from an empty pipe that has no writer into it. On Linux that copy
/// is a `splice(2)`, which answers `EPIPE` when the pipe has no reader, and else finds the empty
/// source at its end and moves nothing. Where the copy is no `splice` (another kernel), it reads
/// the empty source and answers 0: the reader is never seen gone, and only a write of the rows
/// tells. Any other error is no answer and counts as a reader.
pub fn reader_gone(pipe: &std::fs::File) -> bool {
    use std::os::fd::OwnedFd;
    let Ok((empty, writer)) = std::io::pipe() else {
        return false;
    };
    drop(writer);
    let mut source = std::fs::File::from(OwnedFd::from(empty));
    let mut sink = pipe;
    matches!(
        std::io::copy(&mut source, &mut sink),
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe
    )
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
/// reader may show as a line end), each bidirectional control ([`bidirectional`]) and each
/// character a reader cannot see ([`unseen`]), written as its escape `\u{1b}`: a value another
/// principal or the service set reaches the terminal as visible text, never as a control
/// sequence, a line of its own, text shown in another order or text that is there and not seen.
/// Text without such characters is unchanged.
pub fn visible(text: &str) -> Cow<'_, str> {
    escaped(text, hidden)
}

/// Whether [`visible`] shows `character` as its escape.
fn hidden(character: char) -> bool {
    (character.is_control() && character != '\n')
        || matches!(character, '\u{2028}' | '\u{2029}')
        || bidirectional(character)
        || unseen(character)
}

/// The most bytes of the text of a body in the block of literal text of a chat post
/// ([`chat_post`]): a longer one is cut.
pub const MOST_CHAT_BODY_BYTES: usize = 3000;

/// The text of the chat post of `row`, a row of `Pending`, for `inbox bridge`: a head line
/// `<kind> from <sender>[ on behalf of <principal>] expires <expires> message <message id>`, then
/// the body in one block of literal text between 2 lines of 3 backticks, the delimiter of the
/// provider's block. Every character of the message is written as [`visible`] shows it (its
/// escape `\u{1b}`), a line end in the head line as `\u{a}`, a backtick in the head line and each
/// backtick of a run of 3 or more in the body as `\u{60}` (so no text of a sender ends the block
/// or opens one), then the provider's 3 escapes, `&` as `&amp;`, `<` as `&lt;` and `>` as `&gt;`
/// (so a mention of the provider, `<!channel>`, arrives as text). A body whose text would be over
/// [`MOST_CHAT_BODY_BYTES`] in the block is cut before the first character whose text would pass
/// it, so no escape is cut in half, and a line after the block says so and names the message id
/// and `inbox pending`, which shows it whole.
pub fn chat_post(row: &Value) -> String {
    let field = |name: &str| text(row, name);
    let mut out = String::new();
    chat_head(&mut out, field("kind"));
    out.push_str(" from ");
    chat_head(&mut out, field("from"));
    if field("on_behalf_of") != field("from") {
        out.push_str(" on behalf of ");
        chat_head(&mut out, field("on_behalf_of"));
    }
    out.push_str(" expires ");
    chat_head(&mut out, field("expires"));
    out.push_str(" message ");
    chat_head(&mut out, field("message_id"));
    out.push_str("\n```\n");
    let body: Vec<char> = field("body").chars().collect();
    let mut cut = false;
    let mut taken = 0;
    let mut piece = String::new();
    let mut at = 0;
    while at < body.len() && !cut {
        // A run of backticks, or one other character.
        let run = if body[at] == '`' {
            body[at..].iter().take_while(|held| **held == '`').count()
        } else {
            1
        };
        for character in &body[at..at + run] {
            piece.clear();
            chat_character(&mut piece, *character, run >= 3, false);
            if taken + piece.len() > MOST_CHAT_BODY_BYTES {
                cut = true;
                break;
            }
            taken += piece.len();
            out.push_str(&piece);
        }
        at += run;
    }
    out.push_str("\n```");
    if cut {
        out.push_str("\ncut at ");
        out.push_str(&MOST_CHAT_BODY_BYTES.to_string());
        out.push_str(" bytes: inbox pending shows the whole message ");
        chat_head(&mut out, field("message_id"));
    }
    out
}

/// The line `inbox bridge` writes into the thread of the chat post of a message that expired
/// unanswered.
pub const CHAT_EXPIRED: &str = "expired, not answered";

/// The line `inbox bridge` writes into the thread of the chat post of a decision silence decided:
/// `taken on silence: option <letter>, <text>`, the letter and the text of the option escaped as
/// a value of the head line of a post ([`chat_post`]); without a text, the letter alone.
pub fn chat_silence(letter: &str, text: Option<&str>) -> String {
    let mut out = String::from("taken on silence: option ");
    chat_head(&mut out, letter);
    if let Some(text) = text {
        out.push_str(", ");
        chat_head(&mut out, text);
    }
    out
}

/// Appends `text`, a value of a message in the head line of a chat post, as [`chat_post`] says.
fn chat_head(out: &mut String, text: &str) {
    for character in text.chars() {
        chat_character(out, character, true, true);
    }
}

/// Appends `character` of a message to a chat post: its escape where [`visible`] shows one (and a
/// line end `in_line`), `\u{60}` for a backtick that is `quoted`, and the provider's escape of
/// `&`, `<` and `>`.
fn chat_character(out: &mut String, character: char, quoted: bool, in_line: bool) {
    match character {
        '`' if quoted => out.push_str("\\u{60}"),
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '\n' if in_line => out.extend(character.escape_unicode()),
        _ if hidden(character) => out.extend(character.escape_unicode()),
        _ => out.push(character),
    }
}

/// Captures the standard output and the standard error of `command`, a child process the client
/// starts (`connectors` of `inbox bridge`): nothing of it reaches the client's own standard
/// output or standard error as it is, which this file alone writes.
pub fn captured(command: &mut std::process::Command) {
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
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

/// Format characters a terminal shows with no width, so 2 texts that differ look the same and a
/// text carries what a person does not see: the soft hyphen (U+00AD), the Mongolian vowel
/// separator (U+180E), the zero width space (U+200B), the word joiner and the invisible
/// operators (U+2060 to U+2064), the deprecated format characters (U+206A to U+206F), the zero
/// width no-break space or byte order mark (U+FEFF), the interlinear annotation characters
/// (U+FFF9 to U+FFFB) and the tag characters (U+E0000 to U+E007F). Not among them, printed as
/// sent because scripts and emoji sequences need them: the joiners (U+200C, U+200D), the
/// combining grapheme joiner (U+034F) and the variation selectors (U+FE00 to U+FE0F, U+E0100 to
/// U+E01EF).
fn unseen(character: char) -> bool {
    matches!(
        character,
        '\u{ad}'
            | '\u{180e}'
            | '\u{200b}'
            | '\u{2060}'..='\u{2064}'
            | '\u{206a}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0000}'..='\u{e007f}'
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
    use std::borrow::Cow;

    use inbox::server::json;

    use super::{MOST_CHAT_BODY_BYTES, chat_post, document, plain_document, rows_text, visible};

    /// A row of `Pending` with `from`, `on_behalf_of`, `kind` and `body` as given.
    fn chat_row(from: &str, on_behalf_of: &str, kind: &str, body: &str) -> json::Value {
        let text = |value: &str| json::Value::Text(value.to_owned());
        json::Value::Object(vec![
            ("message_id".to_owned(), text("01J00000000000000000000001")),
            ("kind".to_owned(), text(kind)),
            ("from".to_owned(), text(from)),
            ("on_behalf_of".to_owned(), text(on_behalf_of)),
            ("expires".to_owned(), text("2099-01-01T00:00:00Z")),
            ("body".to_owned(), text(body)),
        ])
    }

    /// The text of the block of literal text of a chat post, and what follows the block.
    fn block(post: &str) -> (&str, &str) {
        let (_, rest) = post.split_once("\n```\n").expect("a block");
        rest.rsplit_once("\n```").expect("the end of the block")
    }

    /// The head line holds no backtick and no line end of a sender, and names the principal a
    /// message speaks for when it is not the sender; its `<`, `>` and `&` are the provider's
    /// escapes, its control characters their escapes.
    #[test]
    fn the_head_line_of_a_chat_post_holds_no_backtick_and_no_line_end_of_a_sender() {
        let post = chat_post(&chat_row(
            "agent:test/a`b\nc<d>&\u{1b}",
            "human:person",
            "note",
            "Hi",
        ));
        let head = post.lines().next().unwrap();
        assert_eq!(
            head,
            "note from agent:test/a\\u{60}b\\u{a}c&lt;d&gt;&amp;\\u{1b} on behalf of human:person \
             expires 2099-01-01T00:00:00Z message 01J00000000000000000000001"
        );
        assert_eq!(post.matches("```").count(), 2, "{post}");
        let plain = chat_post(&chat_row("agent:test/a", "agent:test/a", "request", "Hi"));
        assert!(
            plain.starts_with("request from agent:test/a expires "),
            "{plain}"
        );
        assert_eq!(block(&plain), ("Hi", ""));
    }

    /// In the body a backtick of a run of 3 or more is `\u{60}`, a run of 1 or 2 stays; line ends
    /// stay; `&`, `<`, `>` are the provider's escapes; every character `visible` escapes is its
    /// escape.
    #[test]
    fn a_run_of_3_backticks_in_a_body_is_escaped_and_a_shorter_one_stays() {
        let post = chat_post(&chat_row(
            "agent:test/a",
            "agent:test/a",
            "note",
            "a``b```c````d`e\n<!here> & \u{202e}x\u{7}",
        ));
        let q = "\\u{60}";
        assert_eq!(
            block(&post),
            (
                format!("a``b{q}{q}{q}c{q}{q}{q}{q}d`e\n&lt;!here&gt; &amp; \\u{{202e}}x\\u{{7}}")
                    .as_str(),
                ""
            )
        );
    }

    /// A body whose text in the block would pass the bound is cut before the first character
    /// whose text does not fit, never inside an escape, and a line after the block names the
    /// message and `inbox pending`; a body whose text fits exactly is not cut.
    #[test]
    fn a_body_over_the_bound_is_cut_at_a_whole_character_and_names_the_message() {
        let fits = "é".repeat(MOST_CHAT_BODY_BYTES / 2);
        let post = chat_post(&chat_row("agent:test/a", "agent:test/a", "note", &fits));
        assert_eq!(block(&post), (fits.as_str(), ""));
        for (body, kept) in [
            (format!("{fits}é"), fits.clone()),
            ("&".repeat(1000), "&amp;".repeat(MOST_CHAT_BODY_BYTES / 5)),
            (
                format!("{}\u{1b}", "a".repeat(MOST_CHAT_BODY_BYTES - 1)),
                "a".repeat(MOST_CHAT_BODY_BYTES - 1),
            ),
        ] {
            let post = chat_post(&chat_row("agent:test/a", "agent:test/a", "note", &body));
            let (shown, after) = block(&post);
            assert_eq!(shown, kept);
            assert_eq!(
                after,
                "\ncut at 3000 bytes: inbox pending shows the whole message \
                 01J00000000000000000000001"
            );
        }
    }

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

    /// Each of `characters` between 2 letters is shown as its escape, named by its code point
    /// when it is not.
    fn each_shown_escaped(characters: impl IntoIterator<Item = char>) {
        for character in characters {
            assert_eq!(
                visible(&format!("a{character}b")),
                format!("a\\u{{{:x}}}b", u32::from(character)),
                "U+{:04X}",
                u32::from(character)
            );
        }
    }

    /// Each of `characters` between 2 letters is printed as written: the text is borrowed, not
    /// copied.
    fn each_printed_as_written(characters: impl IntoIterator<Item = char>) {
        for character in characters {
            let text = format!("a{character}b");
            assert!(
                matches!(visible(&text), Cow::Borrowed(shown) if std::ptr::eq(shown, text.as_str())),
                "U+{:04X}",
                u32::from(character)
            );
        }
    }

    /// The soft hyphen (U+00AD) is shown as its escape; U+00AC and U+00AE beside it are not.
    #[test]
    fn the_soft_hyphen_is_shown_escaped() {
        each_shown_escaped(['\u{ad}']);
        each_printed_as_written(['\u{ac}', '\u{ae}']);
    }

    /// The Mongolian vowel separator (U+180E) is shown as its escape; U+180D and U+180F beside it
    /// are not.
    #[test]
    fn the_mongolian_vowel_separator_is_shown_escaped() {
        each_shown_escaped(['\u{180e}']);
        each_printed_as_written(['\u{180d}', '\u{180f}']);
    }

    /// The zero width space (U+200B) is shown as its escape, also in an option text; U+200A and
    /// the zero width non-joiner U+200C beside it are not.
    #[test]
    fn the_zero_width_space_is_shown_escaped() {
        each_shown_escaped(['\u{200b}']);
        each_printed_as_written(['\u{200a}', '\u{200c}']);
        let text = shown(&decision("Ship\u{200b}\u{200b}now", ""));
        assert!(text.contains("  A) Ship\\u{200b}\\u{200b}now"), "{text}");
    }

    /// The word joiner and the invisible operators (U+2060 to U+2064) are shown as their
    /// escapes; U+205F before them and U+2065 after them are not.
    #[test]
    fn the_word_joiner_and_the_invisible_operators_are_shown_escaped() {
        each_shown_escaped('\u{2060}'..='\u{2064}');
        each_printed_as_written(['\u{205f}', '\u{2065}']);
    }

    /// The deprecated format characters (U+206A to U+206F) are shown as their escapes; U+2070
    /// after them is not (U+2069 before them is a bidirectional control).
    #[test]
    fn the_deprecated_format_characters_are_shown_escaped() {
        each_shown_escaped('\u{206a}'..='\u{206f}');
        each_printed_as_written(['\u{2070}']);
    }

    /// The zero width no-break space (U+FEFF, the byte order mark) is shown as its escape, also
    /// at the start of a text; U+FEFE and U+FF00 beside it are not.
    #[test]
    fn the_zero_width_no_break_space_is_shown_escaped() {
        each_shown_escaped(['\u{feff}']);
        assert_eq!(visible("\u{feff}Ship"), "\\u{feff}Ship");
        each_printed_as_written(['\u{fefe}', '\u{ff00}']);
    }

    /// The interlinear annotation characters (U+FFF9 to U+FFFB) are shown as their escapes;
    /// U+FFF8 and U+FFFC beside them are not.
    #[test]
    fn the_interlinear_annotation_characters_are_shown_escaped() {
        each_shown_escaped('\u{fff9}'..='\u{fffb}');
        each_printed_as_written(['\u{fff8}', '\u{fffc}']);
    }

    /// The tag characters (U+E0000 to U+E007F) are shown as their escapes, so the flag of a part
    /// of a country (an emoji tag sequence) is shown as its black flag and the escapes of its
    /// tags; U+DFFFF and U+E0080 beside them are not.
    #[test]
    fn the_tag_characters_are_shown_escaped() {
        each_shown_escaped('\u{e0000}'..='\u{e007f}');
        each_printed_as_written(['\u{dffff}', '\u{e0080}']);
        assert_eq!(
            visible("\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}"),
            "\u{1f3f4}\\u{e0067}\\u{e0062}\\u{e0073}\\u{e0063}\\u{e0074}\\u{e007f}"
        );
    }

    /// The joiners (U+200C, U+200D), the combining grapheme joiner (U+034F) and the variation
    /// selectors (U+FE00 to U+FE0F, U+E0100 to U+E01EF) are printed as sent: a Persian word with
    /// the zero width non-joiner between 2 of its letters, an emoji sequence with the zero width
    /// joiner and U+FE0F.
    #[test]
    fn the_joiners_and_the_variation_selectors_are_printed_as_sent() {
        for plain in [
            "\u{645}\u{6cc}\u{200c}\u{62e}\u{648}\u{627}\u{647}\u{645}",
            "\u{1f3f3}\u{fe0f}\u{200d}\u{1f308}",
            "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}",
        ] {
            assert!(
                matches!(visible(plain), Cow::Borrowed(text) if std::ptr::eq(text, plain)),
                "{plain:?}"
            );
        }
        each_printed_as_written(['\u{200c}', '\u{200d}', '\u{34f}']);
        each_printed_as_written('\u{fe00}'..='\u{fe0f}');
        each_printed_as_written('\u{e0100}'..='\u{e01ef}');
    }

    /// A text without a character `visible` escapes, with the neighbours of each range it
    /// escapes, is borrowed, not copied.
    #[test]
    fn a_text_without_an_escaped_character_is_borrowed() {
        let plain = "Grüße \u{ac}\u{ae} \u{180d}\u{180f} \u{200a}\u{200c}\u{200d} \u{205f}\u{2065}\u{2070} \u{fefe}\u{ff00} \u{fff8}\u{fffc} \u{dffff}\u{e0080} \u{1f3f3}\u{fe0f}\u{200d}\u{1f308} 世界";
        assert!(matches!(visible(plain), Cow::Borrowed(text) if std::ptr::eq(text, plain)));
    }

    /// `--json` is unchanged: each character of the ranges `visible` shows as their escapes
    /// beside the control characters (U+00AD, U+180E, U+200B, U+2060 to U+206F, U+FEFF, U+FFF9
    /// to U+FFFB, U+E0000 to U+E007F) stays in the document raw, as it was sent.
    #[test]
    fn a_document_keeps_a_character_text_shows_escaped_as_it_was_sent() {
        let sent: String = ['\u{ad}', '\u{180e}', '\u{200b}', '\u{feff}']
            .into_iter()
            .chain('\u{2060}'..='\u{206f}')
            .chain('\u{fff9}'..='\u{fffb}')
            .chain('\u{e0000}'..='\u{e007f}')
            .collect();
        let value = json::Value::Object(vec![("body".to_owned(), json::Value::Text(sent.clone()))]);
        let written = document(&value);
        assert!(written.contains(&sent), "{written:?}");
        assert!(matches!(plain_document(&written), Cow::Borrowed(text) if text == written));
        assert_eq!(json::parse(&written).unwrap(), value);
    }

    /// Every scalar value from U+0000 to U+10FFFF between 2 letters: `visible` shows it as its
    /// escape exactly when release 0.6.0 did (a control character but the line end, U+2028,
    /// U+2029, a bidirectional control) or it is in the list the story decided (U+00AD, U+180E,
    /// U+200B, U+2060 to U+2064, U+206A to U+206F, U+FEFF, U+FFF9 to U+FFFB, U+E0000 to
    /// U+E007F), and prints it as sent, borrowed, otherwise.
    #[test]
    fn every_scalar_value_is_escaped_exactly_when_0_6_0_or_the_decided_list_escapes_it() {
        let released = |character: char| {
            (character.is_control() && character != '\n')
                || matches!(character, '\u{2028}' | '\u{2029}')
                || matches!(
                    character,
                    '\u{61c}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        };
        let decided = |character: char| {
            matches!(
                u32::from(character),
                0xad | 0x180e
                    | 0x200b
                    | 0x2060..=0x2064
                    | 0x206a..=0x206f
                    | 0xfeff
                    | 0xfff9..=0xfffb
                    | 0xe0000..=0xe007f
            )
        };
        let mut wrong = Vec::new();
        for character in '\0'..=char::MAX {
            let text = format!("a{character}b");
            let shown = visible(&text);
            let escaped = matches!(shown, Cow::Owned(_));
            let expected = released(character) || decided(character);
            if escaped != expected {
                wrong.push(format!("U+{:04X} escaped {escaped}", u32::from(character)));
            } else if escaped {
                let want = format!("a\\u{{{:x}}}b", u32::from(character));
                if shown != want {
                    wrong.push(format!("U+{:04X} shown {shown:?}", u32::from(character)));
                }
            } else if !matches!(shown, Cow::Borrowed(kept) if std::ptr::eq(kept, text.as_str())) {
                wrong.push(format!("U+{:04X} not borrowed", u32::from(character)));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
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
