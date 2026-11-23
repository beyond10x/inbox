//! The secret key file a client signs with, `inbox key new`, the two commands that append an
//! entry signed with the operator key to the key file the service reads, and the TLS file of the
//! service, `inbox tls new`.
//!
//! A secret key file is one JSON object:
//!
//! ```text
//! {"inbox_secret_key":1,"key_id":"…","principal":"…","public_key":"<44 base64>","seed":"<44 base64>"}
//! ```
//!
//! `seed` is the 32-byte Ed25519 seed (`inbox_verify::SecretKey::to_seed`) in canonical base64;
//! `public_key` is its public half, checked against the seed when the file is read. `key_id` and
//! `principal` name the key entry of the key file that holds this key; an operator key has
//! neither. The file is created with mode 0600 and never widened, and a file that grants any
//! permission to its group or to others is refused when read.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use inbox::messaging::{DelegationEntry, DelegationId, KeyEntry, KeyFile, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_server::tls::Identity;
use inbox_server::{Clock, SystemClock, format_instant};
use inbox_verify::{
    IgnoreReason, Instant, KeyRing, PublicKey, SecretKey, duplicate_member, sign_delegation_entry,
    sign_key_entry,
};
use zeroize::Zeroizing;

use crate::failure::Failure;
use crate::output::Done;

/// The version member of a secret key file.
const VERSION_MEMBER: &str = "inbox_secret_key";

/// A secret key read from its file.
pub struct Secret {
    /// The key.
    pub key: SecretKey,
    /// The key id of its entry in the key file, for a client key.
    pub key_id: Option<String>,
    /// The principal of its entry in the key file, for a client key.
    pub principal: Option<String>,
}

/// The caller of a client command: its key, the key id the service resolves, and the principal
/// that entry names, which is also the address of the caller's inbox.
pub struct Caller {
    /// The key.
    pub key: SecretKey,
    /// The key id.
    pub key_id: String,
    /// The principal.
    pub principal: String,
}

/// The text of the file at `path` that holds a secret, in a holder zeroed when dropped.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be read, or grants a permission to its group or to
/// others.
fn read_private(path: &Path) -> Result<Zeroizing<String>, Failure> {
    let shown = path.display();
    let metadata =
        fs::metadata(path).map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
    let mode = metadata.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(Failure::Local(format!(
            "{shown}: a secret key file must be readable by its owner only (mode {mode:04o}, \
             expected 0600)"
        )));
    }
    // Read into a buffer large enough from the start, so no reallocation leaves a copy behind.
    let capacity = usize::try_from(metadata.len())
        .unwrap_or(0)
        .saturating_add(64);
    let mut text = Zeroizing::new(String::with_capacity(capacity));
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut text))
        .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
    Ok(text)
}

/// Reads the secret key file at `path`.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be read, grants a permission to its group or to
/// others, or is not a secret key file.
pub fn read_secret(path: &Path) -> Result<Secret, Failure> {
    let shown = path.display();
    let text = read_private(path)?;
    let refused = |why: &str| Failure::Local(format!("{shown}: not a secret key file: {why}"));
    let mut value = json::parse(&text).map_err(|error| refused(&error.to_string()))?;
    // The seed's text leaves the parsed tree into a holder zeroed when dropped, before anything
    // else can fail and drop the tree.
    let seed_text = take_text(&mut value, "seed");
    if let Some(member) = duplicate_member(&value) {
        return Err(refused(&format!("the member `{member}` is there twice")));
    }
    if value.member(VERSION_MEMBER) != Some(&Value::Number("1".to_owned())) {
        return Err(refused(&format!("`{VERSION_MEMBER}` is not 1")));
    }
    let seed = seed_text
        .as_deref()
        .and_then(|text| seed_from_base64(text))
        .ok_or_else(|| refused("`seed` is not 32 bytes of canonical base64"))?;
    let key = SecretKey::from_seed(&seed);
    let public_key = optional_text(&value, "public_key");
    if public_key.as_deref() != Some(key.public_key().to_base64().as_str()) {
        return Err(refused("`public_key` is not the public half of `seed`"));
    }
    Ok(Secret {
        key,
        key_id: optional_text(&value, "key_id"),
        principal: optional_text(&value, "principal"),
    })
}

/// Moves the text of the top-level member `name` out of `value` into a holder zeroed when
/// dropped; the member is left an empty text. `None` when it is absent or not a text.
fn take_text(value: &mut Value, name: &str) -> Option<Zeroizing<String>> {
    match value {
        Value::Object(members) => members.iter_mut().find_map(|(member, held)| match held {
            Value::Text(text) if member == name => Some(Zeroizing::new(std::mem::take(text))),
            _ => None,
        }),
        _ => None,
    }
}

/// The standard base64 alphabet, padded with `=`: the wire encoding of keys (`signing.md`).
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The 44 characters of a 32-byte seed, appended to `out` without a buffer of their own: `out`
/// is a holder the caller made large enough, so nothing is reallocated.
fn seed_to_base64(seed: &[u8; 32], out: &mut String) {
    for chunk in seed.chunks(3) {
        let a = u32::from(chunk[0]);
        let b = chunk.get(1).copied().map_or(0, u32::from);
        let c = chunk.get(2).copied().map_or(0, u32::from);
        let packed = (a << 16) | (b << 8) | c;
        for at in 0..4 {
            if at <= chunk.len() {
                let index = usize::try_from((packed >> (18 - 6 * at)) & 63).unwrap_or(0);
                out.push(char::from(BASE64[index]));
            } else {
                out.push('=');
            }
        }
    }
}

/// The seed from its canonical base64: exactly 44 characters that decode to 32 bytes and encode
/// back to the same text.
fn seed_from_base64(text: &str) -> Option<Zeroizing<[u8; 32]>> {
    let bytes = text.as_bytes();
    if bytes.len() != 44 || bytes[43] != b'=' {
        return None;
    }
    let mut seed = Zeroizing::new([0_u8; 32]);
    let mut bits: u32 = 0;
    let mut count = 0;
    let mut at = 0;
    for symbol in &bytes[..43] {
        let value = BASE64.iter().position(|known| known == symbol)?;
        bits = (bits << 6) | u32::try_from(value).ok()?;
        count += 6;
        if count >= 8 {
            count -= 8;
            if at < 32 {
                seed[at] = u8::try_from((bits >> count) & 0xFF).ok()?;
            }
            at += 1;
        }
        bits &= (1 << count) - 1;
    }
    let mut again = Zeroizing::new(String::with_capacity(44));
    seed_to_base64(&seed, &mut again);
    (at == 32 && again.as_str() == text).then_some(seed)
}

fn optional_text(value: &Value, name: &str) -> Option<String> {
    match value.member(name) {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

/// The caller from the secret key file at `path` (the flag `--secret-key-file` or the variable
/// `INBOX_SECRET_KEY_FILE`).
///
/// # Errors
///
/// [`Failure::Usage`] when neither names a file; [`Failure::Local`] as [`read_secret`], and for a
/// key file that names no key id or principal (an operator key).
pub fn caller(path: Option<&Path>) -> Result<Caller, Failure> {
    let path = path.ok_or_else(|| {
        Failure::Usage(
            "the caller's key: pass --secret-key-file <path> or set INBOX_SECRET_KEY_FILE"
                .to_owned(),
        )
    })?;
    let secret = read_secret(path)?;
    match (secret.key_id, secret.principal) {
        (Some(key_id), Some(principal)) => Ok(Caller {
            key: secret.key,
            key_id,
            principal,
        }),
        _ => Err(Failure::Local(format!(
            "{}: names no key id and principal; make a client key with \
             `inbox key new --key-id <id> --principal <principal>`",
            path.display()
        ))),
    }
}

/// `inbox key new`: a fresh key pair, its secret written to `out` with mode 0600; prints the
/// public key and nothing secret.
///
/// # Errors
///
/// [`Failure::Local`] when `out` exists (never overwritten) or cannot be written.
pub fn new(out: &Path, identity: Option<(&str, &str)>) -> Result<Done, Failure> {
    let key = SecretKey::generate().map_err(|error| Failure::Local(error.to_string()))?;
    let public_key = key.public_key().to_base64();
    // Large enough from the start (a character escaped takes at most 6 bytes), so the buffer
    // that holds the seed's text is never reallocated and only this one copy is zeroed.
    let identity_bytes = identity.map_or(0, |(key_id, principal)| key_id.len() + principal.len());
    let mut text = Zeroizing::new(String::with_capacity(256 + 6 * identity_bytes));
    text.push('{');
    json::member(&mut text, VERSION_MEMBER);
    json::push_integer(&mut text, 1);
    if let Some((key_id, principal)) = identity {
        json::member(&mut text, "key_id");
        json::push_text(&mut text, key_id);
        json::member(&mut text, "principal");
        json::push_text(&mut text, principal);
    }
    json::member(&mut text, "public_key");
    json::push_text(&mut text, &public_key);
    json::member(&mut text, "seed");
    text.push('"');
    seed_to_base64(&key.to_seed(), &mut text);
    text.push_str("\"}\n");
    write_private(out, &text, "inbox key new")?;
    let mut document = String::from("{");
    json::member(&mut document, "public_key");
    json::push_text(&mut document, &public_key);
    json::member(&mut document, "file");
    json::push_text(&mut document, &out.display().to_string());
    if let Some((key_id, principal)) = identity {
        json::member(&mut document, "key_id");
        json::push_text(&mut document, key_id);
        json::member(&mut document, "principal");
        json::push_text(&mut document, principal);
    }
    document.push('}');
    Ok(Done {
        document,
        text: public_key,
    })
}

/// Creates the file `out` with mode 0600 and writes `text` into it; an existing file is never
/// overwritten, which the refusal says in the name of `command`.
fn write_private(out: &Path, text: &str, command: &str) -> Result<(), Failure> {
    let shown = out.display();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(out)
        .map_err(|error| match error.kind() {
            ErrorKind::AlreadyExists => Failure::Local(format!(
                "{shown} exists: `{command}` never overwrites a file"
            )),
            _ => Failure::Local(format!("{shown}: {error}")),
        })?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| Failure::Local(format!("{shown}: {error}")))
}

/// The version member of a TLS file.
const TLS_VERSION_MEMBER: &str = "inbox_tls";

/// `inbox tls new`: a fresh Ed25519 key and a self-signed certificate over it
/// ([`Identity::generate`]), written to `out` with mode 0600 as one JSON object
/// (`{"inbox_tls":1,"certificate":"<base64 DER>","seed":"<44 base64>"}`); prints the pin of the
/// certificate and nothing secret.
///
/// # Errors
///
/// [`Failure::Local`] when `out` exists (never overwritten) or cannot be written, or the OS
/// random source does not answer.
pub fn tls_new(out: &Path) -> Result<Done, Failure> {
    let identity = Identity::generate().map_err(|error| Failure::Local(error.to_string()))?;
    let mut certificate = String::new();
    json::push_base64(&mut certificate, identity.certificate());
    // Large enough from the start, so the buffer that holds the seed's text is never
    // reallocated and only this one copy is zeroed.
    let mut text = Zeroizing::new(String::with_capacity(128 + certificate.len()));
    text.push('{');
    json::member(&mut text, TLS_VERSION_MEMBER);
    json::push_integer(&mut text, 1);
    json::member(&mut text, "certificate");
    text.push_str(&certificate);
    json::member(&mut text, "seed");
    text.push('"');
    seed_to_base64(identity.seed(), &mut text);
    text.push_str("\"}\n");
    write_private(out, &text, "inbox tls new")?;
    let pin = identity.pin().to_base64();
    let mut document = String::from("{");
    json::member(&mut document, "pin");
    json::push_text(&mut document, &pin);
    json::member(&mut document, "file");
    json::push_text(&mut document, &out.display().to_string());
    document.push('}');
    Ok(Done {
        document,
        text: pin,
    })
}

/// Reads the TLS file `inbox tls new` wrote at `path`: the certificate and the key of the
/// service.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be read, grants a permission to its group or to
/// others, or is not a TLS file: its version member is not 1, its certificate or seed does not
/// decode, or the certificate is not of the seed's key.
pub fn read_tls(path: &Path) -> Result<Identity, Failure> {
    let shown = path.display();
    let text = read_private(path)?;
    let refused = |why: &str| Failure::Local(format!("{shown}: not a TLS file: {why}"));
    let mut value = json::parse(&text).map_err(|error| refused(&error.to_string()))?;
    let seed_text = take_text(&mut value, "seed");
    if let Some(member) = duplicate_member(&value) {
        return Err(refused(&format!("the member `{member}` is there twice")));
    }
    if value.member(TLS_VERSION_MEMBER) != Some(&Value::Number("1".to_owned())) {
        return Err(refused(&format!("`{TLS_VERSION_MEMBER}` is not 1")));
    }
    let seed = seed_text
        .as_deref()
        .and_then(|text| seed_from_base64(text))
        .ok_or_else(|| refused("`seed` is not 32 bytes of canonical base64"))?;
    let certificate = value
        .member("certificate")
        .and_then(|certificate| json::bytes_at(certificate, "certificate", "base64").ok())
        .ok_or_else(|| refused("`certificate` is not base64"))?;
    Identity::from_parts(certificate, seed).map_err(|why| refused(&why))
}

/// What `inbox keyfile add-key` and `add-delegation` share: the key file and the operator key.
pub struct KeyFileTarget<'a> {
    /// The key file; created when absent.
    pub file: &'a Path,
    /// The operator's secret key file.
    pub operator: &'a Path,
}

/// An entry's `valid_until`: the text given, held to RFC 3339 and after now, or 30 days from now.
///
/// # Errors
///
/// [`Failure::Usage`] for a text that is not an RFC 3339 instant, or one at or before now: such
/// an entry is lapsed from the start (`signing.md`, lapsed), and would count for nothing.
pub fn valid_until(given: Option<&str>) -> Result<String, Failure> {
    let now = SystemClock.now();
    match given {
        Some(text) => {
            let until = Instant::parse(text)
                .map_err(|error| Failure::Usage(format!("--valid-until `{text}`: {error}")))?;
            if until <= now {
                return Err(Failure::Usage(format!(
                    "--valid-until `{text}` is not after now ({}): the entry would be lapsed \
                     from the start",
                    format_instant(now)
                )));
            }
            Ok(text.to_owned())
        }
        None => Ok(format_instant(now.plus_seconds(30 * 86_400))),
    }
}

/// `inbox keyfile add-key`.
///
/// # Errors
///
/// [`Failure::Usage`] for a public key that is not one; [`Failure::Local`] for a key file that
/// cannot be read or written, does not read as a key file under the operator key, or already
/// carries the id (an id is never reused).
pub fn add_key(
    target: &KeyFileTarget<'_>,
    key_id: &str,
    principal: &str,
    public_key: &str,
    valid_until: &str,
) -> Result<Done, Failure> {
    PublicKey::from_base64(public_key)
        .map_err(|error| Failure::Usage(format!("--public-key: {error}")))?;
    let mut entry = KeyEntry {
        key_id: key_id.to_owned(),
        principal: Principal(principal.to_owned()),
        public_key: public_key.to_owned(),
        valid_until: Timestamp(valid_until.to_owned()),
        operator_signature: String::new(),
    };
    append(target, key_id, |file, operator| {
        if file.keys.iter().any(|held| held.key_id == key_id) {
            return false;
        }
        sign_key_entry(&mut entry, operator);
        file.keys.push(entry.clone());
        true
    })?;
    let mut document = String::from("{");
    json::member(&mut document, "file");
    json::push_text(&mut document, &target.file.display().to_string());
    json::member(&mut document, "key_id");
    json::push_text(&mut document, key_id);
    json::member(&mut document, "principal");
    json::push_text(&mut document, principal);
    json::member(&mut document, "valid_until");
    json::push_text(&mut document, valid_until);
    document.push('}');
    Ok(Done {
        document,
        text: format!(
            "key {key_id} of {principal}, valid until {valid_until}, added to {}",
            target.file.display()
        ),
    })
}

/// `inbox keyfile add-delegation`.
///
/// # Errors
///
/// As [`add_key`], for a delegation id.
pub fn add_delegation(
    target: &KeyFileTarget<'_>,
    delegation_id: &str,
    agent: &str,
    on_behalf_of: &str,
    valid_until: &str,
) -> Result<Done, Failure> {
    let mut entry = DelegationEntry {
        delegation_id: DelegationId(delegation_id.to_owned()),
        agent: Principal(agent.to_owned()),
        on_behalf_of: Principal(on_behalf_of.to_owned()),
        valid_until: Timestamp(valid_until.to_owned()),
        operator_signature: String::new(),
    };
    append(target, delegation_id, |file, operator| {
        if file
            .delegations
            .iter()
            .any(|held| held.delegation_id.0 == delegation_id)
        {
            return false;
        }
        sign_delegation_entry(&mut entry, operator);
        file.delegations.push(entry.clone());
        true
    })?;
    let mut document = String::from("{");
    json::member(&mut document, "file");
    json::push_text(&mut document, &target.file.display().to_string());
    json::member(&mut document, "delegation_id");
    json::push_text(&mut document, delegation_id);
    json::member(&mut document, "agent");
    json::push_text(&mut document, agent);
    json::member(&mut document, "on_behalf_of");
    json::push_text(&mut document, on_behalf_of);
    json::member(&mut document, "valid_until");
    json::push_text(&mut document, valid_until);
    document.push('}');
    Ok(Done {
        document,
        text: format!(
            "delegation {delegation_id}: {agent} speaks for {on_behalf_of} until {valid_until}, \
             added to {}",
            target.file.display()
        ),
    })
}

/// Under an exclusive lock of `<file>.lock` (two runs at once lose no entry): reads the key file
/// (an empty one when absent), refuses one holding an entry this operator key did not sign, lets
/// `add` append one entry signed with the operator key, checks that `KeyRing::parse` reads the
/// result and holds the new entry, and replaces the file with it, keeping its mode.
fn append(
    target: &KeyFileTarget<'_>,
    id: &str,
    add: impl FnOnce(&mut KeyFile, &SecretKey) -> bool,
) -> Result<(), Failure> {
    let operator = read_secret(target.operator)?.key;
    let operator_public = operator.public_key();
    let shown = target.file.display();
    let local = |error: std::io::Error| Failure::Local(format!("{shown}: {error}"));
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(beside(target.file, ".lock"))
        .map_err(local)?;
    lock.lock().map_err(local)?;
    let (mut file, mode) = match fs::read_to_string(target.file) {
        Ok(text) => {
            let ring = KeyRing::parse(&text, &operator_public)
                .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
            if let Some(foreign) = ring.ignored().iter().find(|ignored| {
                matches!(
                    ignored.reason,
                    IgnoreReason::NotSignedByOperator | IgnoreReason::SignatureEncoding
                )
            }) {
                return Err(Failure::Local(format!(
                    "{shown}: the {:?} entry `{}` is not signed by this operator key ({:?}); \
                     a key file holds the entries of one operator key, and nothing is appended \
                     to it with another",
                    foreign.kind, foreign.id, foreign.reason
                )));
            }
            let value =
                json::parse(&text).map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
            let file = wire::decode_inbox_messaging_key_file(&value, "")
                .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
            let mode = fs::metadata(target.file)
                .map_err(local)?
                .permissions()
                .mode()
                & 0o7777;
            (file, Some(mode))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => (
            KeyFile {
                keys: Vec::new(),
                delegations: Vec::new(),
            },
            None,
        ),
        Err(error) => return Err(local(error)),
    };
    if !add(&mut file, &operator) {
        return Err(Failure::Local(format!(
            "{shown} already carries the id `{id}`: an id is never reused, a new entry takes a \
             new id"
        )));
    }
    let mut text = String::new();
    wire::encode_inbox_messaging_key_file(&file, &mut text);
    text.push('\n');
    let ring = KeyRing::parse(&text, &operator_public)
        .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
    if let Some(ignored) = ring.ignored().iter().find(|ignored| ignored.id == id) {
        return Err(Failure::Local(format!(
            "{shown}: the new entry `{id}` would be ignored ({:?}); the file is unchanged",
            ignored.reason
        )));
    }
    replace(target.file, text.as_bytes(), mode)?;
    lock.unlock().map_err(local)
}

fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut beside = PathBuf::from(path);
    beside.as_mut_os_string().push(suffix);
    beside
}

/// Replaces the file at `path` with `bytes`: written to a file of this process's own beside it,
/// `<path>.<pid>.new`, created anew by [`stage`] with `mode` (the mode of the file it replaces) or,
/// when there is none, 0666 narrowed by the umask (the only callers, `add_key` and
/// `add_delegation`, write the key file, which holds public keys and signatures and no secret
/// bytes); then given `mode` exactly (the umask may narrow it at creation), written, synced and
/// renamed over it. On a failure the staged file is removed.
fn replace(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<(), Failure> {
    let shown = path.display();
    let staged = beside(path, &format!(".{}.new", std::process::id()));
    let written = (|| {
        let mut file = stage(&staged, mode.unwrap_or(0o666))?;
        if let Some(mode) = mode {
            file.set_permissions(fs::Permissions::from_mode(mode))?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&staged, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&staged);
    }
    written.map_err(|error| Failure::Local(format!("{shown}: {error}")))
}

/// Removes what is at `staged` (a file, a link, a FIFO: none of them is opened or followed; a
/// directory is a failure) and creates a new file there with `mode`, narrowed by the umask.
fn stage(staged: &Path, mode: u32) -> std::io::Result<File> {
    if let Err(error) = fs::remove_file(staged)
        && error.kind() != ErrorKind::NotFound
    {
        return Err(error);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(staged)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::process::Command;

    use inbox::server::json;

    use super::{seed_from_base64, seed_to_base64, stage};

    /// The umask of this process, from `/proc/self/status`.
    fn umask() -> u32 {
        let status = fs::read_to_string("/proc/self/status").expect("/proc/self/status is read");
        let line = status
            .lines()
            .find_map(|line| line.strip_prefix("Umask:"))
            .expect("/proc/self/status names the umask");
        u32::from_str_radix(line.trim(), 8).expect("the umask is octal")
    }

    /// `stage` removes what is at the staged place and creates an empty regular file there with
    /// the mode it is given, narrowed only by the umask: nothing there, a link to another file of
    /// the directory (removed, its target byte for byte unchanged), a FIFO (removed, not opened)
    /// and a regular file left behind. A directory there is a failure and is left as it is. The
    /// row of mode 0000 tells a file created with its mode from one created with 0666 under any
    /// umask but one that removes every read and write permission.
    #[test]
    fn the_staged_file_is_created_anew_with_its_mode() {
        let dir = std::env::current_exe()
            .expect("the test binary has a path")
            .with_file_name(format!("keys-stage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the directory is made");
        let other = dir.join("other");
        fs::write(&other, b"other bytes\n").unwrap();
        fs::set_permissions(&other, fs::Permissions::from_mode(0o640)).unwrap();
        let staged = dir.join("keys.json.1.new");
        let umask = umask();
        for (planted, mode) in [
            ("nothing", 0o000),
            ("a link", 0o600),
            ("a FIFO", 0o640),
            ("a file", 0o600),
        ] {
            match planted {
                "a link" => symlink("other", &staged).unwrap(),
                "a FIFO" => assert!(
                    Command::new("mkfifo")
                        .arg(&staged)
                        .status()
                        .expect("mkfifo runs")
                        .success()
                ),
                "a file" => fs::write(&staged, b"left behind").unwrap(),
                _ => {}
            }
            drop(stage(&staged, mode).unwrap_or_else(|error| panic!("{planted}: {error}")));
            let made = fs::symlink_metadata(&staged).unwrap();
            assert!(made.file_type().is_file(), "{planted}: {made:?}");
            assert_eq!(made.len(), 0, "{planted}");
            assert_eq!(made.mode() & 0o7777, mode & !umask, "{planted}");
            assert_eq!(fs::read(&other).unwrap(), b"other bytes\n", "{planted}");
            assert_eq!(fs::metadata(&other).unwrap().mode() & 0o7777, 0o640);
            fs::remove_file(&staged).unwrap();
        }
        fs::create_dir(&staged).unwrap();
        assert!(stage(&staged, 0o600).is_err(), "a directory is a failure");
        assert!(fs::symlink_metadata(&staged).unwrap().is_dir());
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The seed's base64 is the one `json::push_base64` writes (`signing.md`'s encoding), and it
    /// reads back to the same bytes.
    #[test]
    fn a_seed_round_trips_through_its_base64() {
        for fill in [0_u8, 1, 0x7F, 0xFF] {
            let mut seed = [fill; 32];
            seed[0] = 0xA5;
            seed[31] = fill.wrapping_add(3);
            let mut ours = String::with_capacity(44);
            seed_to_base64(&seed, &mut ours);
            assert_eq!(ours.capacity(), 44, "written without a reallocation");
            let mut theirs = String::new();
            json::push_base64(&mut theirs, &seed);
            assert_eq!(format!("\"{ours}\""), theirs);
            assert_eq!(*seed_from_base64(&ours).unwrap(), seed);
        }
    }

    #[test]
    fn a_seed_text_that_is_not_canonical_base64_of_32_bytes_is_refused() {
        let mut text = String::with_capacity(44);
        seed_to_base64(&[7; 32], &mut text);
        let last = text.chars().nth(42).unwrap();
        let mut unused_bits = text.clone();
        // The last symbol carries 2 unused bits; setting one gives the same bytes, not canonical.
        unused_bits.replace_range(42..43, if last == 'E' { "F" } else { "B" });
        for refused in [
            unused_bits.as_str(),
            &text[..43],
            &format!("{text}="),
            &text.replace('=', "A"),
            &text.replacen(char::is_alphanumeric, "-", 1),
        ] {
            assert!(seed_from_base64(refused).is_none(), "{refused}");
        }
    }
}
