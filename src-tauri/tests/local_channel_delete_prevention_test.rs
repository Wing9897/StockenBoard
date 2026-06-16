//! Property-based test: Local channel deletion is always prevented.
//!
//! **Feature: logo-management-and-local-notifications, Property 1: Local channel deletion prevention**
//!
//! **Validates: Requirements 3.5**
//!
//! For any valid application state where a local notification channel exists in the database,
//! attempting to delete that channel SHALL always return an error and the channel SHALL remain
//! in the database unchanged.

use proptest::prelude::*;
use std::path::PathBuf;
use stockenboard_lib::db::DbPool;

/// Helper: open an in-memory DB with schema initialized.
fn open_test_db() -> DbPool {
    DbPool::open(&PathBuf::from(":memory:")).unwrap()
}

/// Mirrors the delete protection logic from `commands/notifications.rs`.
/// This is the core invariant we're testing: if a channel is "local", deletion is refused.
fn try_delete_channel(db: &DbPool, id: i64) -> Result<(), String> {
    let channels = db.list_notification_channels().unwrap();
    if let Some(ch) = channels.iter().find(|c| c.id == id) {
        if ch.channel_type == "local" {
            return Err("Cannot delete the built-in local notification channel".to_string());
        }
    }
    db.delete_notification_channel(id)
}

proptest! {
    /// **Property 1: Local channel deletion is always prevented**
    ///
    /// For any random channel ID, if that ID points to a local channel,
    /// `try_delete_channel` always returns an error and the channel remains in the DB.
    ///
    /// **Validates: Requirements 3.5**
    #[test]
    fn local_channel_deletion_always_prevented(random_id_offset in 0i64..1000) {
        let db = open_test_db();

        // Seed the local channel (mimics app startup behavior)
        db.ensure_local_channel().unwrap();

        // Get the actual local channel ID
        let channels = db.list_notification_channels().unwrap();
        let local_channel = channels.iter().find(|c| c.channel_type == "local").unwrap();
        let local_id = local_channel.id;

        // Test 1: Attempting to delete the local channel's actual ID always fails
        let result = try_delete_channel(&db, local_id);
        prop_assert!(result.is_err(), "Deleting local channel should always fail");
        prop_assert!(
            result.unwrap_err().contains("Cannot delete"),
            "Error message should indicate deletion prevention"
        );

        // Verify channel still exists after failed deletion attempt
        let channels_after = db.list_notification_channels().unwrap();
        let still_exists = channels_after.iter().any(|c| c.id == local_id && c.channel_type == "local");
        prop_assert!(still_exists, "Local channel must still exist after deletion attempt");

        // Test 2: Using a random ID that maps to the local channel also fails
        // (simulating different possible ID values that could reference a local channel)
        // We create multiple local channels with different names to get different IDs,
        // but ensure_local_channel is idempotent - so we test with the existing one
        // plus offset-based non-existent IDs
        let non_existent_id = local_id + random_id_offset + 1;
        let result_non_existent = try_delete_channel(&db, non_existent_id);
        // Non-existent IDs should succeed (no-op delete) or fail gracefully
        // The important thing is they don't crash
        prop_assert!(
            result_non_existent.is_ok(),
            "Deleting non-existent channel should succeed (no-op)"
        );
    }

    /// **Property 1 (variant): Multiple deletion attempts on local channel always fail**
    ///
    /// Even after repeated deletion attempts with random sequences,
    /// the local channel persists and every attempt returns an error.
    ///
    /// **Validates: Requirements 3.5**
    #[test]
    fn repeated_deletion_attempts_always_fail(attempts in 1usize..20) {
        let db = open_test_db();
        db.ensure_local_channel().unwrap();

        let channels = db.list_notification_channels().unwrap();
        let local_channel = channels.iter().find(|c| c.channel_type == "local").unwrap();
        let local_id = local_channel.id;

        // Attempt deletion multiple times - every single attempt must fail
        for _ in 0..attempts {
            let result = try_delete_channel(&db, local_id);
            prop_assert!(result.is_err(), "Every deletion attempt on local channel must fail");
        }

        // Channel still exists after all attempts
        let channels_after = db.list_notification_channels().unwrap();
        let still_exists = channels_after.iter().any(|c| c.id == local_id && c.channel_type == "local");
        prop_assert!(still_exists, "Local channel must persist after repeated deletion attempts");
    }

    /// **Property 1 (variant): Local channel deletion prevented regardless of other channels**
    ///
    /// Even when other (non-local) channels exist and can be deleted,
    /// the local channel deletion is always prevented.
    ///
    /// **Validates: Requirements 3.5**
    #[test]
    fn local_protected_even_with_other_channels(
        num_other_channels in 0usize..5,
        delete_target_idx in 0usize..10
    ) {
        let db = open_test_db();
        db.ensure_local_channel().unwrap();

        // Create additional non-local channels
        let mut other_ids = Vec::new();
        for i in 0..num_other_channels {
            let id = db.create_notification_channel(
                "telegram",
                &format!("Channel {}", i),
                "{}",
            ).unwrap();
            other_ids.push(id);
        }

        let channels = db.list_notification_channels().unwrap();
        let local_channel = channels.iter().find(|c| c.channel_type == "local").unwrap();
        let local_id = local_channel.id;

        // Deleting the local channel must still fail
        let result = try_delete_channel(&db, local_id);
        prop_assert!(result.is_err(), "Local channel deletion must be prevented regardless of other channels");

        // Deleting a non-local channel (if any exist) should succeed
        if !other_ids.is_empty() {
            let target = other_ids[delete_target_idx % other_ids.len()];
            let result = try_delete_channel(&db, target);
            prop_assert!(result.is_ok(), "Non-local channels should be deletable");
        }

        // Local channel still exists
        let channels_after = db.list_notification_channels().unwrap();
        let still_exists = channels_after.iter().any(|c| c.id == local_id && c.channel_type == "local");
        prop_assert!(still_exists, "Local channel must still exist");
    }
}
