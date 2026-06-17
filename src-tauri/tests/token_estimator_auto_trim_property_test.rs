//! Property-Based Tests for Auto-Trim Invariants (Property 3)
//!
//! **Validates: Requirements 3.1, 3.3, 3.5**
//!
//! For any `numSubscriptions >= 1`, `historyWindow >= 1`, and `maxContextTokens >= 500`,
//! `compute_trimmed_window(numSubscriptions, historyWindow, Some(maxContextTokens))`
//! SHALL return a value `w` such that:
//! - `estimate_tokens(numSubscriptions, w) <= maxContextTokens` (fits within limit)
//! - `w >= 1` (minimum 1 record per subscription)
//! - The same `w` is applied equally to all subscriptions (equal distribution by construction)

use proptest::prelude::*;
use stockenboard_lib::notifications::token_estimator::{
    compute_trimmed_window, estimate_tokens, TOKENS_PER_RECORD, TOTAL_OVERHEAD,
};

proptest! {
    /// **Validates: Requirements 3.1, 3.3, 3.5**
    /// Property 3(a): Auto-trim fits within token limit
    ///
    /// For any valid triple, the trimmed window produces an estimate
    /// that does not exceed `maxContextTokens`.
    /// When the minimum guarantee (w >= 1) conflicts with the token limit
    /// (i.e., even 1 record per subscription exceeds the budget), the minimum
    /// guarantee takes precedence (per design: "Proceed with 1 record per subscription").
    /// In such cases, the invariant is: w == 1 (minimum enforced).
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
            // (minimum guarantee takes precedence over token limit)
            prop_assert_eq!(w, 1,
                "When min allocation exceeds budget, w should be 1. \
                 n={}, h={}, max_tokens={}, min_possible={}",
                n, h, max_tokens, min_possible);
        }
    }

    /// **Validates: Requirements 3.1, 3.3, 3.5**
    /// Property 3(b): Auto-trim guarantees minimum window of 1
    ///
    /// For any valid triple, the trimmed window is always at least 1,
    /// ensuring every subscription gets at least one record.
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

    /// **Validates: Requirements 3.1, 3.3, 3.5**
    /// Property 3(c): Auto-trim equal distribution by construction
    ///
    /// The function returns a single `w` applied uniformly to all subscriptions.
    /// Each subscription contributes exactly `w * TOKENS_PER_RECORD` tokens.
    /// We verify this structural invariant by checking that:
    /// `n * w * TOKENS_PER_RECORD + TOTAL_OVERHEAD == estimate_tokens(n, w)`
    #[test]
    fn prop_auto_trim_equal_distribution(
        n in 1u32..=100,
        h in 1u32..=1000,
        max_tokens in 500u32..=200_000,
    ) {
        let w = compute_trimmed_window(n, h, Some(max_tokens));
        // Each subscription gets exactly w records (equal distribution).
        // Total token usage = n * w * TOKENS_PER_RECORD + TOTAL_OVERHEAD.
        let per_subscription_tokens = w * TOKENS_PER_RECORD;
        let total_from_records = n * per_subscription_tokens;
        let total_estimated = total_from_records + TOTAL_OVERHEAD;
        prop_assert_eq!(total_estimated, estimate_tokens(n, w),
            "Equal distribution violated: n={}, w={}, per_sub_tokens={}, total={}",
            n, w, per_subscription_tokens, total_estimated);
    }
}
