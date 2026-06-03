//! 全局通知冷卻期 — 控制所有通知規則的最小觸發間隔
//!
//! `GlobalCooldown` 為執行緒安全的共享結構，由 `NotificationEngine` 和 `AiScheduler`
//! 透過 `Arc<GlobalCooldown>` 共同引用。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// 全局通知冷卻期控制器
///
/// - `cooldown_secs`: 全局冷卻秒數（0 表示不啟用全局限制）
/// - `last_trigger`: 上次任何規則成功觸發的時間戳（None 代表尚未觸發過）
pub struct GlobalCooldown {
    cooldown_secs: AtomicU64,
    last_trigger: RwLock<Option<Instant>>,
}

impl GlobalCooldown {
    /// 建立新的 GlobalCooldown 實例
    ///
    /// # Arguments
    /// * `cooldown_secs` - 全局冷卻秒數，0 表示不啟用全局限制
    pub fn new(cooldown_secs: u64) -> Self {
        Self {
            cooldown_secs: AtomicU64::new(cooldown_secs),
            // None 代表從未觸發過，第一次 check_and_trigger 一定會通過
            last_trigger: RwLock::new(None),
        }
    }

    /// 檢查全局冷卻期是否已過，若已過則更新 last_trigger 並回傳 true。
    ///
    /// 此方法為原子性操作：取得寫鎖後同時檢查和更新，避免 race condition。
    ///
    /// # Returns
    /// - `true`: 冷卻期已過，允許觸發（已更新 last_trigger）
    /// - `false`: 仍在冷卻期內，不允許觸發
    pub fn check_and_trigger(&self) -> bool {
        let cooldown = self.cooldown_secs.load(Ordering::Relaxed);

        // cooldown = 0 代表不啟用全局限制，直接放行
        if cooldown == 0 {
            return true;
        }

        let mut last = self.last_trigger.write().unwrap();
        let can_trigger = match *last {
            None => true, // 從未觸發過，放行
            Some(prev) => prev.elapsed().as_secs() >= cooldown,
        };

        if can_trigger {
            *last = Some(Instant::now());
            true
        } else {
            false
        }
    }

    /// 設定全局冷卻秒數
    ///
    /// # Arguments
    /// * `secs` - 新的冷卻秒數，0 表示停用全局限制
    pub fn set_cooldown(&self, secs: u64) {
        self.cooldown_secs.store(secs, Ordering::Relaxed);
    }

    /// 取得目前的全局冷卻秒數
    pub fn get_cooldown(&self) -> u64 {
        self.cooldown_secs.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_with_zero_always_allows() {
        let gc = GlobalCooldown::new(0);
        assert!(gc.check_and_trigger());
        assert!(gc.check_and_trigger());
        assert!(gc.check_and_trigger());
    }

    #[test]
    fn test_first_trigger_always_passes() {
        let gc = GlobalCooldown::new(60);
        // 第一次觸發應該通過（因為 last_trigger 初始化為很久以前）
        assert!(gc.check_and_trigger());
    }

    #[test]
    fn test_second_trigger_within_cooldown_blocked() {
        let gc = GlobalCooldown::new(60);
        assert!(gc.check_and_trigger());
        // 立即再觸發應被擋下
        assert!(!gc.check_and_trigger());
    }

    #[test]
    fn test_trigger_after_cooldown_passes() {
        let gc = GlobalCooldown::new(1);
        assert!(gc.check_and_trigger());
        // 等待超過 cooldown
        thread::sleep(Duration::from_millis(1100));
        assert!(gc.check_and_trigger());
    }

    #[test]
    fn test_set_and_get_cooldown() {
        let gc = GlobalCooldown::new(30);
        assert_eq!(gc.get_cooldown(), 30);
        gc.set_cooldown(120);
        assert_eq!(gc.get_cooldown(), 120);
    }

    #[test]
    fn test_set_cooldown_to_zero_disables() {
        let gc = GlobalCooldown::new(60);
        assert!(gc.check_and_trigger());
        // 在 cooldown 內，應被擋
        assert!(!gc.check_and_trigger());
        // 設定為 0 後，立刻放行
        gc.set_cooldown(0);
        assert!(gc.check_and_trigger());
        assert!(gc.check_and_trigger());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;

        let gc = Arc::new(GlobalCooldown::new(1));
        let mut handles = vec![];

        // 多執行緒同時嘗試觸發，只有一個應成功
        for _ in 0..10 {
            let gc_clone = gc.clone();
            handles.push(thread::spawn(move || gc_clone.check_and_trigger()));
        }

        let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let success_count = results.iter().filter(|&&r| r).count();
        // 至少一個成功，且不會全部都成功（因為 cooldown = 1 秒）
        assert!(success_count >= 1);
        assert!(success_count < 10);
    }
}
