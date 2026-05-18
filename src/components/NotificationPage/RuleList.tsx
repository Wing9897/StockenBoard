import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';

interface NotificationRuleRow {
  id: number;
  name: string;
  subscription_id: number;
  condition_type: string;
  threshold: number;
  channel_ids: string;
  cooldown_secs: number;
  enabled: boolean;
  ai_config: string | null;
  created_at: number;
  updated_at: number;
}

interface RuleListProps {
  onAddRule?: () => void;
  onEditRule?: (rule: NotificationRuleRow) => void;
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

function getAiPromptSummary(aiConfig: string | null): string {
  if (!aiConfig) return 'AI 規則';
  try {
    const config = JSON.parse(aiConfig) as { prompt?: string };
    const prompt = config.prompt || '';
    if (prompt.length > 50) {
      return prompt.slice(0, 50) + '…';
    }
    return prompt || 'AI 規則';
  } catch {
    return 'AI 規則';
  }
}


export function RuleList({ onAddRule, onEditRule }: RuleListProps) {
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
    if (!confirm(t.notifications.deleteConfirm)) return;
    try {
      await invoke('delete_notification_rule', { id });
      setRules(prev => prev.filter(r => r.id !== id));
    } catch (e) {
      console.error('Failed to delete rule:', e);
    }
  };

  if (loading) return <div className="notification-placeholder"><p>{t.common.loading}</p></div>;

  return (
    <div className="rule-list">
      <div className="rule-list-header">
        <h3>{t.notifications.rules}</h3>
        {onAddRule && <button className="btn-add-rule" onClick={onAddRule}>+ {t.notifications.addRule}</button>}
      </div>
      {rules.length === 0 ? (
        <div className="notification-placeholder"><p>{t.notifications.noRules}</p></div>
      ) : (
        <div className="rule-items">
          {rules.map(rule => (
            <div key={rule.id} className={`rule-item ${!rule.enabled ? 'disabled' : ''}`}>
              <div className="rule-info">
                <span className="rule-name">
                  {rule.name}
                  {rule.condition_type === 'ai' && <span className="ai-badge">AI</span>}
                </span>
                <span className="rule-condition">
                  {rule.condition_type === 'ai'
                    ? getAiPromptSummary(rule.ai_config)
                    : formatCondition(rule.condition_type, rule.threshold)}
                </span>
              </div>
              <div className="rule-actions">
                {onEditRule && (
                  <button className="btn-edit-rule" onClick={() => onEditRule(rule)} title="編輯">✏️</button>
                )}
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
