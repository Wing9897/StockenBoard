/**
 * 歷史圖表元件 — 從 HistoryPage.tsx 抽出
 */
import { useEffect, useRef, useMemo, useCallback } from 'react';
import { createChart, LineSeries, type IChartApi } from 'lightweight-charts';
import type { PriceHistoryRecord } from '../../types';
import type { Time } from 'lightweight-charts';

type SessionFilter = 'regular' | 'pre' | 'post';

/** 本地時區偏移（秒） */
const TZ_OFFSET_SEC = -new Date().getTimezoneOffset() * 60;

interface HistoryChartProps {
  records: PriceHistoryRecord[];
  session: SessionFilter;
}

export function HistoryChart({ records, session }: HistoryChartProps) {
  const chartRef = useRef<HTMLDivElement>(null);
  const chartApi = useRef<IChartApi | null>(null);

  const getPrice = useCallback((r: PriceHistoryRecord) => {
    if (session === 'pre' && r.pre_price != null) return r.pre_price;
    if (session === 'post' && r.post_price != null) return r.post_price;
    return r.price;
  }, [session]);

  const chartData = useMemo(() =>
    records.map(r => ({ time: (r.recorded_at + TZ_OFFSET_SEC) as Time, value: getPrice(r) })),
  [records, getPrice]);

  const chartColor = useCallback(() => {
    const cs = getComputedStyle(document.documentElement);
    if (session === 'pre') return cs.getPropertyValue('--pre-market-color').trim() || 'orange';
    if (session === 'post') return cs.getPropertyValue('--post-market-color').trim() || 'purple';
    return cs.getPropertyValue('--accent').trim() || cs.getPropertyValue('--blue').trim() || 'steelblue';
  }, [session]);

  useEffect(() => {
    if (!chartRef.current) return;
    chartApi.current?.remove();
    chartApi.current = null;
    if (!chartData.length) return;

    const el = chartRef.current;
    const cs = getComputedStyle(document.documentElement);
    const chart = createChart(el, {
      width: el.clientWidth, height: el.clientHeight,
      layout: { background: { color: 'transparent' }, textColor: cs.getPropertyValue('--subtext0').trim() || 'gray' },
      grid: { vertLines: { color: 'rgba(128,128,128,0.1)' }, horzLines: { color: 'rgba(128,128,128,0.1)' } },
      timeScale: { timeVisible: true, secondsVisible: false },
      crosshair: { mode: 0 },
    });
    chartApi.current = chart;
    const s = chart.addSeries(LineSeries, { color: chartColor(), lineWidth: 2 });
    s.setData(chartData);
    chart.timeScale().fitContent();

    const ro = new ResizeObserver(() => chart.applyOptions({ width: el.clientWidth, height: el.clientHeight }));
    ro.observe(el);
    return () => { ro.disconnect(); chart.remove(); chartApi.current = null; };
  }, [chartData, chartColor]);

  return (
    <div className="h-card history-chart-wrapper">
      <div className="history-chart-container" ref={chartRef} />
    </div>
  );
}
