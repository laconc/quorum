//! Message authentication, for artifacts we issue and later verify ourselves.
//!
//! Used by signed URLs for member-only files and by reply-by-email thread
//! tokens. Both are values we hand out and must recognise on return; neither
//! needs a third party to verify them, so a symmetric construction is the right
//! tool.
//!
//! HMAC over SHA-384 is quantum-resistant as it stands: Grover's algorithm
//! offers only a square-root speedup against a symmetric primitive, which
//! leaves ample margin at this width. Unlike the signature surface, nothing
//! here needs to migrate.

// As in `digest`, this crate is the one place permitted to name the
// underlying primitive directly.
#![allow(clippy::disallowed_types)]

use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha384;
use std::fmt;
use subtle::ConstantTimeEq;

type HmacSha384 = Hmac<Sha384>;

/// A message authentication code, with the algorithm that produced it.
#[derive(Clone, PartialEq, Eq)]
pub struct Mac {
    alg: &'static str,
    bytes: Vec<u8>,
}

impl Mac {
    /// The algorithm identifier, as stored beside the artifact.
    #[must_use]
    pub const fn alg(&self) -> &'static str {
        self.alg
    }

    /// The raw bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Lowercase hex, for embedding in a URL or a token.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }
}

// Never print the tag itself: a code that leaks into a log is a code an
// attacker can replay for as long as it remains valid.
impl fmt::Debug for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mac({}, <redacted>)", self.alg)
    }
}

/// Authenticate `message` under `key`.
///
/// # Panics
///
/// Does not panic in practice: HMAC is defined for a key of any length, so
/// the keying step cannot fail. The `expect` is there because the underlying
/// API is generic over constructions that do have a key-size restriction.
///
/// # Examples
///
/// ```
/// use app_crypto::{hmac_sha384, verify_mac};
///
/// let tag = hmac_sha384(b"secret key material", b"lot=42&expires=1772000000");
/// assert!(verify_mac(&tag, b"secret key material", b"lot=42&expires=1772000000"));
/// assert!(!verify_mac(&tag, b"secret key material", b"lot=43&expires=1772000000"));
/// ```
#[must_use]
pub fn hmac_sha384(key: &[u8], message: &[u8]) -> Mac {
    let mut mac = HmacSha384::new_from_slice(key).expect("HMAC accepts a key of any length");
    mac.update(message);
    Mac {
        alg: "HMAC-SHA-384",
        bytes: mac.finalize().into_bytes().to_vec(),
    }
}

/// Check a code in constant time.
///
/// Comparing tags with `==` leaks, through timing, how many leading bytes
/// matched — which is enough to forge one byte at a time. This is the only
/// comparison that should ever be used on a code.
#[must_use]
pub fn verify_mac(tag: &Mac, key: &[u8], message: &[u8]) -> bool {
    let expected = hmac_sha384(key, message);
    if tag.alg != expected.alg {
        return false;
    }
    expected.bytes.ct_eq(&tag.bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"a key that is long enough to be realistic";

    #[test]
    fn a_code_verifies_against_its_own_message() {
        let tag = hmac_sha384(KEY, b"message");
        assert!(verify_mac(&tag, KEY, b"message"));
    }

    #[test]
    fn a_changed_message_fails() {
        let tag = hmac_sha384(KEY, b"message");
        assert!(!verify_mac(&tag, KEY, b"messagf"));
    }

    #[test]
    fn a_changed_key_fails() {
        let tag = hmac_sha384(KEY, b"message");
        assert!(!verify_mac(&tag, b"a different key entirely", b"message"));
    }

    #[test]
    fn the_code_is_the_expected_width() {
        assert_eq!(hmac_sha384(KEY, b"message").bytes().len(), 48);
    }

    #[test]
    fn debug_does_not_leak_the_code() {
        let tag = hmac_sha384(KEY, b"message");
        let rendered = format!("{tag:?}");
        assert!(rendered.contains("redacted"));
        assert!(!rendered.contains(&tag.to_hex()));
    }
}
