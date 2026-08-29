//! A deterministic JSON encoding, for anything that gets hashed.
//!
//! This is a strict subset of RFC 8785 (JSON Canonicalization Scheme): object
//! keys are sorted by UTF-16 code unit, there is no insignificant whitespace,
//! and strings use the minimal escape set. It differs from RFC 8785 in one
//! deliberate way — **floating-point numbers are rejected rather than
//! serialised**.
//!
//! That rejection is the point. RFC 8785's hardest requirement is reproducing
//! ECMAScript's `Number::toString` for doubles, which is where independent
//! implementations disagree. This system has no legitimate use for a float in a
//! hashed artifact: money is `i64` cents, fractional shares are integer
//! numerator/denominator pairs, and timestamps are integers. Refusing floats
//! removes the only genuinely subtle part of the specification and turns a
//! whole class of "the chain does not verify on the other machine" into a typed
//! error at the point of encoding.
//!
//! Determinism here is load-bearing: a non-deterministic encoding makes the
//! audit hash chain decorative rather than evidential, so this module is
//! property-tested rather than merely unit-tested.

use serde_json::Value;
use std::fmt::Write as _;

/// Why a value could not be canonically encoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalError {
    /// A floating-point number appeared in a value that will be hashed.
    ///
    /// Represent the quantity as an integer instead: money as minor units, a
    /// ratio as a numerator and denominator, a timestamp as seconds.
    #[error(
        "floating-point numbers cannot be canonically encoded (found at {path}); \
         represent the value as an integer — money as cents, a ratio as a \
         numerator/denominator pair, a timestamp as seconds"
    )]
    FloatNotPermitted {
        /// Where in the value the float appeared, as a slash-separated path.
        path: String,
    },
}

/// Encode `value` canonically.
///
/// # Errors
///
/// Returns [`CanonicalError::FloatNotPermitted`] if the value contains a
/// floating-point number anywhere.
///
/// # Examples
///
/// Key order in the input does not affect the output:
///
/// ```
/// use app_crypto::canonicalize;
/// use serde_json::json;
///
/// let a = canonicalize(&json!({"b": 1, "a": 2}))?;
/// let b = canonicalize(&json!({"a": 2, "b": 1}))?;
/// assert_eq!(a, b);
/// assert_eq!(String::from_utf8(a).unwrap(), r#"{"a":2,"b":1}"#);
/// # Ok::<(), app_crypto::CanonicalError>(())
/// ```
pub fn canonicalize(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let mut out = String::new();
    write_value(value, &mut out, "")?;
    Ok(out.into_bytes())
}

fn write_value(value: &Value, out: &mut String, path: &str) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let _ = write!(out, "{i}");
            } else if let Some(u) = n.as_u64() {
                let _ = write!(out, "{u}");
            } else {
                return Err(CanonicalError::FloatNotPermitted {
                    path: if path.is_empty() {
                        "/".to_owned()
                    } else {
                        path.to_owned()
                    },
                });
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out, &format!("{path}/{i}"))?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // RFC 8785 orders keys by UTF-16 code unit, which is not the same
            // as Rust's UTF-8 byte ordering above the basic multilingual plane:
            // a surrogate pair sorts before U+E000..U+FFFF in UTF-16 but after
            // it in UTF-8. Compare the encoded code units directly so the
            // ordering is right for every input, not just ASCII ones.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by_cached_key(|k| utf16_units(k));

            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                let child = format!("{path}/{key}");
                write_value(&map[key.as_str()], out, &child)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn utf16_units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn encode(v: &Value) -> String {
        String::from_utf8(canonicalize(v).expect("encodes")).expect("utf-8")
    }

    #[test]
    fn key_order_does_not_matter() {
        assert_eq!(
            encode(&json!({"b": 1, "a": 2})),
            encode(&json!({"a": 2, "b": 1}))
        );
    }

    #[test]
    fn no_insignificant_whitespace() {
        assert_eq!(
            encode(&json!({"a": [1, 2], "b": {}})),
            r#"{"a":[1,2],"b":{}}"#
        );
    }

    #[test]
    fn array_order_is_preserved() {
        assert_ne!(encode(&json!([1, 2])), encode(&json!([2, 1])));
    }

    #[test]
    fn control_characters_use_the_minimal_escape_set() {
        // Newline and tab get their short forms; every other character
        // below U+0020 gets the six-character escape, in lowercase hex.
        assert_eq!(encode(&json!("a\nb\tc\u{1}")), r#""a\nb\tc\u0001""#);
        assert_eq!(encode(&json!("\u{8}\u{c}\r")), r#""\b\f\r""#);
        assert_eq!(encode(&json!("\u{1f}")), r#""\u001f""#);
    }

    #[test]
    fn quotes_and_backslashes_are_escaped() {
        assert_eq!(encode(&json!(r#"a"b\c"#)), r#""a\"b\\c""#);
    }

    #[test]
    fn floats_are_rejected_with_their_location() {
        let err = canonicalize(&json!({"amount": {"total": 1.5}})).unwrap_err();
        let CanonicalError::FloatNotPermitted { path } = err;
        assert_eq!(path, "/amount/total");
    }

    #[test]
    fn large_integers_survive() {
        assert_eq!(encode(&json!(i64::MIN)), i64::MIN.to_string());
        assert_eq!(encode(&json!(u64::MAX)), u64::MAX.to_string());
    }

    #[test]
    fn keys_sort_by_utf16_not_utf8() {
        // U+FF3A (fullwidth Z) is one UTF-16 unit, 0xFF3A. U+10000 is a
        // surrogate pair beginning 0xD800, so it sorts *first* in UTF-16 and
        // *last* in UTF-8. This is the case that separates the two orderings,
        // and getting it wrong would make the chain verify differently for any
        // record containing an astral character.
        let fullwidth = '\u{ff3a}'.to_string();
        let astral = '\u{10000}'.to_string();
        let encoded = encode(&json!({ &fullwidth: 1, &astral: 2 }));
        let fullwidth_at = encoded.find(&fullwidth).expect("fullwidth key present");
        let astral_at = encoded.find(&astral).expect("astral key present");
        assert!(
            astral_at < fullwidth_at,
            "UTF-16 ordering puts the surrogate pair first; got {encoded}"
        );
    }
}
