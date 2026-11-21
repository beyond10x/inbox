//! The secret key file a client signs with, `inbox key new`, and the two commands that append an
//! entry signed with the operator key to the key file the service reads.
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

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use inbox::messaging::{DelegationEntry, DelegationId, KeyEntry, KeyFile, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_server::{Clock, SystemClock, format_instant};
use inbox_verify::{
    Instant, KeyRing, PublicKey, SecretKey, duplicate_member, sign_delegation_entry, sign_key_entry,
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

/// Reads the secret key file at `path`.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be read, grants a permission to its group or to
/// others, or is not a secret key file.
pub fn read_secret(path: &Path) -> Result<Secret, Failure> {
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
    let text = Zeroizing::new(
        fs::read_to_string(path).map_err(|error| Failure::Local(format!("{shown}: {error}")))?,
    );
    let refused = |why: &str| Failure::Local(format!("{shown}: not a secret key file: {why}"));
    let value = json::parse(&text).map_err(|error| refused(&error.to_string()))?;
    if let Some(member) = duplicate_member(&value) {
        return Err(refused(&format!("the member `{member}` is there twice")));
    }
    if value.member(VERSION_MEMBER) != Some(&Value::Number("1".to_owned())) {
        return Err(refused(&format!("`{VERSION_MEMBER}` is not 1")));
    }
    let seed = Zeroizing::new(
        json::bytes_at(
            value.member("seed").unwrap_or(&Value::Null),
            "seed",
            "base64",
        )
        .map_err(|error| refused(&error.to_string()))?,
    );
    let seed: Zeroizing<[u8; 32]> = Zeroizing::new(
        seed.as_slice()
            .try_into()
            .map_err(|_| refused("`seed` is not 32 bytes"))?,
    );
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
    let mut text = Zeroizing::new(String::from("{"));
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
    json::push_base64(&mut text, key.to_seed().as_slice());
    text.push_str("}\n");
    let shown = out.display();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(out)
        .map_err(|error| match error.kind() {
            ErrorKind::AlreadyExists => Failure::Local(format!(
                "{shown} exists: `inbox key new` never overwrites a file"
            )),
            _ => Failure::Local(format!("{shown}: {error}")),
        })?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
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

/// What `inbox keyfile add-key` and `add-delegation` share: the key file and the operator key.
pub struct KeyFileTarget<'a> {
    /// The key file; created when absent.
    pub file: &'a Path,
    /// The operator's secret key file.
    pub operator: &'a Path,
}

/// An entry's `valid_until`: the text given, held to RFC 3339, or 30 days from now.
///
/// # Errors
///
/// [`Failure::Usage`] for a text that is not an RFC 3339 instant.
pub fn valid_until(given: Option<&str>) -> Result<String, Failure> {
    match given {
        Some(text) => {
            Instant::parse(text)
                .map_err(|error| Failure::Usage(format!("--valid-until `{text}`: {error}")))?;
            Ok(text.to_owned())
        }
        None => Ok(format_instant(SystemClock.now().plus_seconds(30 * 86_400))),
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

/// Reads the key file (an empty one when absent), lets `add` append one entry signed with the
/// operator key, checks that `KeyRing::parse` reads the result and holds the new entry, and
/// replaces the file with it.
fn append(
    target: &KeyFileTarget<'_>,
    id: &str,
    add: impl FnOnce(&mut KeyFile, &SecretKey) -> bool,
) -> Result<(), Failure> {
    let operator = read_secret(target.operator)?.key;
    let operator_public = operator.public_key();
    let shown = target.file.display();
    let mut file = match fs::read_to_string(target.file) {
        Ok(text) => {
            KeyRing::parse(&text, &operator_public)
                .map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
            let value =
                json::parse(&text).map_err(|error| Failure::Local(format!("{shown}: {error}")))?;
            wire::decode_inbox_messaging_key_file(&value, "")
                .map_err(|error| Failure::Local(format!("{shown}: {error}")))?
        }
        Err(error) if error.kind() == ErrorKind::NotFound => KeyFile {
            keys: Vec::new(),
            delegations: Vec::new(),
        },
        Err(error) => return Err(Failure::Local(format!("{shown}: {error}"))),
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
    replace(target.file, text.as_bytes())
}

/// Replaces the file at `path` with `bytes`: written beside it, then renamed over it.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
    let shown = path.display();
    let mut beside = PathBuf::from(path);
    beside.as_mut_os_string().push(".new");
    let written = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&beside)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&beside, path)
    })();
    written.map_err(|error| Failure::Local(format!("{shown}: {error}")))
}
