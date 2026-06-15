import { useState, useEffect, useCallback } from 'react';
import { transport } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { silentLog } from '../../lib/errorLog';
import { useConfirm } from '../../hooks/useConfirm';
import { ConfirmDialog } from '../ConfirmDialog/ConfirmDialog';
import type { NotificationRuleRow } from '../../types';

interface RuleListProps {
  onAddRule?: () => void;
  onEditRule?: (rule: NotificationRuleRow) => void;
}

function formatCondition(conditionType: string, threshold: number): string {
  switch (conditionType) {
    case 'price_above': return t.notifications.condPriceAbove(threshold.toLocaleString());
    case 'price_below': return t.notifications.condPriceBelow(threshold.toLocaleString());
    case 'change_pct_above': return t.notifications.condChangeUp(String(threshold));
    case 'change_pct_below': return t.notifications.condChangeDown(String(threshold));
    default: return conditionType;
  }
}

function getAiPromptSummary(aiConfig: string | null): string {
  if (!aiConfig) return t.notifications.aiRule;
  try {
    const config = JSON.parse(aiConfig) as { prompt?: string };
    const prompt = config.prompt || '';
    if (prompt.length > 50) {
      return prompt.slice(0, 50) + '…';
    }
    return prompt || t.notifications.aiRule;
  } catch {
    return t.notifications.aiRule;
  }
}


export function RuleList({ onAddRule, onEditRule }: RuleListProps) {
  const [rules, setRules] = useState<NotificationRuleRow[]>([]);
  const [loading, setLoading] = useState(true);
  const { confirmState, requestConfirm, handleConfirm, handleCancel } = useConfirm();

  const fetchRules = useCallback(async () => {
    try {
      const data = await transport.invoke<NotificationRuleRow[]>('list_notification_rules');
      setRules(data);
    } catch (e) {
      silentLog('RuleList.fetchRules', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchRules(); }, [fetchRules]);

  const handleToggle = async (id: number, currentEnabled: boolean) => {
    try {
      await transport.invoke('toggle_notification_rule', { id, enabled: !currentEnabled });
      setRules(prev => prev.map(r => r.id === id ? { ...r, enabled: !currentEnabled } : r));
    } catch (e) {
      silentLog('RuleList.toggle', e);
    }
  };

  const handleDelete = async (id: number) => {
    const ok = await requestConfirm(t.notifications.deleteConfirm);
    if (!ok) return;
    try {
      await transport.invoke('delete_notification_rule', { id });
      setRules(prev => prev.filter(r => r.id !== id));
    } catch (e) {
      silentLog('RuleList.delete', e);
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
                  <button className="btn-edit-rule" onClick={() => onEditRule(rule)} title={t.common.edit}>✏️</button>
                )}
                <label className="toggle-switch">
                  <input type="checkbox" checked={rule.enabled} onChange={() => handleToggle(rule.id, rule.enabled)} />
                  <span className="toggle-slider"></span>
                </label>
                <button className="btn-delete-rule" onClick={() => handleDelete(rule.id)} title={t.common.delete}>🗑</button>
              </div>
            </div>
          ))}
        </div>
      )}
      {confirmState && (
        <ConfirmDialog message={confirmState.message} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  );
}
