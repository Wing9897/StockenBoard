//! Token 估算器 — 純函式，用於估算 AI 規則 prompt 的 token 數量

/// 每筆價格紀錄估算使用的 token 數
pub const TOKENS_PER_RECORD: u32 = 25;

/// 系統訊息（system message）的固定 token 開銷
pub const SYSTEM_MESSAGE_OVERHEAD: u32 = 200;

/// 使用者訊息（user message）的固定 token 開銷
pub const USER_MESSAGE_OVERHEAD: u32 = 100;

/// 總固定 token 開銷（系統訊息 + 使用者訊息）
pub const TOTAL_OVERHEAD: u32 = SYSTEM_MESSAGE_OVERHEAD + USER_MESSAGE_OVERHEAD;

/// Estimate the token count for a multi-subscription AI rule prompt.
///
/// # Formula
/// `num_subscriptions * history_window * TOKENS_PER_RECORD + TOTAL_OVERHEAD`
///
/// # Arguments
/// - `num_subscriptions` — Number of subscriptions in the rule
/// - `history_window` — Number of price records per subscription
///
/// # Returns
/// Estimated token count
pub fn estimate_tokens(num_subscriptions: u32, history_window: u32) -> u32 {
    num_subscriptions * history_window * TOKENS_PER_RECORD + TOTAL_OVERHEAD
}

/// Compute the trimmed history window per subscription.
///
/// Returns the original `history_window` if no trimming is needed,
/// or `max_context_tokens` is `None` (unconfigured).
///
/// # Invariants
/// - Result fits within `max_context_tokens` when configured
/// - Each subscription gets equal allocation
/// - Minimum 1 record per subscription
///
/// # Arguments
/// - `num_subscriptions` — Number of subscriptions in the rule
/// - `history_window` — Desired number of price records per subscription
/// - `max_context_tokens` — Optional context window limit
///
/// # Returns
/// The (possibly reduced) history window to use per subscription
pub fn compute_trimmed_window(
    num_subscriptions: u32,
    history_window: u32,
    max_context_tokens: Option<u32>,
) -> u32 {
    let max_tokens = match max_context_tokens {
        Some(max) => max,
        None => return history_window, // No limit configured, skip trim
    };

    let estimated = estimate_tokens(num_subscriptions, history_window);
    if estimated <= max_tokens {
        return history_window; // Already fits, no trim needed
    }

    // Edge case: zero subscriptions means no records to trim
    if num_subscriptions == 0 {
        return history_window;
    }

    // available_record_budget = (max_tokens - TOTAL_OVERHEAD) / TOKENS_PER_RECORD
    // trimmed_window = available_record_budget / num_subscriptions
    let available_for_records = max_tokens.saturating_sub(TOTAL_OVERHEAD);
    let total_records = available_for_records / TOKENS_PER_RECORD;
    let trimmed = total_records / num_subscriptions;

    // Enforce minimum of 1 record per subscription
    trimmed.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_constants() {
        assert_eq!(TOKENS_PER_RECORD, 25);
        assert_eq!(SYSTEM_MESSAGE_OVERHEAD, 200);
        assert_eq!(USER_MESSAGE_OVERHEAD, 100);
        assert_eq!(TOTAL_OVERHEAD, 300);
    }

    #[test]
    fn test_estimate_tokens_zero_subscriptions() {
        assert_eq!(estimate_tokens(0, 10), 300);
    }

    #[test]
    fn test_estimate_tokens_zero_history() {
        assert_eq!(estimate_tokens(5, 0), 300);
    }

    #[test]
    fn test_estimate_tokens_single_subscription() {
        // 1 * 10 * 25 + 300 = 550
        assert_eq!(estimate_tokens(1, 10), 550);
    }

    #[test]
    fn test_estimate_tokens_multiple_subscriptions() {
        // 3 * 20 * 25 + 300 = 1800
        assert_eq!(estimate_tokens(3, 20), 1800);
    }

    #[test]
    fn test_estimate_tokens_formula() {
        // Verify the formula for arbitrary values
        let n = 5;
        let h = 30;
        let expected = n * h * 25 + 300;
        assert_eq!(estimate_tokens(n, h), expected);
    }

    // --- compute_trimmed_window tests ---

    #[test]
    fn test_trimmed_window_none_max_tokens_returns_original() {
        // When max_context_tokens is None, return history_window unchanged
        assert_eq!(compute_trimmed_window(3, 20, None), 20);
        assert_eq!(compute_trimmed_window(0, 50, None), 50);
        assert_eq!(compute_trimmed_window(10, 100, None), 100);
    }

    #[test]
    fn test_trimmed_window_zero_subscriptions() {
        // Zero subscriptions → no records to trim, return original
        assert_eq!(compute_trimmed_window(0, 10, Some(500)), 10);
    }

    #[test]
    fn test_trimmed_window_no_trimming_needed() {
        // 2 subs * 5 records * 25 + 300 = 550, limit is 1000 → no trim
        assert_eq!(compute_trimmed_window(2, 5, Some(1000)), 5);
    }

    #[test]
    fn test_trimmed_window_trimming_required() {
        // 3 subs * 20 records * 25 + 300 = 1800, limit is 1000
        // available_for_records = 1000 - 300 = 700
        // total_records = 700 / 25 = 28
        // trimmed = 28 / 3 = 9
        assert_eq!(compute_trimmed_window(3, 20, Some(1000)), 9);
    }

    #[test]
    fn test_trimmed_window_minimum_enforcement() {
        // 10 subs * 20 records * 25 + 300 = 5300, limit is 500
        // available_for_records = 500 - 300 = 200
        // total_records = 200 / 25 = 8
        // trimmed = 8 / 10 = 0 → clamped to 1
        assert_eq!(compute_trimmed_window(10, 20, Some(500)), 1);
    }

    #[test]
    fn test_trimmed_window_exact_fit() {
        // 2 subs * 10 records * 25 + 300 = 800, limit is exactly 800 → no trim
        assert_eq!(compute_trimmed_window(2, 10, Some(800)), 10);
    }

    #[test]
    fn test_trimmed_window_very_low_max_tokens() {
        // max_tokens less than TOTAL_OVERHEAD
        // available_for_records = saturating_sub → 0
        // total_records = 0 / 25 = 0
        // trimmed = 0 / 5 = 0 → clamped to 1
        assert_eq!(compute_trimmed_window(5, 20, Some(100)), 1);
    }

    #[test]
    fn test_trimmed_window_result_fits_within_limit() {
        // Verify the invariant: estimate_tokens(n, trimmed) <= max_tokens
        let n = 4;
        let h = 50;
        let max = 1500u32;
        let trimmed = compute_trimmed_window(n, h, Some(max));
        assert!(estimate_tokens(n, trimmed) <= max);
        assert!(trimmed >= 1);
    }

    // --- Property-Based Tests ---
    // **Validates: Requirements 2.1, 2.3**
    // Property 1: Token estimation formula correctness
    // For any non-negative integers numSubscriptions and historyWindow,
    // estimate_tokens(n, h) == n * h * 25 + 300.
    proptest! {
        #[test]
        fn prop_estimate_tokens_formula(
            n in 0u32..=1000,
            h in 0u32..=1000,
        ) {
            let result = estimate_tokens(n, h);
            let expected = n * h * TOKENS_PER_RECORD + TOTAL_OVERHEAD;
            prop_assert_eq!(result, expected);
        }

        #[test]
        fn prop_estimate_tokens_always_gte_overhead(
            n in 0u32..=1000,
            h in 0u32..=1000,
        ) {
            let result = estimate_tokens(n, h);
            prop_assert!(result >= TOTAL_OVERHEAD,
                "estimate_tokens({}, {}) = {} should be >= TOTAL_OVERHEAD ({})",
                n, h, result, TOTAL_OVERHEAD);
        }

        /// **Validates: Requirements 3.1, 3.3, 3.5**
        /// Property 3: Auto-trim invariants
        /// For any (numSubscriptions >= 1, historyWindow >= 1, maxContextTokens >= 500),
        /// compute_trimmed_window returns w such that:
        /// (a) estimate_tokens(n, w) <= maxContextTokens (when feasible)
        /// (b) w >= 1
        /// (c) equal distribution by construction (single w returned for all subscriptions)
        ///
        /// When even the minimum allocation (1 record per subscription) exceeds the budget,
        /// the minimum guarantee (w >= 1) takes precedence over the token limit.
        #[test]
        fn prop_auto_trim_fits_within_limit(
            n in 1u32..=100,
            h in 1u32..=1000,
            max_tokens in 500u32..=200_000,
        ) {
            let w = compute_trimmed_window(n, h, Some(max_tokens));
            let estimated = estimate_tokens(n, w);
            let min_possible = estimate_tokens(n, 1);

            if min_possible <= max_tokens {
                // When it's feasible to fit within the limit, the invariant holds
                prop_assert!(estimated <= max_tokens,
                    "estimate_tokens({}, {}) = {} exceeds max_context_tokens {} \
                     (min_possible {} fits, so trimming should work)",
                    n, w, estimated, max_tokens, min_possible);
            } else {
                // When even minimum allocation exceeds budget, w is clamped to 1
                prop_assert_eq!(w, 1,
                    "When min allocation exceeds budget, w should be 1. \
                     n={}, h={}, max_tokens={}, min_possible={}",
                    n, h, max_tokens, min_possible);
            }
        }

        #[test]
        fn prop_auto_trim_minimum_window(
            n in 1u32..=100,
            h in 1u32..=1000,
            max_tokens in 500u32..=200_000,
        ) {
            let w = compute_trimmed_window(n, h, Some(max_tokens));
            prop_assert!(w >= 1,
                "compute_trimmed_window({}, {}, Some({})) = {} should be >= 1",
                n, h, max_tokens, w);
        }

        #[test]
        fn prop_auto_trim_equal_distribution(
            n in 1u32..=100,
            h in 1u32..=1000,
            max_tokens in 500u32..=200_000,
        ) {
            // The function returns a single w that is applied equally to all
            // subscriptions. Verify that the same value is returned regardless
            // of which "subscription slot" we consider — i.e., calling
            // compute_trimmed_window once yields one w for all n subscriptions.
            let w = compute_trimmed_window(n, h, Some(max_tokens));
            // The total token usage equals n * w * TOKENS_PER_RECORD + TOTAL_OVERHEAD,
            // meaning each subscription contributes exactly w * TOKENS_PER_RECORD tokens.
            // This confirms equal distribution by construction.
            let per_subscription_tokens = w * TOKENS_PER_RECORD;
            let total_from_records = n * per_subscription_tokens;
            let total_estimated = total_from_records + TOTAL_OVERHEAD;
            prop_assert_eq!(total_estimated, estimate_tokens(n, w),
                "Equal distribution violated: n={}, w={}, per_sub_tokens={}, total={}",
                n, w, per_subscription_tokens, total_estimated);
        }
    }
}
