import { memo, useState, useEffect, useCallback } from 'react';
import { usePollTick } from '../../hooks/useAssetData';
import { t } from '../../lib/i18n';

interface CountdownCircleProps {
  providerId: string;
  fallbackInterval: number;
  size?: number;
  isWebSocket?: boolean;
}

type Listener = () => void;
const _listeners = new Set<Listener>();
let _globalTimer: ReturnType<typeof setInterval> | null = null;

function subscribe(fn: Listener) {
  _listeners.add(fn);
  if (!_globalTimer) {
    _globalTimer = setInterval(() => {
      for (const fn of _listeners) fn();
    }, 1000);
  }
  return () => {
    _listeners.delete(fn);
    if (_listeners.size === 0 && _globalTimer) {
      clearInterval(_globalTimer);
      _globalTimer = null;
    }
  };
}

export const CountdownCircle = memo(function CountdownCircle({ providerId, fallbackInterval, size = 24, isWebSocket = false }: CountdownCircleProps) {
  const tick = usePollTick(providerId);
  const [now, setNow] = useState(Date.now);
  const refresh = useCallback(() => setNow(Date.now()), []);

  const interval = tick?.intervalMs ?? fallbackInterval;
  const lastFetch = tick?.fetchedAt ?? 0;

  useEffect(() => {
    if (isWebSocket || interval <= 0) return;
    refresh();
    return subscribe(refresh);
  }, [interval, lastFetch, isWebSocket, refresh]);

  const r = (size - 4) / 2;
  const c = 2 * Math.PI * r;
  const center = size / 2;

  if (isWebSocket) {
    return (
      <div className="countdown-circle" title={t.countdown.ws}>
        <svg width={size} height={size}>
          <circle cx={center} cy={center} r={r} fill="none" stroke="var(--surface0)" strokeWidth="2" />
          <circle cx={center} cy={center} r={r} fill="none" stroke="var(--green)" strokeWidth="2"
            strokeDasharray={c} strokeDashoffset={0}
            strokeLinecap="round" transform={`rotate(-90 ${center} ${center})`} />
          <text x={center} y={center + 1} textAnchor="middle" dominantBaseline="middle"
            fill="var(--green)" fontSize="7" fontWeight="600">WS</text>
        </svg>
      </div>
    );
  }

  if (!tick) {
    return (
      <div className="countdown-circle" title={t.countdown.waiting}>
        <svg width={size} height={size}>
          <circle cx={center} cy={center} r={r} fill="none" stroke="var(--surface0)" strokeWidth="2" />
          <text x={center} y={center + 1} textAnchor="middle" dominantBaseline="middle"
            fill="var(--overlay0)" fontSize="7" fontWeight="500">...</text>
        </svg>
      </div>
    );
  }

  const elapsed = now - lastFetch;
  const progress = Math.min(elapsed / interval, 1);
  const offset = c * (1 - progress);
  const remaining = Math.max(0, Math.ceil((interval - elapsed) / 1000));
  const color = progress > 0.85 ? 'var(--yellow)' : 'var(--blue)';

  return (
    <div className="countdown-circle" title={t.countdown.updateIn(remaining)}>
      <svg width={size} height={size}>
        <circle cx={center} cy={center} r={r} fill="none" stroke="var(--surface0)" strokeWidth="2" />
        <circle cx={center} cy={center} r={r} fill="none" stroke={color} strokeWidth="2"
          strokeDasharray={c} strokeDashoffset={offset}
          strokeLinecap="round" transform={`rotate(-90 ${center} ${center})`}
          style={{ transition: 'stroke-dashoffset 1s linear' }} />
        <text x={center} y={center + 1} textAnchor="middle" dominantBaseline="middle"
          fill="var(--overlay0)" fontSize="8" fontWeight="500">{remaining}</text>
      </svg>
    </div>
  );
});
