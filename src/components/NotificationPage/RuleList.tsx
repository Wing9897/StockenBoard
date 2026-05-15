import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface NotificationRuleRow {
  id: number;
  name: string;
  subscription_id: number;
  condition_type: string;
  threshold: number;
  channel_ids: string;
  cooldown_secs: number;
  enabled: boolean;
  created_at: number;
  updated_at: number;
}

interface RuleListProps {
  onAddRule?: () => void;
}

function formatCondition(conditionType: string, threshold: number): string {
  switch (conditionType) {
    case 'price_above': return `價格 > $${threshold.toLocaleString()}`;
    case 'price_below': return `價格 < $${threshold.toLocaleString()}`;
    case 'change_pct_above': return `漲幅 > ${threshold}%`;
    case 'change_pct_below': return `跌幅 < ${threshold}%`;
    default: return conditionType;
  }
}

export function RuleList({ onAddRule }: RuleListProps) {
  const [rules, setRules] = useState<NotificationRuleRow[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchRules = useCallback(async () => {
    try {
      const data = await invoke<NotificationRuleRow[]>('list_notification_rules');
      setRules(data);
    } catch (e) {
      console.error('Failed to fetch rules:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchRules(); }, [fetchRules]);

  const handleToggle = async (id: number, currentEnabled: boolean) => {
    try {
      await invoke('toggle_notification_rule', { id, enabled: !currentEnabled });
      setRules(prev => prev.map(r => r.id === id ? { ...r, enabled: !currentEnabled } : r));
    } catch (e) {
      console.error('Failed to toggle rule:', e);
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('確定要刪除此規則？')) return;
    try {
      await invoke('delete_notification_rule', { id });
      setRules(prev => prev.filter(r => r.id !== id));
    } catch (e) {
      console.error('Failed to delete rule:', e);
    }
  };

  if (loading) return <div className="notification-placeholder"><p>載入中...</p></div>;

  return (
    <div className="rule-list">
      <div className="rule-list-header">
        <h3>通知規則</h3>
        {onAddRule && <button className="btn-add-rule" onClick={onAddRule}>+ 新增規則</button>}
      </div>
      {rules.length === 0 ? (
        <div className="notification-placeholder"><p>尚無通知規則</p></div>
      ) : (
        <div className="rule-items">
          {rules.map(rule => (
            <div key={rule.id} className={`rule-item ${!rule.enabled ? 'disabled' : ''}`}>
              <div className="rule-info">
                <span className="rule-name">{rule.name}</span>
                <span className="rule-condition">{formatCondition(rule.condition_type, rule.threshold)}</span>
              </div>
              <div className="rule-actions">
                <label className="toggle-switch">
                  <input type="checkbox" checked={rule.enabled} onChange={() => handleToggle(rule.id, rule.enabled)} />
                  <span className="toggle-slider"></span>
                </label>
                <button className="btn-delete-rule" onClick={() => handleDelete(rule.id)} title="刪除">🗑</button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
