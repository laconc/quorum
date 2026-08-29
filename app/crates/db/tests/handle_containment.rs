//! The structural half of tenant isolation.
//!
//! `app-db` is the only crate permitted to depend on `rusqlite`. That is what
//! makes "every handle comes from the connection factory" a fact about the
//! module graph rather than a rule someone has to remember while writing a
//! handler. A new crate that reaches for `rusqlite` directly fails this test,
//! which is the moment to ask why it needs one.

use std::fs;
use std::path::Path;

#[test]
fn no_crate_other_than_db_depends_on_rusqlite() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory");

    let mut offenders = Vec::new();
    for entry in fs::read_dir(crates_dir).expect("read crates directory") {
        let entry = entry.expect("directory entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "db" {
            continue;
        }
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("read manifest");
        if text.contains("rusqlite") {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "these crates reach for rusqlite directly, bypassing the connection \
         factory: {offenders:?}. Tenant isolation depends on there being one \
         way to open a database; add the access you need to app-db instead."
    );
}

#[test]
fn the_workspace_manifest_lists_members_explicitly() {
    // A glob would let a crate join the workspace without anyone noticing,
    // including one that bypasses the factory.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("Cargo.toml");
    let text = fs::read_to_string(&workspace).expect("read workspace manifest");
    assert!(
        !text.contains("crates/*"),
        "workspace members must be listed explicitly, not globbed"
    );
}
