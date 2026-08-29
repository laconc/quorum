//! Hashing, message authentication, canonical encoding, and token generation.
//!
//! This is the only crate in the workspace permitted to construct a hash or a
//! message authentication code. Everything else calls in here, and a lint
//! enforces it (see `app/clippy.toml`).
//!
//! # Why a crate rather than a function call
//!
//! Two reasons, and the second is the one that matters in ten years.
//!
//! **Algorithms are recorded, not assumed.** Every value this crate produces
//! carries the algorithm that produced it — [`Digest`] is `{alg, bytes}`, not a
//! bare byte array. Verification reads the stored algorithm and dispatches on
//! it. The records this system writes today have a seven-year retention period,
//! which is longer than the expected service life of any particular algorithm
//! choice. A hash chain whose links do not say which hash produced them is a
//! chain with an expiry date.
//!
//! **One place to change.** When an algorithm is retired, the migration is a
//! new enum variant and a dispatch arm, not an archaeological survey of every
//! call site.
//!
//! # What is here today
//!
//! - [`Digest`] and [`HashAlg`] — SHA-384, with room for successors.
//! - [`hash_chain_link`] — the audit chain's one primitive.
//! - [`Mac`] and [`hmac_sha384`] — for signed URLs and email thread tokens.
//! - [`Token`] and [`random_token`] — 256-bit opaque tokens.
//! - [`canonicalize`] — a deterministic JSON encoding, property-tested.
//!
//! Post-quantum signatures (ML-DSA-87, FIPS 204) over daily audit roots, export
//! bundles, and certified election results are declared as ignored tests here
//! and implemented in Phase 1.

pub mod canonical;
pub mod digest;
pub mod mac;
pub mod token;

pub use canonical::{CanonicalError, canonicalize};
pub use digest::{Digest, HashAlg, hash, hash_chain_link};
pub use mac::{Mac, hmac_sha384, verify_mac};
pub use token::{DEFAULT_TOKEN_BYTES, Token, random_token};
