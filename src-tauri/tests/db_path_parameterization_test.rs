//! Property-based test: Database path parameterization.
//!
//! **Feature: web-server-mode, Property 9: Database path parameterization**
//! *For any* valid filesystem path, `DbPool::open(path)` SHALL create a working SQLite database
//! at that location with WAL mode enabled, making no hardcoded assumptions about the path.
//!
//! **Validates: Requirements 8.3**

use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

use stockenboard_lib::db::DbPool;

/// Generate a single valid directory name segment (alphanumeric, 1-12 chars).
fn path_segment_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,12}".prop_map(|s| s)
}

/// Generate a vector of 1-4 nested path segments to form a subdirectory chain.
fn nested_path_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(path_segment_strategy(), 1..=4)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 8.3**
    ///
    /// For any valid filesystem path composed of random alphanumeric segments,
    /// `DbPool::open(path)` must create a working SQLite database at that location
    /// with WAL mode enabled.
    #[test]
    fn db_opens_at_any_valid_path_with_wal_mode(
        segments in nested_path_strategy(),
        db_name in "[a-zA-Z0-9]{3,10}"
    ) {
        // Create a temp directory as the root
        let tmp = TempDir::new().unwrap();

        // Build a nested subdirectory path from the generated segments
        let mut nested_dir = tmp.path().to_path_buf();
        for seg in &segments {
            nested_dir.push(seg);
        }
        std::fs::create_dir_all(&nested_dir).unwrap();

        // Build the database file path
        let db_filename = format!("{}.db", db_name);
        let db_path: PathBuf = nested_dir.join(&db_filename);

        // Open the database — this should succeed for any valid path
        let pool = DbPool::open(&db_path).expect("DbPool::open should succeed for any valid path");

        // 1. Verify the database file was created at the specified path
        assert!(
            db_path.exists(),
            "Database file should exist at {:?}",
            db_path
        );

        // 2. Verify WAL mode is enabled by querying PRAGMA journal_mode
        // Confirm pool is usable first
        let _ = pool.get_setting("__nonexistent__");

        // Use a raw rusqlite connection to verify WAL mode
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            mode.to_lowercase(),
            "wal",
            "Expected WAL journal mode at {:?}, got '{}'",
            db_path,
            mode
        );

        // 3. Verify basic read/write operations work
        pool.set_setting("test_key", "test_value").expect("Write should succeed");
        let read_back = pool.get_setting("test_key").expect("Read should succeed");
        assert_eq!(
            read_back,
            Some("test_value".to_string()),
            "Should be able to read back the written value"
        );

        // 4. Verify no hardcoded paths — the database is exactly where we asked
        // (implicitly tested: if DbPool used a hardcoded path, the file wouldn't
        // be at our random location, and the assertions above would fail)
    }

    /// **Validates: Requirements 8.3**
    ///
    /// For any valid filesystem path, the database created by `DbPool::open` must
    /// have the full schema initialized (tables exist and are usable).
    #[test]
    fn db_schema_initialized_at_any_path(
        segments in nested_path_strategy(),
        db_name in "[a-zA-Z0-9]{3,10}"
    ) {
        let tmp = TempDir::new().unwrap();

        let mut nested_dir = tmp.path().to_path_buf();
        for seg in &segments {
            nested_dir.push(seg);
        }
        std::fs::create_dir_all(&nested_dir).unwrap();

        let db_path: PathBuf = nested_dir.join(format!("{}.db", db_name));

        let pool = DbPool::open(&db_path).expect("DbPool::open should succeed");

        // Verify the schema is fully initialized by performing operations
        // that require the tables to exist
        let subs = pool.list_all_subscriptions().expect("list_all_subscriptions should work");
        // Fresh database should have no subscriptions
        assert!(subs.is_empty(), "Fresh DB should have no subscriptions");

        // Views table should have default views inserted by the schema
        let views = pool.list_views("asset").expect("list_views should work");
        assert!(
            !views.is_empty(),
            "Fresh DB should have default 'All' asset view"
        );
    }
}
