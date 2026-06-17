//! AI Scheduler Integration Tests
//! **Validates: Requirements 3.1, 3.4, 3.5, 3.6**

mod ai_scheduler_integration_tests {
    use crate::db::DbPool;
    use crate::notifications::ai_scheduler::AiScheduler;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn setup_scheduler() -> (Arc<DbPool>, AiScheduler) {
        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());
        let scheduler = AiScheduler::new(db.clone());
        (db, scheduler)
    }

    fn create_test_subscription(db: &DbPool) -> i64 {
        db.add_subscription(
            "asset", "BTC/USDT", None, "binance", "crypto", None, None, None,
        )
        .unwrap()
    }

    fn create_ai_rule(db: &DbPool, subscription_id: i64, enabled: bool) -> i64 {
        let ai_config =
            r#"{"prompt": "test prompt", "history_window": 20, "analysis_interval_secs": 60}"#;
        let channel_ids = "[1]";
        let rule_id = db
            .create_notification_rule(
                "Test AI Rule",
                subscription_id,
                "ai",
                0.0,
                channel_ids,
                300,
                Some(ai_config),
                None,
            )
            .unwrap();

        if !enabled {
            db.toggle_notification_rule(rule_id, false).unwrap();
        }

        rule_id
    }

    #[tokio::test]
    async fn test_start_spawns_tasks_for_enabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        assert_eq!(scheduler.task_count().await, 0);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_start_with_no_ai_rules_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_without_provider_config_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_ignores_disabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, false);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_start_ignores_non_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);

        db.create_notification_rule(
            "Threshold Rule",
            sub_id,
            "price_above",
            50000.0,
            "[1]",
            300,
            None,
            None,
        )
        .unwrap();

        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_rule_stops_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_rule_only_affects_target_rule() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.remove_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_rule_idempotent() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_rule_is_noop() {
        let (_db, scheduler) = setup_scheduler().await;

        scheduler.remove_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_starts_new_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        assert_eq!(scheduler.task_count().await, 0);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_restarts_existing_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_disabled_rule_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, false);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_without_provider_config_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_nonexistent_rule_does_not_start_task() {
        let (_db, scheduler) = setup_scheduler().await;

        scheduler.upsert_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_multiple_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        let rule_id_2 = create_ai_rule(&db, sub_id, true);

        scheduler.upsert_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id_2).await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_restarts_all_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_picks_up_new_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        create_ai_rule(&db, sub_id, true);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_drops_removed_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        db.toggle_notification_rule(rule_id_1, false).unwrap();

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_reload_with_no_provider_config_stops_all() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        db.set_setting("ai_base_url", "").unwrap();
        db.set_setting("ai_model", "").unwrap();

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_reload_multiple_times_is_stable() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }
}
