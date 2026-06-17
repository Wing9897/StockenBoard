/**
 * Backward-compatibility utility for subscription_ids → subscription_id derivation.
 *
 * When an AI rule stores multiple subscription IDs, the legacy `subscription_id`
 * column must equal the first element of the array for backward compatibility
 * with pre-migration code paths.
 */

/**
 * Derive the backward-compatible `subscription_id` from a non-empty
 * `subscription_ids` array.
 *
 * @param subscriptionIds - Non-empty array of subscription IDs
 * @returns The first element of the array (used as subscription_id)
 * @throws If the array is empty
 */
export function deriveSubscriptionId(subscriptionIds: number[]): number {
  if (subscriptionIds.length === 0) {
    throw new Error('subscription_ids must not be empty');
  }
  return subscriptionIds[0];
}

/**
 * Build the rule payload fields for subscription IDs, enforcing
 * the backward-compatibility invariant: subscription_id = subscriptionIds[0].
 *
 * @param subscriptionIds - Non-empty array of subscription IDs for an AI rule
 * @returns Object with both subscription_id and subscription_ids set correctly
 */
export function buildSubscriptionPayload(subscriptionIds: number[]): {
  subscription_id: number;
  subscription_ids: number[];
} {
  return {
    subscription_id: deriveSubscriptionId(subscriptionIds),
    subscription_ids: subscriptionIds,
  };
}
