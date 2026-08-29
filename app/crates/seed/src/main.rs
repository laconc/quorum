//! Generate the seed databases.
//!
//! One generator, three consumers: integration tests, the isolation harness,
//! and the screenshot suite. They share a generator so that the demonstration
//! data and the security fixture cannot drift apart — the two-association
//! fixture the isolation sweep needs is the same pair the screenshots are taken
//! against.
//!
//! Determinism is a requirement, not a nicety. The screenshot pipeline compares
//! images byte-for-byte across runs, so identifiers, ordering, and content must
//! be identical every time. Nothing here may draw from a random source or read
//! a clock.
//!
//! Phase 0 emits a valid, empty database pair — enough to prove the pipeline
//! end to end. Phase 2 fills it: two associations, the personas, and the
//! ownership edge cases that make the rights model visible.

use anyhow::{Context as _, Result};
use app_db::{AssociationId, DbRoot};
use std::path::PathBuf;

/// The associations every scenario contains.
///
/// Oakwood Hills is the established association most screenshots use. Marina
/// Point was provisioned last week and is nearly empty, which is how the zero
/// states get exercised rather than imagined — a newly provisioned association
/// must read as "not yet", never as "broken".
const ASSOCIATIONS: [&str; 2] = ["oakwood-hills", "marina-point"];

fn main() -> Result<()> {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data".to_owned())
        .into();

    let root = DbRoot::new(&dir).context("preparing the data directory")?;

    let platform = root.platform().context("opening the platform database")?;
    platform
        .integrity_check()
        .context("checking the platform database")?;
    println!("seeded {}", platform.path().display());

    for slug in ASSOCIATIONS {
        let id = AssociationId::parse(slug).context("association identifier")?;
        let assoc = root
            .association(&id)
            .with_context(|| format!("opening the database for {slug}"))?;
        assoc
            .integrity_check()
            .with_context(|| format!("checking the database for {slug}"))?;
        println!("seeded {}", assoc.path().display());
    }

    Ok(())
}
