//! The connection factory.
//!
//! Every handle in the system is produced here, from a [`DbRoot`] that knows
//! the data directory and an [`AssociationId`] that has already been validated.
//! No other construction path exists.

use crate::id::AssociationId;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use std::path::{Path, PathBuf};

/// Milliseconds a writer waits for a competing writer before giving up.
///
/// A single instance with a handful of concurrent users will effectively never
/// reach this; it exists so that a slow write during a backup or a checkpoint
/// queues rather than failing.
const BUSY_TIMEOUT_MS: u32 = 5_000;

/// Something went wrong opening or checking a database.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// The data directory could not be prepared.
    #[error("could not prepare the data directory at {path}: {source}")]
    DataDirectory {
        /// The directory in question.
        path: PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },

    /// A connection pool could not be built.
    #[error("could not open a connection pool for {path}: {source}")]
    Pool {
        /// The database file.
        path: PathBuf,
        /// The underlying failure.
        source: r2d2::Error,
    },

    /// A statement failed.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// `PRAGMA integrity_check` reported a problem.
    #[error("integrity check failed for {path}: {report}")]
    IntegrityCheck {
        /// The database file.
        path: PathBuf,
        /// What SQLite reported.
        report: String,
    },
}

/// Apply the settings every connection in this system must have.
///
/// Three of these are security controls rather than tuning, and the comments
/// say which is which, because a future reader optimising this function needs
/// to know what is safe to change.
fn harden(conn: &Connection) -> Result<(), rusqlite::Error> {
    // --- Security controls. Do not relax. ---

    // Setting the attached-database limit to zero makes ATTACH fail. This is
    // the control that keeps one association's file unreachable from a
    // connection opened for another: without it, a single ATTACH statement
    // reduces physical file separation to a naming convention. There is no
    // legitimate use of ATTACH anywhere in this system.
    conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;

    // ATTACH is refused twice over. The limit above is the control; these two
    // deny creating and writing an attached file even if some future SQLite
    // reinterprets a zero limit. Cheap, and the failure they guard against is
    // one association's data reachable from another's connection.
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE, false)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE, false)?;

    // Extension loading — arbitrary code execution reachable from a SQL
    // injection — needs no call to disable here: rusqlite's `load_extension`
    // feature is deliberately not enabled, so the capability is not compiled
    // in, and SQLite's own default is off. Not depending on the feature is a
    // stronger guarantee than remembering to turn it off at runtime. If anyone
    // ever adds that feature to this crate's manifest, this comment is the
    // reason to ask why.

    // Defensive mode refuses direct writes to shadow tables and to the schema,
    // which is the difference between a SQL defect corrupting rows and one
    // corrupting the database's own structure.
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;

    // Double-quoted string literals are a SQLite compatibility misfeature: a
    // mistyped column name silently becomes a string instead of an error.
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DQS_DDL, false)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DQS_DML, false)?;

    // --- Operational settings. ---

    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Write-ahead logging: readers do not block the writer, and it is what
    // continuous replication reads from.
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;

    // NORMAL rather than FULL: with WAL, this trades a fsync per commit for the
    // possibility of losing the most recent transactions on an operating-system
    // crash — not on a process crash. That window is already accepted, and
    // named as the largest accepted risk in the design, because replication is
    // asynchronous; paying for FULL here would not close it.
    conn.pragma_update(None, "synchronous", "NORMAL")?;

    conn.busy_timeout(std::time::Duration::from_millis(u64::from(BUSY_TIMEOUT_MS)))?;

    Ok(())
}

fn build_pool(path: &Path) -> Result<Pool<SqliteConnectionManager>, DbError> {
    let manager = SqliteConnectionManager::file(path).with_init(|conn| harden(conn));
    Pool::builder()
        .max_size(8)
        .build(manager)
        .map_err(|source| DbError::Pool {
            path: path.to_path_buf(),
            source,
        })
}

fn integrity_check(pool: &Pool<SqliteConnectionManager>, path: &Path) -> Result<(), DbError> {
    let conn = pool.get().map_err(|source| DbError::Pool {
        path: path.to_path_buf(),
        source,
    })?;
    let report: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if report == "ok" {
        Ok(())
    } else {
        Err(DbError::IntegrityCheck {
            path: path.to_path_buf(),
            report,
        })
    }
}

/// The data directory, and the only thing that can hand out a database.
#[derive(Debug, Clone)]
pub struct DbRoot {
    dir: PathBuf,
}

impl DbRoot {
    /// Point at a data directory, creating it if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::DataDirectory`] if the directory cannot be created.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, DbError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|source| DbError::DataDirectory {
            path: dir.clone(),
            source,
        })?;
        Ok(Self { dir })
    }

    /// Open the platform database, which holds identity, sessions, the
    /// association registry, and cross-association membership.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] if the pool cannot be built.
    pub fn platform(&self) -> Result<PlatformDb, DbError> {
        let path = self.dir.join("platform.db");
        let pool = build_pool(&path)?;
        tracing::debug!(path = %path.display(), "opened platform database");
        Ok(PlatformDb { pool, path })
    }

    /// Open one association's database.
    ///
    /// The identifier must already have been validated, which means it came
    /// from a session or a platform-database row — never from a request.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] if the pool cannot be built.
    pub fn association(&self, id: &AssociationId) -> Result<AssocDb, DbError> {
        // The filename is derived from the validated identifier and nothing
        // else. There is no format string here taking caller-supplied text.
        let path = self.dir.join(id.file_name());
        let pool = build_pool(&path)?;
        tracing::debug!(association = %id, path = %path.display(), "opened association database");
        Ok(AssocDb {
            pool,
            path,
            id: id.clone(),
        })
    }
}

/// A handle to the platform database.
///
/// The underlying connection type is deliberately not reachable from outside
/// this crate. Typed access arrives in Phase 1 alongside the schema.
#[derive(Debug, Clone)]
pub struct PlatformDb {
    pool: Pool<SqliteConnectionManager>,
    path: PathBuf,
}

impl PlatformDb {
    /// Run `PRAGMA integrity_check`.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::IntegrityCheck`] if SQLite reports anything other
    /// than `ok`.
    pub fn integrity_check(&self) -> Result<(), DbError> {
        integrity_check(&self.pool, &self.path)
    }

    /// The file this handle is bound to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &Pool<SqliteConnectionManager> {
        &self.pool
    }
}

/// A handle to one association's database.
#[derive(Debug, Clone)]
pub struct AssocDb {
    pool: Pool<SqliteConnectionManager>,
    path: PathBuf,
    id: AssociationId,
}

impl AssocDb {
    /// The association this handle is bound to.
    #[must_use]
    pub fn id(&self) -> &AssociationId {
        &self.id
    }

    /// The file this handle is bound to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run `PRAGMA integrity_check`.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::IntegrityCheck`] if SQLite reports anything other
    /// than `ok`.
    pub fn integrity_check(&self) -> Result<(), DbError> {
        integrity_check(&self.pool, &self.path)
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &Pool<SqliteConnectionManager> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn root() -> (TempDir, DbRoot) {
        let dir = TempDir::new().expect("temp dir");
        let root = DbRoot::new(dir.path()).expect("root");
        (dir, root)
    }

    fn oakwood() -> AssociationId {
        AssociationId::parse("oakwood-hills").expect("valid")
    }

    #[test]
    fn opens_a_platform_and_an_association_database() {
        let (_dir, root) = root();
        let platform = root.platform().expect("platform");
        let assoc = root.association(&oakwood()).expect("association");

        platform.integrity_check().expect("platform integrity");
        assoc.integrity_check().expect("association integrity");
        assert_ne!(platform.path(), assoc.path());
    }

    #[test]
    fn attach_is_disabled() {
        // The control from D-02 and RT-08. If this test ever goes green by
        // being deleted, physical file separation stops meaning anything.
        let (dir, root) = root();
        let assoc = root.association(&oakwood()).expect("association");
        let other = dir.path().join("assoc_marina-point.db");

        let conn = assoc.pool().get().expect("connection");
        let err = conn
            .execute_batch(&format!("ATTACH DATABASE '{}' AS other", other.display()))
            .expect_err("ATTACH must be refused");

        assert!(
            err.to_string().contains("too many attached databases")
                || err.to_string().contains("not authorized"),
            "unexpected refusal: {err}"
        );
    }

    #[test]
    fn attach_of_an_in_memory_database_is_also_disabled() {
        // ':memory:' does not name a file, so a limit implemented as a path
        // check rather than a real limit would let it through.
        let (_dir, root) = root();
        let assoc = root.association(&oakwood()).expect("association");
        let conn = assoc.pool().get().expect("connection");
        assert!(
            conn.execute_batch("ATTACH DATABASE ':memory:' AS scratch")
                .is_err(),
            "ATTACH of an in-memory database must be refused too"
        );
    }

    #[test]
    fn every_connection_in_the_pool_is_hardened() {
        // Holding the first connection forces the pool to build a second rather
        // than hand back the same one. That every connection gets the init hook
        // is r2d2's contract, not ours — and a connection that quietly missed
        // it would be indistinguishable until the day it mattered.
        let (_dir, root) = root();
        let assoc = root.association(&oakwood()).expect("association");

        let first = assoc.pool().get().expect("first connection");
        let second = assoc.pool().get().expect("second connection");

        for (label, conn) in [("first", &first), ("second", &second)] {
            assert!(
                conn.execute_batch("ATTACH DATABASE ':memory:' AS scratch")
                    .is_err(),
                "{label} connection allowed ATTACH"
            );
            let mode: String = conn
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .expect("journal_mode");
            assert_eq!(mode.to_lowercase(), "wal", "{label} connection");
        }
    }

    #[test]
    fn write_ahead_logging_is_on() {
        let (_dir, root) = root();
        let platform = root.platform().expect("platform");
        let conn = platform.pool().get().expect("connection");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal_mode");
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let (_dir, root) = root();
        let assoc = root.association(&oakwood()).expect("association");
        let conn = assoc.pool().get().expect("connection");
        let on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign_keys");
        assert_eq!(on, 1);

        conn.execute_batch(
            "CREATE TABLE parent (id INTEGER PRIMARY KEY) STRICT;
             CREATE TABLE child (id INTEGER PRIMARY KEY,
                                 parent_id INTEGER NOT NULL REFERENCES parent(id)) STRICT;",
        )
        .expect("schema");
        assert!(
            conn.execute("INSERT INTO child (id, parent_id) VALUES (1, 999)", [])
                .is_err(),
            "a dangling foreign key must be refused"
        );
    }

    #[test]
    fn double_quoted_string_literals_are_refused() {
        // With this misfeature on, a mistyped column name silently becomes a
        // string literal instead of an error.
        let (_dir, root) = root();
        let assoc = root.association(&oakwood()).expect("association");
        let conn = assoc.pool().get().expect("connection");
        conn.execute_batch("CREATE TABLE t (a TEXT) STRICT;")
            .expect("schema");
        assert!(
            conn.execute("INSERT INTO t (a) VALUES (\"not_a_column\")", [])
                .is_err(),
            "a double-quoted literal must be refused, not silently accepted"
        );
    }

    #[test]
    fn each_association_gets_its_own_file() {
        let (_dir, root) = root();
        let a = root.association(&oakwood()).expect("a");
        let b = root
            .association(&AssociationId::parse("marina-point").expect("valid"))
            .expect("b");
        assert_ne!(a.path(), b.path());
        assert!(a.path().ends_with("assoc_oakwood-hills.db"));
        assert!(b.path().ends_with("assoc_marina-point.db"));
    }

    #[test]
    fn a_hostile_identifier_never_reaches_the_filesystem() {
        // Rejection happens in the identifier parser, before a path is built.
        // This test states the end-to-end consequence: there is no string a
        // caller can supply that names a file outside the data directory.
        let (dir, root) = root();
        for hostile in ["../escape", "/etc/passwd", "a/b", "..", "a\0b"] {
            assert!(
                AssociationId::parse(hostile).is_err(),
                "{hostile:?} must never become an identifier"
            );
        }
        drop(root);
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read data dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert!(
            entries.iter().all(|name| {
                let name = name.to_string_lossy();
                name.starts_with("platform.db") || name.starts_with("assoc_")
            }),
            "unexpected files in the data directory: {entries:?}"
        );
    }
}
