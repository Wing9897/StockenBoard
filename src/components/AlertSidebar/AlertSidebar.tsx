/**
 * AlertSidebar — 懸浮通知面板 + pop-up toast
 *
 * - panelOpen: 點 🔔 展開從右側滑入的懸浮面板（overlay,不擠 dashboard）
 * - 新觸發事件 → 右下角 pop-up 小卡片 3 秒後自動消失
 * - 點 pop-up 或 🔔 可開面板
 */
import { useState, useEffect, useRef, useCallback } from 'react';
import { getTransport } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { formatConditionLabel, formatTimestamp } from '../../lib/format';
import type { NotificationTriggeredEvent } from '../../types';
import './AlertSidebar.css';

export interface AlertItem {
  id: number;
  rule_name: string;
  symbol: string;
  provider: string;
  price: number;
  condition_type: string;
  threshold: number;
  triggered_at: number;
  is_ai: boolean;
  ai_reason: string | null;
}

/**
 * Pure filter function for AlertSidebar items.
 * Applies text search (case-insensitive on rule_name, symbol, ai_reason)
 * and condition type filter as an intersection.
 */
export function filterAlerts(
  items: AlertItem[],
  searchText: string,
  conditionType: string
): AlertItem[] {
  return items.filter(item => {
    // Text search: case-insensitive match on rule_name, symbol, ai_reason
    if (searchText) {
      const lower = searchText.toLowerCase();
      const matchesText =
        item.rule_name.toLowerCase().includes(lower) ||
        item.symbol.toLowerCase().includes(lower) ||
        (item.ai_reason?.toLowerCase().includes(lower) ?? false);
      if (!matchesText) return false;
    }
    // Condition type filter
    if (conditionType !== 'all' && item.condition_type !== conditionType) {
      return false;
    }
    return true;
  });
}

/* ── FilterBar ── */
interface FilterBarProps {
  searchText: string;
  onSearchChange: (text: string) => void;
  conditionFilter: string;
  onConditionChange: (type: string) => void;
  onClear: () => void;
}

function FilterBar({ searchText, onSearchChange, conditionFilter, onConditionChange, onClear }: FilterBarProps) {
  const isActive = searchText !== '' || conditionFilter !== 'all';

  return (
    <div className="alert-filter-bar">
      <input
        type="text"
        className="alert-filter-search"
        placeholder="Search alerts..."
        value={searchText}
        onChange={e => onSearchChange(e.target.value)}
      />
      <select
        className="alert-filter-select"
        value={conditionFilter}
        onChange={e => onConditionChange(e.target.value)}
      >
        <option value="all">All</option>
        <option value="price_above">Price Above</option>
        <option value="price_below">Price Below</option>
        <option value="change_pct_above">Change % Up</option>
        <option value="change_pct_below">Change % Down</option>
        <option value="ai">AI</option>
      </select>
      {isActive && (
        <button className="alert-filter-clear" onClick={onClear} aria-label="Clear filters">
          ✕
        </button>
      )}
    </div>
  );
}

interface Props {
  panelOpen: boolean;
  onClose: () => void;
}

let nextId = 1;

export function AlertSidebar({ panelOpen, onClose }: Props) {
  const [items, setItems] = useState<AlertItem[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [popups, setPopups] = useState<AlertItem[]>([]);
  const listRef = useRef<HTMLDivElement>(null);

  // Filter state
  const [searchText, setSearchText] = useState('');
  const [conditionFilter, setConditionFilter] = useState<string>('all');
  const [debouncedSearch, setDebouncedSearch] = useState('');

  // 200ms debounce for search text
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchText);
    }, 200);
    return () => clearTimeout(timer);
  }, [searchText]);

  // Compute visible items by applying active filters
  const visibleItems = filterAlerts(items, debouncedSearch, conditionFilter);

  const handleClearFilters = useCallback(() => {
    setSearchText('');
    setConditionFilter('all');
  }, []);

  // Load recent history on first panel open
  useEffect(() => {
    if (!panelOpen || loaded) return;
    (async () => {
      try {
        const history = await getTransport().invoke<{
          id: number;
          rule_id: number;
          channel_id: number;
          status: string;
          price: number;
          message: string;
          error: string | null;
          sent_at: number;
        }[]>('get_notification_history', { ruleId: null, from: null, to: null, limit: 30 });

        const initial: AlertItem[] = history.map(h => ({
          id: nextId++,
          rule_name: h.message.slice(0, 40) || `Rule #${h.rule_id}`,
          symbol: '',
          provider: '',
          price: h.price,
          condition_type: '',
          threshold: 0,
          triggered_at: h.sent_at,
          is_ai: false,
          ai_reason: null,
        }));
        setItems(initial.reverse());
      } catch { /* ignore */ }
      setLoaded(true);
    })();
  }, [panelOpen, loaded]);

  // Pop-up auto-dismiss
  const dismissPopup = useCallback((id: number) => {
    setPopups(prev => prev.filter(p => p.id !== id));
  }, []);

  // Subscribe to real-time triggered events
  useEffect(() => {
    const unlisten = getTransport().listen('notification-triggered', (payload) => {
      const evt = payload as NotificationTriggeredEvent;
      const item: AlertItem = {
        id: nextId++,
        rule_name: evt.rule_name,
        symbol: evt.symbol,
        provider: evt.provider,
        price: evt.price,
        condition_type: evt.condition_type,
        threshold: evt.threshold,
        triggered_at: evt.triggered_at,
        is_ai: evt.is_ai,
        ai_reason: evt.ai_reason,
      };
      // Add to history list
      setItems(prev => [...prev.slice(-99), item]);
      // Show pop-up (auto-dismiss after 4 seconds)
      setPopups(prev => [...prev.slice(-2), item]);
      setTimeout(() => dismissPopup(item.id), 4000);
      // Auto-scroll panel if open
      setTimeout(() => listRef.current?.scrollTo({ top: listRef.current.scrollHeight, behavior: 'smooth' }), 50);
    });

    return () => { unlisten(); };
  }, [dismissPopup]);

  return (
    <>
      {/* Pop-up toasts — always visible, bottom-right */}
      {popups.length > 0 && (
        <div className="alert-popups">
          {popups.map(item => (
            <div key={item.id} className="alert-popup" onClick={onClose}>
              <div className="alert-popup-header">
                <span className="alert-popup-rule">{item.rule_name}</span>
                {item.is_ai && <span className="alert-card-ai-badge">AI</span>}
              </div>
              {item.symbol && <span className="alert-popup-symbol">{item.symbol} {item.price > 0 && `$${item.price.toLocaleString()}`}</span>}
              {item.is_ai && item.ai_reason && <span className="alert-popup-reason">{item.ai_reason.slice(0, 60)}</span>}
            </div>
          ))}
        </div>
      )}

      {/* Floating panel overlay — from right side */}
      {panelOpen && (
        <div className="alert-panel-backdrop" onClick={onClose}>
          <div className="alert-panel" onClick={e => e.stopPropagation()}>
            <div className="alert-panel-header">
              <h3>{t.nav.notifications}</h3>
              <button className="alert-panel-close" onClick={onClose} aria-label={t.common.close}>✕</button>
            </div>
            <FilterBar
              searchText={searchText}
              onSearchChange={setSearchText}
              conditionFilter={conditionFilter}
              onConditionChange={setConditionFilter}
              onClear={handleClearFilters}
            />
            <div className="alert-panel-list" ref={listRef}>
              {items.length === 0 ? (
                <div className="alert-panel-empty">{t.notifications.noHistory}</div>
              ) : visibleItems.length === 0 ? (
                <div className="alert-panel-empty">{t.notifications.noFilterResults}</div>
              ) : (
                visibleItems.map(item => (
                  <div key={item.id} className="alert-card">
                    <div className="alert-card-header">
                      <span className="alert-card-rule">{item.rule_name}</span>
                      {item.is_ai && <span className="alert-card-ai-badge">AI</span>}
                    </div>
                    {item.symbol && (
                      <div className="alert-card-row">
                        <span className="alert-card-symbol">{item.symbol}</span>
                        {item.provider && <span className="alert-card-provider">{item.provider}</span>}
                      </div>
                    )}
                    <div className="alert-card-row">
                      <span className="alert-card-condition">
                        {item.condition_type ? formatConditionLabel(item.condition_type, item.threshold) : ''}
                      </span>
                      {item.price > 0 && <span className="alert-card-price">${item.price.toLocaleString()}</span>}
                    </div>
                    {item.is_ai && item.ai_reason && (
                      <div className="alert-card-reason">{item.ai_reason}</div>
                    )}
                    <div className="alert-card-time">{formatTimestamp(item.triggered_at)}</div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
