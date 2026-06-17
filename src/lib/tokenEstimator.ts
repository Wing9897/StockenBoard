/** Constants for token estimation */
export const TOKENS_PER_RECORD = 25;
export const SYSTEM_MESSAGE_OVERHEAD = 200;
export const USER_MESSAGE_OVERHEAD = 100;
export const TOTAL_OVERHEAD = SYSTEM_MESSAGE_OVERHEAD + USER_MESSAGE_OVERHEAD; // 300

/**
 * Estimate the token count for an AI rule evaluation prompt.
 *
 * @param numSubscriptions - Number of subscriptions in the rule
 * @param historyWindow - Number of price records per subscription
 * @returns Estimated token count
 */
export function estimateTokens(numSubscriptions: number, historyWindow: number): number {
  return numSubscriptions * historyWindow * TOKENS_PER_RECORD + TOTAL_OVERHEAD;
}
