//! Notification system tests — split by concern
//!
//! - rule_evaluation: condition evaluation, rule matching, filtering
//! - dispatching: channel delivery, message formatting, e2e dispatch
//! - ai_evaluation: AI config validation, prompt building, response parsing, scheduler
//! - cooldown: cooldown suppression logic
//! - history_record: local notification history record integrity

mod rule_evaluation;
mod dispatching;
mod ai_evaluation;
mod cooldown;
mod history_record;
