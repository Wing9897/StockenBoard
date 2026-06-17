//! build_prompt unit tests

mod build_prompt_tests {
    use crate::notifications::ai_evaluator::{build_prompt, PriceRecord};

    #[test]
    fn test_build_prompt_returns_two_messages() {
        let records = vec![PriceRecord {
            price: 68500.0,
            change_pct: 2.3,
            volume: 1234.5,
            recorded_at: "2024-01-15 10:30".to_string(),
        }];
        let messages = build_prompt("價格上升超過5%時提醒", &records);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_prompt_system_message_content() {
        let messages = build_prompt("test", &[]);
        let system = &messages[0];
        assert!(system.content.contains("金融市場分析助手"));
        assert!(system.content.contains("JSON"));
        assert!(system.content.contains("trigger"));
        assert!(system.content.contains("reason"));
    }

    #[test]
    fn test_build_prompt_user_message_contains_condition() {
        let condition = "當價格在短時間內大幅上升超過 5% 時提醒我";
        let messages = build_prompt(condition, &[]);
        let user = &messages[1];
        assert!(user.content.contains(condition));
        assert!(user.content.contains("觸發條件："));
    }

    #[test]
    fn test_build_prompt_user_message_contains_all_price_data() {
        let records = vec![
            PriceRecord {
                price: 68500.0,
                change_pct: 2.3,
                volume: 1234.5,
                recorded_at: "2024-01-15 10:30".to_string(),
            },
            PriceRecord {
                price: 67000.0,
                change_pct: -1.5,
                volume: 987.2,
                recorded_at: "2024-01-15 10:25".to_string(),
            },
        ];
        let messages = build_prompt("test condition", &records);
        let user = &messages[1];

        assert!(user.content.contains("68500.00"));
        assert!(user.content.contains("67000.00"));

        assert!(user.content.contains("+2.3%"));
        assert!(user.content.contains("-1.5%"));

        assert!(user.content.contains("1234.5"));
        assert!(user.content.contains("987.2"));

        assert!(user.content.contains("2024-01-15 10:30"));
        assert!(user.content.contains("2024-01-15 10:25"));
    }

    #[test]
    fn test_build_prompt_user_message_contains_record_count() {
        let records = vec![
            PriceRecord {
                price: 100.0,
                change_pct: 1.0,
                volume: 500.0,
                recorded_at: "2024-01-01 00:00".to_string(),
            },
            PriceRecord {
                price: 200.0,
                change_pct: 2.0,
                volume: 600.0,
                recorded_at: "2024-01-01 01:00".to_string(),
            },
            PriceRecord {
                price: 300.0,
                change_pct: 3.0,
                volume: 700.0,
                recorded_at: "2024-01-01 02:00".to_string(),
            },
        ];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("最近 3 筆價格紀錄"));
    }

    #[test]
    fn test_build_prompt_empty_price_history() {
        let messages = build_prompt("test condition", &[]);
        assert_eq!(messages.len(), 2);
        let user = &messages[1];
        assert!(user.content.contains("最近 0 筆價格紀錄"));
        assert!(user.content.contains("test condition"));
    }

    #[test]
    fn test_build_prompt_table_header_present() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 0.0,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user
            .content
            .contains("| 時間 | 價格 | 漲跌幅(%) | 成交量 |"));
        assert!(user
            .content
            .contains("|------|------|-----------|--------|"));
    }

    #[test]
    fn test_build_prompt_positive_change_has_plus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 5.5,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("+5.5%"));
    }

    #[test]
    fn test_build_prompt_negative_change_has_minus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: -3.2,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("-3.2%"));
    }

    #[test]
    fn test_build_prompt_zero_change_has_plus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 0.0,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("+0.0%"));
    }
}
