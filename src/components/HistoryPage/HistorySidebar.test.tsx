/**
 * Property-based tests for HistorySidebar recording toggle behavior.
 *
 * Feature: auto-unattended-polling
 *
 * Property 1: Recording toggles are always interactive (never disabled by unattended state)
 * **Validates: Requirements 1.2**
 *
 * Property 2: Disabling unattended preserves recording states
 * **Validates: Requirements 3.4**
 */
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';
import fc from 'fast-check';
import { HistorySidebar } from './HistorySidebar';
import type { Subscription } from '../../types';

// Mock the transport layer (AssetIcon uses it internally)
vi.mock('../../lib/transport', () => ({
  getTransport: () => ({
    invoke: () => Promise.resolve(),
    listen: () => () => {},
  }),
  createTransport: () => ({
    invoke: () => Promise.resolve(),
    listen: () => () => {},
  }),
  isTauri: () => false,
}));

/** Arbitrary generator for a Subscription object with relevant fields. */
const arbSubscription = (id: number): fc.Arbitrary<Subscription> =>
  fc.record({
    id: fc.constant(id),
    sub_type: fc.constantFrom('asset' as const, 'dex' as const),
    symbol: fc.string({ minLength: 1, maxLength: 8 }).filter(s => /\S/.test(s)),
    display_name: fc.option(fc.string({ minLength: 1, maxLength: 16 }), { nil: undefined }),
    selected_provider_id: fc.constantFrom('binance', 'coingecko', 'coinmarketcap'),
    asset_type: fc.constantFrom('crypto', 'stock'),
    sort_order: fc.constant(id),
    record_enabled: fc.constantFrom(0, 1),
    record_from_hour: fc.constant(null),
    record_to_hour: fc.constant(null),
  }) as fc.Arbitrary<Subscription>;

/** Generates a list of subscriptions with unique IDs. */
const arbSubscriptions: fc.Arbitrary<Subscription[]> = fc
  .integer({ min: 1, max: 15 })
  .chain(len =>
    fc.tuple(...Array.from({ length: len }, (_, i) => arbSubscription(i + 1)))
  );

const noop = () => {};

describe('HistorySidebar — Property 1: Recording toggles are always interactive', () => {
  it('recording toggles and batch buttons are never disabled due to isUnattended state', () => {
    fc.assert(
      fc.property(
        arbSubscriptions,
        fc.boolean(),
        (subs, isUnattended) => {
          const { container } = render(
            <HistorySidebar
              subs={subs}
              selectedId={null}
              filter="all"
              search=""
              onSelectId={noop}
              onSetFilter={noop}
              onSetSearch={noop}
              onToggle={noop}
              onBatchToggle={noop}
              onCollapse={noop}
              onSaveRecordHours={noop}
              tzLabel="UTC"
              isUnattended={isUnattended}
            />
          );

          // Individual recording toggle buttons should NEVER be disabled
          const recordToggles = container.querySelectorAll<HTMLButtonElement>('.history-record-toggle');
          expect(recordToggles.length).toBe(subs.length);
          for (const btn of recordToggles) {
            expect(btn.disabled).toBe(false);
          }

          // Batch enable button: disabled only when all are already on
          const batchEnable = container.querySelector<HTMLButtonElement>('.history-batch-btn.enable');
          expect(batchEnable).not.toBeNull();
          const allOn = subs.length > 0 && subs.every(s => s.record_enabled);
          expect(batchEnable!.disabled).toBe(allOn);

          // Batch disable button: disabled only when no filtered recordings exist
          const batchDisable = container.querySelector<HTMLButtonElement>('.history-batch-btn.disable');
          expect(batchDisable).not.toBeNull();
          const filtRecCount = subs.filter(s => s.record_enabled).length;
          expect(batchDisable!.disabled).toBe(filtRecCount === 0);
        }
      ),
      { numRuns: 100 }
    );
  }, 30_000);
});


describe('HistorySidebar — Property 2: Disabling unattended preserves recording states', () => {
  /**
   * **Validates: Requirements 3.4**
   *
   * For any list of subscriptions with arbitrary record_enabled values,
   * when the unattended polling toggle transitions from enabled to disabled,
   * the record_enabled state of every subscription SHALL remain unchanged.
   */
  it('record_enabled values and visual indicators are preserved when isUnattended transitions true→false', () => {
    fc.assert(
      fc.property(
        arbSubscriptions,
        (subs) => {
          // 1. Render with isUnattended=true (enabled state)
          const { container, rerender } = render(
            <HistorySidebar
              subs={subs}
              selectedId={null}
              filter="all"
              search=""
              onSelectId={noop}
              onSetFilter={noop}
              onSetSearch={noop}
              onToggle={noop}
              onBatchToggle={noop}
              onCollapse={noop}
              onSaveRecordHours={noop}
              tzLabel="UTC"
              isUnattended={true}
            />
          );

          // Capture the initial record_enabled values
          const originalRecordStates = subs.map(s => s.record_enabled);

          // 2. Re-render with isUnattended=false (simulating the toggle transition)
          rerender(
            <HistorySidebar
              subs={subs}
              selectedId={null}
              filter="all"
              search=""
              onSelectId={noop}
              onSetFilter={noop}
              onSetSearch={noop}
              onToggle={noop}
              onBatchToggle={noop}
              onCollapse={noop}
              onSaveRecordHours={noop}
              tzLabel="UTC"
              isUnattended={false}
            />
          );

          // 3. Verify the subscription data record_enabled values remain unchanged
          for (let i = 0; i < subs.length; i++) {
            expect(subs[i].record_enabled).toBe(originalRecordStates[i]);
          }

          // 4. Verify visual recording indicators still reflect the original record_enabled values
          const recordToggles = container.querySelectorAll<HTMLButtonElement>('.history-record-toggle');
          expect(recordToggles.length).toBe(subs.length);
          for (let i = 0; i < subs.length; i++) {
            const hasRecordingClass = recordToggles[i].classList.contains('recording');
            const expectedRecording = subs[i].record_enabled === 1;
            expect(hasRecordingClass).toBe(expectedRecording);
          }

          // Also verify the recording dots still appear for subscriptions with record_enabled
          const recDots = container.querySelectorAll('.history-rec-dot');
          const expectedDotCount = subs.filter(s => s.record_enabled).length;
          expect(recDots.length).toBe(expectedDotCount);
        }
      ),
      { numRuns: 100 }
    );
  });
});
