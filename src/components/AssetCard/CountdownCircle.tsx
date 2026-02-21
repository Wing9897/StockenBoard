import { useState, useEffect } from 'react';

interface CountdownCircleProps {
  interval: number; // ms, 0 = websocket (live)
  lastFetch: number; // timestamp ms
  size?: number;
}

export function CountdownCircle({ interval, lastFetch, size = 24 }: CountdownCircleProps) {
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (interval <= 0) {
      setProgress(1);
      return;
    }

    const tick = () => {
      const elapsed = Date.now() - lastFetch;
      const p = Math.min(elapsed / interval, 1);
      setProgress(p);
    };

    tick();
    const id = setInterval(tick, 200);
    return () => clearInterval(id);
  }, [interval, lastFetch]);

  const r = (size - 4) / 2;
  const c = 2 * Math.PI * r;
  const offset = c * (1 - progress);
  const center = size / 2;

  // WebSocket = always live
  if (interval <= 0) {
    return (
      <div className="countdown-circle" title="WebSocket 即時">
        <svg width={size} height={size}>
          <circle cx={center} cy={center} r={r} fill="none" stroke="#313244" strokeWidth="2" />
          <circle cx={center} cy={center} r={r} fill="none" stroke="#a6e3a1" strokeWidth="2"
            strokeDasharray={c} strokeDashoffset={0}
            strokeLinecap="round" transform={`rotate(-90 ${center} ${center})`} />
          <text x={center} y={center + 1} textAnchor="middle" dominantBaseline="middle"
            fill="#a6e3a1" fontSize="7" fontWeight="600">WS</text>
        </svg>
      </div>
    );
  }

  const remaining = Math.max(0, Math.ceil((interval - (Date.now() - lastFetch)) / 1000));
  const color = progress > 0.85 ? '#f9e2af' : '#89b4fa';

  return (
    <div className="countdown-circle" title={`${remaining}秒後更新`}>
      <svg width={size} height={size}>
        <circle cx={center} cy={center} r={r} fill="none" stroke="#313244" strokeWidth="2" />
        <circle cx={center} cy={center} r={r} fill="none" stroke={color} strokeWidth="2"
          strokeDasharray={c} strokeDashoffset={offset}
          strokeLinecap="round" transform={`rotate(-90 ${center} ${center})`}
          style={{ transition: 'stroke-dashoffset 0.2s linear' }} />
        <text x={center} y={center + 1} textAnchor="middle" dominantBaseline="middle"
          fill="#6c7086" fontSize="8" fontWeight="500">{remaining}</text>
      </svg>
    </div>
  );
}
