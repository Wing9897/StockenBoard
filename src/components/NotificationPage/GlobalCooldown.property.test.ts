/**
 * **Feature: notification-ui-refactor, Property 2: Non-integer input rejection**
 * **Validates: Requirements 2.7**
 *
 * Property: For any string that is not a non-negative integer pattern,
 * `isValidCooldownInput` returns false. For any non-negative integer string,
 * `isValidCooldownInput` returns true.
 *
 * Strategy: Use fast-check to generate:
 * 1. Valid inputs via fc.nat().map(String) — these should always return true
 * 2. Invalid inputs via strings containing decimals, letters, special chars,
 *    empty string, or whitespace — these should always return false
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { isValidCooldownInput } from './GlobalCooldownInline';

// ── Arbitrary generators ──

/** Generates valid non-negative integer strings (e.g., "0", "42", "999") */
const validIntegerStringArb: fc.Arbitrary<string> = fc.nat().map(String);

/** Generates strings that contain at least one non-digit character */
const invalidStringWithNonDigitArb: fc.Arbitrary<string> = fc.array(
  fc.constantFrom(
    'a', 'b', 'c', 'x', 'y', 'z', 'A', 'B', 'Z',
    '.', '-', '+', 'e', 'E', ' ', '\t', '\n',
    '!', '@', '#', '$', '%', '&', '*', '/', '?'
  ),
  { minLength: 1, maxLength: 20 }
).map(chars => chars.join(''));

/** Generates strings that look like decimals (e.g., "3.14", "0.5") */
const decimalStringArb: fc.Arbitrary<string> = fc.tuple(fc.nat(), fc.nat({ max: 999999 }))
  .map(([whole, frac]) => `${whole}.${frac}`);

/** Generates strings with leading/trailing whitespace around digits */
const whitespaceWrappedDigitsArb: fc.Arbitrary<string> = fc.tuple(
  fc.constantFrom(' ', '\t', '\n', '  '),
  fc.nat().map(String),
  fc.constantFrom(' ', '\t', '\n', '  ')
).map(([pre, num, post]) => `${pre}${num}${post}`);

/** Generates strings with negative sign prefix */
const negativeIntegerStringArb: fc.Arbitrary<string> = fc.nat({ max: 999999 })
  .filter(n => n > 0)
  .map(n => `-${n}`);

/** Generates strings with letters mixed with digits */
const alphanumericMixedArb: fc.Arbitrary<string> = fc.tuple(
  fc.nat().map(String),
  fc.array(
    fc.constantFrom('a', 'b', 'c', 'x', 'y', 'z', 'A', 'B', 'Z'),
    { minLength: 1, maxLength: 5 }
  ).map(chars => chars.join(''))
).map(([digits, letters]) => `${digits}${letters}`);

// ── Property tests ──

describe('Feature: notification-ui-refactor, Property 2: Non-integer input rejection', () => {
  it('returns true for any non-negative integer string', () => {
    fc.assert(
      fc.property(validIntegerStringArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(true);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for strings containing non-digit characters', () => {
    fc.assert(
      fc.property(invalidStringWithNonDigitArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for decimal number strings', () => {
    fc.assert(
      fc.property(decimalStringArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for strings with whitespace around digits', () => {
    fc.assert(
      fc.property(whitespaceWrappedDigitsArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for negative integer strings', () => {
    fc.assert(
      fc.property(negativeIntegerStringArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for alphanumeric mixed strings', () => {
    fc.assert(
      fc.property(alphanumericMixedArb, (input) => {
        expect(isValidCooldownInput(input)).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('returns false for empty string', () => {
    expect(isValidCooldownInput('')).toBe(false);
  });
});
