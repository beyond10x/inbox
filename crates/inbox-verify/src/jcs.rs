//! The JSON Canonicalization Scheme (RFC 8785) over the generated JSON value type.
//!
//! Members are ordered by the UTF-16 code units of their names, strings are escaped as ECMAScript
//! `JSON.stringify` escapes them, and numbers are written as ECMAScript writes an IEEE 754 double.

use std::fmt;

use inbox::server::json::Value;

/// A value RFC 8785 cannot canonicalize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonError {
    /// A number that is not JSON, or not a finite IEEE 754 double.
    Number(String),
    /// An object carrying one member name twice (I-JSON forbids it).
    DuplicateMember(String),
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(spelling) => write!(f, "`{spelling}` is not a finite JSON number"),
            Self::DuplicateMember(name) => write!(f, "the member `{name}` appears twice"),
        }
    }
}

impl std::error::Error for CanonError {}

/// The canonical text of `value`.
///
/// # Errors
///
/// [`CanonError`] for a number that is not a finite double, or an object with a duplicate member.
pub fn canonicalize(value: &Value) -> Result<String, CanonError> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, out: &mut String) -> Result<(), CanonError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(spelling) => out.push_str(&number(spelling)?),
        Value::Text(text) => write_text(text, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut sorted: Vec<&(String, Value)> = members.iter().collect();
            sorted.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            if let Some(pair) = sorted.windows(2).find(|pair| pair[0].0 == pair[1].0) {
                return Err(CanonError::DuplicateMember(pair[0].0.clone()));
            }
            out.push('{');
            for (index, (name, item)) in sorted.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_text(name, out);
                out.push(':');
                write_value(item, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_text(text: &str, out: &mut String) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// The canonical spelling of a JSON number: the ECMAScript `Number::toString` of the double it
/// denotes.
///
/// # Errors
///
/// [`CanonError::Number`] when the spelling is not a JSON number or overflows a double.
pub fn number(spelling: &str) -> Result<String, CanonError> {
    if !json_number(spelling) {
        return Err(CanonError::Number(spelling.to_owned()));
    }
    let value: f64 = spelling
        .parse()
        .map_err(|_| CanonError::Number(spelling.to_owned()))?;
    double(value).ok_or_else(|| CanonError::Number(spelling.to_owned()))
}

/// The ECMAScript `Number::toString` of a finite double; `None` for NaN or an infinity.
pub fn double(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    if value == 0.0 {
        return Some("0".to_owned());
    }
    // Rust writes the shortest digits that read back as the same double, closest first: the
    // digits ECMAScript asks for. `{:e}` gives them as `d.ddde±x`.
    let (digits, exponent) = scientific(&format!("{:e}", value.abs()))?;
    let digits = tie_to_even(value.abs(), digits, exponent);
    let k = i32::try_from(digits.len()).ok()?;
    let n = exponent + 1;
    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.push_str(&"0".repeat(usize::try_from(n - k).ok()?));
    } else if 0 < n && n <= 21 {
        let split = usize::try_from(n).ok()?;
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat(usize::try_from(-n).ok()?));
        out.push_str(&digits);
    } else {
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if n - 1 < 0 { '-' } else { '+' });
        out.push_str(&(n - 1).abs().to_string());
    }
    Some(out)
}

/// The significant digits and the decimal exponent of Rust's `{:e}` text, `d.ddde±x`.
fn scientific(text: &str) -> Option<(String, i32)> {
    let (mantissa, exponent) = text.split_once('e')?;
    let digits = mantissa.chars().filter(char::is_ascii_digit).collect();
    Some((digits, exponent.parse().ok()?))
}

/// ECMAScript breaks a tie between two shortest digit strings, equally close to the double, to the
/// one whose last digit is even (ECMA-262, `Number::toString`, note 2); Rust's shortest formatting
/// does not promise which it takes. A tie exists only when the double is exactly the midpoint: its
/// exact decimal expansion is the shorter digits followed by one `5`.
fn tie_to_even(value: f64, digits: String, exponent: i32) -> String {
    if !digits.ends_with(['1', '3', '5', '7', '9']) {
        return digits;
    }
    // 801 digits hold the exact expansion of every double (at most 767 significant digits).
    let Some((exact, exact_exponent)) = scientific(&format!("{value:.800e}")) else {
        return digits;
    };
    let exact = exact.trim_end_matches('0');
    if exact_exponent != exponent || exact.len() != digits.len() + 1 || !exact.ends_with('5') {
        return digits;
    }
    let lower = &exact[..digits.len()];
    let Some(upper) = increment(lower) else {
        return digits;
    };
    let scale = i32::try_from(digits.len()).map_or(exponent, |count| exponent - (count - 1));
    let reads_back = |candidate: &str| {
        format!("{candidate}e{scale}")
            .parse::<f64>()
            .is_ok_and(|read| read == value)
    };
    [lower.to_owned(), upper]
        .into_iter()
        .find(|candidate| candidate.ends_with(['0', '2', '4', '6', '8']) && reads_back(candidate))
        .unwrap_or(digits)
}

/// The decimal digits one greater, when that keeps their count.
fn increment(digits: &str) -> Option<String> {
    let mut bytes = digits.as_bytes().to_vec();
    for byte in bytes.iter_mut().rev() {
        if *byte == b'9' {
            *byte = b'0';
        } else {
            *byte += 1;
            return String::from_utf8(bytes).ok();
        }
    }
    None
}

/// Whether `text` is spelled as RFC 8259 spells a number.
fn json_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    if bytes.get(at) == Some(&b'-') {
        at += 1;
    }
    match bytes.get(at) {
        Some(b'0') => at += 1,
        Some(b'1'..=b'9') => {
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
        _ => return false,
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return false;
        }
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return false;
        }
    }
    at == bytes.len()
}
