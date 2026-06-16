import { useState, useEffect, useCallback, useRef } from 'react';
import { getTransport } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { silentLog } from '../../lib/errorLog';

/**
 * Clamps a cooldown value to [0, 3600], rounding to the nearest integer.
 */
export function clampCooldown(value: number): number {
  return Math.max(0, Math.min(3600, Math.round(value)));
}

/**
 * Validates that a string represents a non-negative integer (digits only).
 */
export function isValidCooldownInput(input: string): boolean {
  return /^\d+$/.test(input);
}

/**
 * Inline global cooldown number input, designed to be rendered
 * inside the RuleList header between the title and "Add Rule" button.
 */
export function GlobalCooldownInline() {
  const [cooldown, setCooldown] = useState<number>(30);
  const [loading, setLoading] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    getTransport().invoke<number>('get_notification_global_cooldown')
      .then(val => setCooldown(val))
      .catch(e => silentLog('GlobalCooldown.load', e))
      .finally(() => setLoading(false));
  }, []);

  const debounceSave = useCallback((value: number) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      getTransport().invoke('set_notification_global_cooldown', { secs: value })
        .catch(e => silentLog('GlobalCooldown.save', e));
    }, 300);
  }, []);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    if (!isValidCooldownInput(raw)) return;
    const num = Number(raw);
    setCooldown(num);
    debounceSave(num);
  }, [debounceSave]);

  const handleBlur = useCallback(() => {
    const clamped = clampCooldown(cooldown);
    if (clamped !== cooldown) {
      setCooldown(clamped);
      debounceSave(clamped);
    }
  }, [cooldown, debounceSave]);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  if (loading) return null;

  return (
    <div className="global-cooldown-inline">
      <span className="global-cooldown-label">{t.notifications.globalCooldown}</span>
      <input
        type="number"
        min={0}
        max={3600}
        step={1}
        value={cooldown}
        onChange={handleChange}
        onBlur={handleBlur}
        className="cooldown-input"
        aria-label={t.notifications.globalCooldown}
      />
      <span className="global-cooldown-unit">{t.notifications.globalCooldownUnit}</span>
    </div>
  );
}
