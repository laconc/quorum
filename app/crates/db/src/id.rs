//! Association identifiers that cannot name a file outside the data directory.

use std::fmt;

/// The longest identifier accepted.
///
/// Long enough for a readable slug, short enough that a filename built from it
/// stays well inside every filesystem limit. Public because it is part of the
/// contract: whatever provisions an association needs to know what it may name
/// one.
pub const MAX_LEN: usize = 48;

/// Why a candidate identifier was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The identifier was empty.
    #[error("an association identifier cannot be empty")]
    Empty,

    /// The identifier was longer than [`MAX_LEN`] bytes.
    #[error("an association identifier may be at most {MAX_LEN} characters, got {len}")]
    TooLong {
        /// The rejected length.
        len: usize,
    },

    /// The identifier contained something other than `a-z`, `0-9`, `-`, or `_`.
    #[error(
        "an association identifier may contain only lowercase letters, digits, \
         hyphen and underscore; found {found:?} at byte {at}"
    )]
    IllegalCharacter {
        /// The offending character.
        found: char,
        /// Its byte offset.
        at: usize,
    },

    /// The identifier began or ended with a separator.
    #[error("an association identifier must start and end with a letter or digit")]
    BadBoundary,
}

/// A validated association identifier.
///
/// The only way to obtain one is [`AssociationId::parse`], and the only
/// legitimate inputs to that are values already inside our trust boundary: a
/// row from the platform database, or the association recorded on a session.
/// **Never parse one out of a path segment, a host header, a query parameter,
/// or a form field.** The host header in particular selects branding and
/// nothing else; it is never an authorization input.
///
/// The character set is deliberately narrow. Path traversal, absolute paths,
/// null bytes, and Unicode normalisation tricks are all excluded by
/// construction rather than by a filter that has to anticipate them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssociationId(String);

impl AssociationId {
    /// Validate a candidate identifier from a trusted source.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] describing the first problem found. Every
    /// rejection is a programming error or an attack; neither should be
    /// recovered from silently.
    ///
    /// # Examples
    ///
    /// ```
    /// use app_db::AssociationId;
    ///
    /// assert!(AssociationId::parse("oakwood-hills").is_ok());
    /// assert!(AssociationId::parse("../etc/passwd").is_err());
    /// assert!(AssociationId::parse("/absolute").is_err());
    /// ```
    pub fn parse(candidate: &str) -> Result<Self, IdError> {
        if candidate.is_empty() {
            return Err(IdError::Empty);
        }
        if candidate.len() > MAX_LEN {
            return Err(IdError::TooLong {
                len: candidate.len(),
            });
        }
        for (at, found) in candidate.char_indices() {
            let legal =
                found.is_ascii_lowercase() || found.is_ascii_digit() || matches!(found, '-' | '_');
            if !legal {
                return Err(IdError::IllegalCharacter { found, at });
            }
        }

        // A leading or trailing separator buys nothing and makes a filename
        // harder to reason about; requiring alphanumeric boundaries also rules
        // out the "-" and "_" identifiers outright.
        let first = candidate.as_bytes()[0];
        let last = candidate.as_bytes()[candidate.len() - 1];
        let alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
        if !alnum(first) || !alnum(last) {
            return Err(IdError::BadBoundary);
        }

        Ok(Self(candidate.to_owned()))
    }

    /// The identifier as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The filename this association's database lives under.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("assoc_{}.db", self.0)
    }
}

impl fmt::Display for AssociationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_reasonable_identifier() {
        let id = AssociationId::parse("oakwood-hills").expect("valid");
        assert_eq!(id.as_str(), "oakwood-hills");
        assert_eq!(id.file_name(), "assoc_oakwood-hills.db");
    }

    #[test]
    fn accepts_a_single_character() {
        assert!(AssociationId::parse("a").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(AssociationId::parse(""), Err(IdError::Empty));
    }

    #[test]
    fn rejects_traversal_and_separators() {
        // The cases that would let an identifier name a file outside the data
        // directory. Each must be refused before anything touches a filesystem.
        for hostile in [
            "..",
            "../etc/passwd",
            "..%2fetc",
            "/absolute",
            "a/b",
            "a\\b",
            ".hidden",
            "a.b",
            "~root",
            "a:b",
        ] {
            assert!(
                AssociationId::parse(hostile).is_err(),
                "{hostile:?} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_null_and_control_bytes() {
        assert!(AssociationId::parse("a\0b").is_err());
        assert!(AssociationId::parse("a\nb").is_err());
        assert!(AssociationId::parse("a\rb").is_err());
    }

    #[test]
    fn rejects_uppercase_and_unicode() {
        // Case folding and Unicode normalisation are both routes to two
        // identifiers that look distinct but name one file.
        assert!(AssociationId::parse("Oakwood").is_err());
        assert!(AssociationId::parse("oakwo\u{00f6}d").is_err());
        assert!(AssociationId::parse("oakwoo\u{0064}\u{0301}").is_err());
        assert!(AssociationId::parse("\u{fe0f}").is_err());
    }

    #[test]
    fn rejects_boundary_separators() {
        for hostile in ["-lead", "trail-", "_lead", "trail_", "-", "_"] {
            assert_eq!(
                AssociationId::parse(hostile),
                Err(IdError::BadBoundary),
                "{hostile:?} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_overlong() {
        let long = "a".repeat(MAX_LEN + 1);
        assert_eq!(
            AssociationId::parse(&long),
            Err(IdError::TooLong { len: MAX_LEN + 1 })
        );
        assert!(AssociationId::parse(&"a".repeat(MAX_LEN)).is_ok());
    }
}
