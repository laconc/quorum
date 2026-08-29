//! Database handles, and the tenant boundary.
//!
//! # Why this crate exists
//!
//! SQLite enforces no tenant boundary of its own. The isolation this system
//! promises comes from one association's data living in a file that a request
//! scoped to another association never opens — and that guarantee is only worth
//! anything if there is exactly one way to open a file.
//!
//! So this crate is the only place in the workspace that depends on `rusqlite`,
//! and the connection type never leaves it. A handler cannot open a database,
//! because it has nothing to open one with. That turns "always go through the
//! connection factory" from a rule people have to remember into a fact about
//! what compiles.
//!
//! There is a test that enforces the arrangement: see
//! `tests/handle_containment.rs`, which fails the build if any other crate
//! takes a dependency on `rusqlite`.
//!
//! # What every connection gets
//!
//! - Write-ahead logging, foreign keys on, a busy timeout.
//! - **`ATTACH` disabled**, by setting the attached-database limit to zero.
//!   Without this, a single `ATTACH` turns file separation into a suggestion.
//! - **Extension loading disabled**, which would otherwise be arbitrary code
//!   execution reachable from SQL.
//! - **Defensive mode**, which refuses direct writes to shadow tables and to the
//!   schema.
//!
//! # Where identifiers come from
//!
//! [`AssociationId`] cannot be built from a request. It is parsed from a
//! trusted source — a session record or a row already read from the platform
//! database — and the parser rejects anything that could escape the data
//! directory. Phase 1 adds the session-resolution path that produces one.

pub mod id;
pub mod open;

pub use id::{AssociationId, IdError, MAX_LEN};
pub use open::{AssocDb, DbError, DbRoot, PlatformDb};
