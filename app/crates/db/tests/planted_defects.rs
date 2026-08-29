//! Test the testers, for the tenant boundary.
//!
//! The identifier parser is what stands between a request and a file outside
//! the data directory. These fixtures plant the two mistakes a permissive
//! parser actually makes and assert the real parser does not make them.

use app_db::AssociationId;

/// A validator that only rejects the literal string `".."`.
///
/// This is the shape of a real mistake: someone thinks about path traversal,
/// blocks the obvious token, and misses that `../etc` contains it without being
/// it — and that a slash alone is enough to leave the directory.
fn permissive_validator(candidate: &str) -> bool {
    candidate != ".."
}

/// Inputs that must never become an identifier, each with the reason.
const HOSTILE: &[(&str, &str)] = &[
    ("..", "the parent directory itself"),
    ("../escape", "traversal with a suffix"),
    ("../../etc/passwd", "repeated traversal"),
    ("/etc/passwd", "an absolute path"),
    ("a/b", "a path separator"),
    ("a\\b", "a Windows path separator"),
    (".hidden", "a leading dot"),
    (
        "a\0b",
        "an embedded null, which truncates in C string handling",
    ),
    (
        "Oakwood",
        "uppercase, which some filesystems fold onto lowercase",
    ),
    ("oakwo\u{00f6}d", "a non-ASCII character"),
];

#[test]
fn the_identifier_check_catches_a_permissive_validator() {
    let mut slipped_through = Vec::new();
    for (hostile, reason) in HOSTILE {
        if permissive_validator(hostile) {
            slipped_through.push((*hostile, *reason));
        }
    }

    assert!(
        !slipped_through.is_empty(),
        "the planted validator rejected everything, so this fixture is no \
         longer demonstrating anything"
    );

    // Everything the planted validator let through, the real one must refuse.
    for (hostile, reason) in slipped_through {
        assert!(
            AssociationId::parse(hostile).is_err(),
            "{hostile:?} ({reason}) was accepted as an association identifier; \
             a database path built from it would leave the data directory"
        );
    }
}

#[test]
fn every_hostile_input_is_refused_by_the_real_parser() {
    for (hostile, reason) in HOSTILE {
        assert!(
            AssociationId::parse(hostile).is_err(),
            "{hostile:?} ({reason}) must be refused"
        );
    }
}

#[test]
fn a_reserved_device_name_is_neutralised_by_the_prefix() {
    // "con", "prn", "nul", "com1" and friends are reserved device names on
    // Windows, and a bare file of that name behaves strangely. They are
    // deliberately *not* rejected as identifiers: every filename this system
    // builds is "assoc_<id>.db", and "assoc_con" is not a reserved name. The
    // prefix is what makes this safe, so this test pins the prefix rather than
    // adding a rejection rule that would only be cargo-culting a constraint we
    // do not have.
    for reserved in ["con", "prn", "aux", "nul", "com1", "lpt1"] {
        let id = AssociationId::parse(reserved).expect("accepted");
        let name = id.file_name();

        // The exact shape, not a prefix check: the prefix is the whole reason
        // this is safe, so it is what the test pins.
        assert_eq!(name, format!("assoc_{reserved}.db"));

        let stem = name.strip_suffix(".db").expect("suffix");
        assert_ne!(stem, reserved, "the stem must not be a bare device name");
    }
}

#[test]
fn a_filename_built_from_a_valid_identifier_stays_in_one_segment() {
    // The end-to-end consequence: whatever an identifier contains, the filename
    // derived from it is a single path component.
    let id = AssociationId::parse("oakwood-hills").expect("valid");
    let name = id.file_name();
    assert!(!name.contains('/'));
    assert!(!name.contains('\\'));
    assert!(!name.contains(".."));
    assert_eq!(std::path::Path::new(&name).components().count(), 1);
}
