//! Hashes that remember which algorithm produced them.

// This crate is the one place permitted to name a hash type directly; the
// lint that forbids it everywhere else is what keeps every digest paired
// with its algorithm. See the crate documentation for why that pairing is
// the point.
#![allow(clippy::disallowed_types)]

use sha2::{Digest as _, Sha384};
use std::fmt;

/// A hash algorithm.
///
/// One variant today. The enum exists so that adding a second is a migration
/// with a compiler-checked exhaustiveness failure at every dispatch site,
/// rather than a search for hard-coded assumptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HashAlg {
    /// SHA-384.
    ///
    /// Chosen over SHA-256 deliberately. Grover's algorithm halves the
    /// effective preimage strength of a hash, taking SHA-256 to roughly 128
    /// bits — adequate by most measures, but the audit chain is a statutory
    /// record with a multi-year retention period, and rehashing a chain after
    /// the fact means re-anchoring everything that was ever published. The
    /// wider digest is close to free today and expensive to retrofit later.
    Sha384,
}

impl HashAlg {
    /// The algorithm's stable identifier, as stored beside every artifact.
    ///
    /// This string is written to the database and must never change for an
    /// existing variant; a new algorithm gets a new string.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Sha384 => "SHA-384",
        }
    }

    /// Parse a stored identifier.
    ///
    /// # Errors
    ///
    /// Returns `None` for an identifier this build does not recognise, which
    /// means a record written by a newer version. The caller must treat that as
    /// "cannot verify", never as "verified".
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "SHA-384" => Some(Self::Sha384),
            _ => None,
        }
    }

    /// The digest length in bytes.
    #[must_use]
    pub const fn digest_len(self) -> usize {
        match self {
            Self::Sha384 => 48,
        }
    }
}

impl fmt::Display for HashAlg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A hash, together with the algorithm that produced it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Digest {
    alg: HashAlg,
    bytes: Vec<u8>,
}

impl Digest {
    /// The algorithm that produced this digest.
    #[must_use]
    pub const fn alg(&self) -> HashAlg {
        self.alg
    }

    /// The raw digest bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The digest as lowercase hex, for storage and display.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Rebuild a digest from its stored algorithm identifier and hex bytes.
    ///
    /// # Errors
    ///
    /// Returns `None` if the algorithm is unrecognised, the hex is malformed,
    /// or the length does not match the algorithm. All three mean the stored
    /// record cannot be verified by this build.
    #[must_use]
    pub fn from_stored(alg_id: &str, hex_bytes: &str) -> Option<Self> {
        let alg = HashAlg::from_id(alg_id)?;
        let bytes = hex::decode(hex_bytes).ok()?;
        (bytes.len() == alg.digest_len()).then_some(Self { alg, bytes })
    }
}

// Hand-written so a digest cannot be confused with arbitrary bytes in a log,
// and so the algorithm travels with it everywhere it is printed.
impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.alg, self.to_hex())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.alg, self.to_hex())
    }
}

/// Hash `input` with the workspace's current algorithm.
///
/// # Examples
///
/// ```
/// use app_crypto::{HashAlg, hash};
///
/// let d = hash(b"lot 42");
/// assert_eq!(d.alg(), HashAlg::Sha384);
/// assert_eq!(d.bytes().len(), 48);
/// ```
#[must_use]
pub fn hash(input: &[u8]) -> Digest {
    let mut hasher = Sha384::new();
    hasher.update(input);
    Digest {
        alg: HashAlg::Sha384,
        bytes: hasher.finalize().to_vec(),
    }
}

/// Compute one link of an append-only hash chain.
///
/// The link commits to both the previous link and the canonical encoding of
/// this entry, so altering any earlier entry invalidates every link after it.
/// `prev` is `None` only for the genesis entry.
///
/// The previous digest is bound by its full display form — algorithm and bytes
/// — so that an entry cannot be replayed under a different algorithm than the
/// one it was chained with.
///
/// # Examples
///
/// ```
/// use app_crypto::hash_chain_link;
///
/// let genesis = hash_chain_link(None, b"first");
/// let second = hash_chain_link(Some(&genesis), b"second");
/// assert_ne!(genesis.to_hex(), second.to_hex());
/// ```
#[must_use]
pub fn hash_chain_link(prev: Option<&Digest>, canonical_entry: &[u8]) -> Digest {
    let mut hasher = Sha384::new();
    match prev {
        Some(prev) => hasher.update(prev.to_string().as_bytes()),
        None => hasher.update(b"GENESIS"),
    }
    hasher.update(b"\x00");
    hasher.update(canonical_entry);
    Digest {
        alg: HashAlg::Sha384,
        bytes: hasher.finalize().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_carries_its_algorithm() {
        let d = hash(b"anything");
        assert_eq!(d.alg(), HashAlg::Sha384);
        assert_eq!(d.bytes().len(), HashAlg::Sha384.digest_len());
    }

    #[test]
    fn digest_round_trips_through_storage() {
        let d = hash(b"anything");
        let restored = Digest::from_stored(d.alg().id(), &d.to_hex()).expect("round trip");
        assert_eq!(d, restored);
    }

    #[test]
    fn unknown_algorithm_cannot_be_verified() {
        // A record written by a newer build must read as "cannot verify",
        // never as "verified".
        assert!(Digest::from_stored("SHA-512", &"ab".repeat(64)).is_none());
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(Digest::from_stored("SHA-384", "abcd").is_none());
    }

    #[test]
    fn chain_links_depend_on_their_predecessor() {
        let a = hash_chain_link(None, b"entry");
        let b = hash_chain_link(None, b"different");
        assert_ne!(a, b);

        let from_a = hash_chain_link(Some(&a), b"same");
        let from_b = hash_chain_link(Some(&b), b"same");
        assert_ne!(
            from_a, from_b,
            "identical entries under different predecessors must not collide"
        );
    }

    #[test]
    fn genesis_is_distinguishable_from_a_link() {
        let genesis = hash_chain_link(None, b"entry");
        let linked = hash_chain_link(Some(&genesis), b"entry");
        assert_ne!(genesis, linked);
    }

    #[test]
    #[ignore = "Phase 1: ML-DSA-87 (FIPS 204) signing over daily audit roots, export bundles, and certified tallies. Implementation selected in the Phase 1 spike (open item O-8) against recorded criteria: FIPS 204 conformance, audit or formal-verification status, maintenance posture, licence, presence in the existing dependency graph, and a clean build on both CI architectures."]
    fn ml_dsa_signs_and_verifies_a_daily_root() {}

    #[test]
    #[ignore = "Phase 1: verification must reject a signature whose stored algorithm identifier does not match the key that produced it, so an algorithm downgrade cannot pass as valid."]
    fn ml_dsa_rejects_algorithm_downgrade() {}
}
