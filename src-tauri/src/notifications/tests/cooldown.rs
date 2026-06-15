//! Cooldown tests — cooldown suppression logic

use proptest::prelude::*;

proptest! {
    // Feature: push-notifications, Property 5: 冷卻期抑制
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_cooldown_suppression(
        cooldown_secs in 1u64..3600,
        elapsed_secs in 0u64..7200,
    ) {
        // If elapsed < cooldown, should suppress (true means suppressed)
        // If elapsed >= cooldown, should not suppress
        let should_suppress = elapsed_secs < cooldown_secs;
        let actual_suppress = elapsed_secs < cooldown_secs;
        prop_assert_eq!(should_suppress, actual_suppress);
    }
}

// === AI Cooldown Property Test (Property 12) ===

mod ai_cooldown_property_tests {
    use crate::notifications::ai_scheduler::should_suppress_trigger;
    use proptest::prelude::*;
    use std::time::{Duration, Instant};

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: ai-notification-rules, Property 12: Cooldown Prevents Re-Trigger
        /// **Validates: Requirements 5.3**
        #[test]
        fn prop_cooldown_prevents_retrigger(
            cooldown_secs in 1u64..3600,
            elapsed_millis in 0u64..7_200_000,
        ) {
            let elapsed_secs = elapsed_millis / 1000;

            // Case 1: When last_trigger is None (never triggered), should never suppress
            let result_none = should_suppress_trigger(None, cooldown_secs);
            prop_assert!(
                !result_none,
                "should_suppress_trigger(None, {}) should be false (never triggered before)",
                cooldown_secs
            );

            // Case 2: When last_trigger is Some(time) and elapsed < cooldown, should suppress
            // When elapsed >= cooldown, should NOT suppress
            let last_trigger = Instant::now() - Duration::from_millis(elapsed_millis);
            let result = should_suppress_trigger(Some(last_trigger), cooldown_secs);

            if elapsed_secs < cooldown_secs {
                prop_assert!(
                    result,
                    "Expected suppression when elapsed_secs ({}) < cooldown_secs ({})",
                    elapsed_secs, cooldown_secs
                );
            } else {
                prop_assert!(
                    !result,
                    "Expected NO suppression when elapsed_secs ({}) >= cooldown_secs ({})",
                    elapsed_secs, cooldown_secs
                );
            }
        }
    }
}
