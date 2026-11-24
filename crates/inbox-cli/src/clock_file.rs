//! The clock of `inbox serve --clock-file <path>`: the service's clock reads the instant a file
//! holds, each time the service asks for the time. A flag of a debug build only, hidden from
//! `--help` and the reference: a test that starts `inbox serve` as a process moves the service's
//! clock past an `expires` by writing the file, and sleeps until no instant passes. A release
//! build (`cargo build --release`, `cargo install`) has no such flag.
//!
//! The file holds one RFC 3339 instant and at most one line end after it, nothing else: no key,
//! no secret and no message body is read from it, and no byte of it is repeated in a message. A
//! writer replaces it whole (writes a file beside it and renames it over it), so a reading never
//! sees half an instant.
//!
//! It fails closed. At the start, a file that is missing, cannot be read, is no regular file (a
//! FIFO, a directory, a device: it is opened without blocking and refused), is empty, or does not
//! hold one RFC 3339 instant ends `inbox serve` before it binds (exit 4). While it serves, a
//! reading that meets one of those, or an instant before the reading before it, ends the process
//! at once with exit 4 and one line on standard error that names the file and the cause: the
//! request or sweep that asked for the time gets no answer and changes nothing, and the service
//! never falls back to the system clock.
//!
//! `inbox bridge --clock-file <path>` reads its own clock the same way (a debug build only,
//! hidden too): the bridge's clock decides when the `expires` of a message and the retention of
//! its record have passed; it still signs its requests with the system clock.

use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use inbox_server::Clock;
use inbox_verify::Instant;

use crate::failure::LOCAL;
use crate::output::warn;

/// The most bytes read from the file: an RFC 3339 instant with nanoseconds, an offset and a line
/// end is 36. A longer file holds something else.
const MOST: u64 = 64;

/// A [`Clock`] that reads the instant a file holds; see the module documentation.
#[derive(Debug)]
pub struct FileClock {
    path: PathBuf,
    last: Mutex<Instant>,
    /// The command whose line names the file when a reading fails: `inbox serve`, `inbox bridge`.
    name: &'static str,
}

impl FileClock {
    /// Reads the file once, for the start of `inbox serve`.
    ///
    /// # Errors
    ///
    /// The cause, as [`read`] states it, when the file does not hold one instant.
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::of("inbox serve", path)
    }

    /// Reads the file once, for the start of the command `name` (`inbox bridge`), whose line
    /// names the file when a later reading fails.
    ///
    /// # Errors
    ///
    /// The cause, as [`read`] states it, when the file does not hold one instant.
    pub fn of(name: &'static str, path: &Path) -> Result<Self, String> {
        let first = read(path)?;
        Ok(Self {
            path: path.to_owned(),
            last: Mutex::new(first),
            name,
        })
    }

    /// The reading at the start.
    pub fn first(&self) -> Instant {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The instant the file holds now, when it is not before `last`.
    fn reading(&self) -> Result<Instant, String> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let at = read(&self.path)?;
        if at < *last {
            return Err(format!(
                "it went back to {}, before {}, the reading before it",
                inbox_server::format_reading(at),
                inbox_server::format_reading(*last)
            ));
        }
        *last = at;
        Ok(at)
    }
}

impl Clock for FileClock {
    fn now(&self) -> Instant {
        match self.reading() {
            Ok(at) => at,
            Err(cause) => {
                warn(&format!(
                    "{}: the clock file {}: {cause}: stopping",
                    self.name,
                    self.path.display()
                ));
                std::process::exit(LOCAL);
            }
        }
    }
}

/// The instant the file at `path` holds.
///
/// # Errors
///
/// Why it holds none: it cannot be opened or read (the error of the OS), it is no regular file,
/// it is empty, or it is longer than an instant or its text is not one RFC 3339 instant. The
/// text itself is never part of the cause. The file is opened with `O_NONBLOCK`, so a FIFO
/// without a writer is refused at once instead of holding the open.
pub fn read(path: &Path) -> Result<Instant, String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| error.to_string())?;
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("it is no regular file".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(MOST + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.is_empty() {
        return Err("it is empty".to_owned());
    }
    if bytes.len() as u64 > MOST {
        return Err("it does not hold one RFC 3339 instant".to_owned());
    }
    let text = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    std::str::from_utf8(text)
        .ok()
        .and_then(|text| Instant::parse(text).ok())
        .ok_or_else(|| "it does not hold one RFC 3339 instant".to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use inbox_verify::Instant;

    use super::{FileClock, read};

    /// A directory of its own under the build directory, emptied first.
    fn dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/clock-file")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the directory is made");
        dir
    }

    fn at(text: &str) -> Instant {
        Instant::parse(text).expect("an instant")
    }

    #[test]
    fn the_file_holds_one_instant_and_at_most_one_line_end() {
        let dir = dir("one-instant");
        let path = dir.join("clock");
        for text in ["2026-11-22T12:00:00Z", "2026-11-22T12:00:00Z\n"] {
            fs::write(&path, text).expect("written");
            assert_eq!(read(&path), Ok(at("2026-11-22T12:00:00Z")), "{text:?}");
        }
        fs::write(&path, "2026-11-22T13:00:00.5+01:00").expect("written");
        assert_eq!(read(&path), Ok(at("2026-11-22T12:00:00.5Z")));
    }

    #[test]
    fn each_bad_form_of_the_file_is_named_and_its_text_is_not() {
        let dir = dir("bad-forms");
        let missing = dir.join("missing");
        assert!(
            read(&missing).is_err_and(|cause| cause.contains("No such file")),
            "{:?}",
            read(&missing)
        );
        let unreadable = dir.join("a-directory");
        fs::create_dir(&unreadable).expect("a directory");
        assert!(read(&unreadable).is_err(), "{:?}", read(&unreadable));
        let path = dir.join("clock");
        for (text, cause) in [
            ("", "it is empty"),
            ("\n", "it does not hold one RFC 3339 instant"),
            (
                "secret-looking text",
                "it does not hold one RFC 3339 instant",
            ),
            (
                "2026-11-22T12:00:00Z\n\n",
                "it does not hold one RFC 3339 instant",
            ),
            (
                "2026-11-22 12:00:00",
                "it does not hold one RFC 3339 instant",
            ),
            (
                "2026-11-22T12:00:00Z\u{ff}",
                "it does not hold one RFC 3339 instant",
            ),
        ] {
            fs::write(&path, text).expect("written");
            assert_eq!(read(&path), Err(cause.to_owned()), "{text:?}");
        }
        fs::write(&path, "2".repeat(65)).expect("written");
        assert_eq!(
            read(&path),
            Err("it does not hold one RFC 3339 instant".to_owned())
        );
        let fifo = dir.join("a-fifo");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo");
        assert_eq!(read(&fifo), Err("it is no regular file".to_owned()));
    }

    #[test]
    fn a_reading_follows_the_file_forward_and_refuses_a_step_back() {
        let dir = dir("forward");
        let path = dir.join("clock");
        fs::write(&path, "2026-11-22T12:00:00Z").expect("written");
        let clock = FileClock::open(&path).expect("the clock opens");
        assert_eq!(clock.first(), at("2026-11-22T12:00:00Z"));
        fs::write(&path, "2026-11-22T12:00:31Z").expect("written");
        assert_eq!(clock.reading(), Ok(at("2026-11-22T12:00:31Z")));
        assert_eq!(clock.reading(), Ok(at("2026-11-22T12:00:31Z")), "the same");
        fs::write(&path, "2026-11-22T12:00:30Z").expect("written");
        assert_eq!(
            clock.reading(),
            Err(
                "it went back to 2026-11-22T12:00:30Z, before 2026-11-22T12:00:31Z, the reading \
                 before it"
                    .to_owned()
            )
        );
        fs::remove_file(&path).expect("removed");
        assert!(clock.reading().is_err());
    }
}
