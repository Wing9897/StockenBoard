//! Notification system tests — split by concern
//!
//! - rule_evaluation: condition evaluation, rule matching, filtering
//! - dispatching: channel delivery, message formatting, e2e dispatch
//! - ai_evaluation_config: AI config validation, provider config, DB persistence
//! - ai_evaluation_prompt: build_prompt unit tests
//! - ai_evaluation_parse: parse_ai_response unit tests
//! - ai_evaluation_property: AI property-based tests (Properties 1-10)
//! - ai_evaluation_scheduler: AI scheduler integration tests
//! - cooldown: cooldown suppression logic
//! - history_record: local notification history record integrity

mod rule_evaluation;
mod dispatching;
mod ai_evaluation_config;
mod ai_evaluation_parse;
mod ai_evaluation_prompt;
mod ai_evaluation_property;
mod ai_evaluation_scheduler;
mod cooldown;
mod history_record;
