//! 推播通知模組 — 條件觸發式推播通知系統
//!
//! 子模組：
//! - models: 資料模型定義
//! - engine: NotificationEngine 主邏輯
//! - evaluator: 條件評估邏輯
//! - dispatcher: 通知派發（含重試）
//! - telegram: Telegram Bot 發送器
//! - webhook: Webhook 發送器

pub mod models;
pub mod engine;
pub mod evaluator;
pub mod dispatcher;
pub mod telegram;
pub mod webhook;
pub mod crypto;

#[cfg(test)]
mod tests;
