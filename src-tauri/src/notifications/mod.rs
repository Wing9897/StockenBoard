//! 推播通知模組 — 條件觸發式推播通知系統
//!
//! 子模組：
//! - models: 資料模型定義
//! - engine: NotificationEngine 主邏輯
//! - evaluator: 條件評估邏輯
//! - dispatcher: 通知派發（含重試）
//! - telegram: Telegram Bot 發送器
//! - webhook: Webhook 發送器
//! - crypto: 加密/解密工具
//! - ai_evaluator: AI 評估器（prompt 組裝、API 呼叫、回應解析）
//! - ai_scheduler: AI 排程器（管理 AI 規則的定期評估 task）
//! - global_cooldown: 全局冷卻期（跨規則共享的最小觸發間隔）
//! - token_estimator: Token 估算器（估算 AI prompt token 數量與自動裁剪）

pub mod ai_evaluator;
pub mod ai_scheduler;
pub mod crypto;
pub mod dispatcher;
pub mod engine;
pub mod evaluator;
pub mod global_cooldown;
pub mod models;
pub mod telegram;
pub mod token_estimator;
pub mod webhook;

#[cfg(test)]
mod tests;
