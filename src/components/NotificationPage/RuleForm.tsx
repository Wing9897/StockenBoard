import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';

interface Subscription {
  id: number;
  symbol: string;
  selected_provider_id: string;
}

interface ChannelRow {
  id: number;
  channel_type: string;
  name: string;
}

interface EditRuleData {
  id: number;
  name: string;
  subscription_id: number;
  condition_type: string;
  threshold: number;
  channel_ids: string;
  cooldown_secs: number;
  ai_config: string | null;
}

interface RuleFormProps {
  onClose: () => void;
  onSaved: () => void;
  editRule?: EditRuleData;
}

type RuleMode = 'threshold' | 'ai';

const CONDITION_TYPES = [
  { value: 'price_above', label: '價格高於' },
  { value: 'price_below', label: '價格低於' },
  { value: 'change_pct_above', label: '24h漲幅超過' },
  { value: 'change_pct_below', label: '24h跌幅超過' },
];

const ANALYSIS_INTERVAL_OPTIONS = [
  { value: 30, label: '30 秒' },
  { value: 60, label: '1 分鐘' },
  { value: 300, label: '5 分鐘' },
  { value: 600, label: '10 分鐘' },
  { value: 1800, label: '30 分鐘' },
  { value: 3600, label: '1 小時' },
];

export function RuleForm({ onClose, onSaved, editRule }: RuleFormProps) {
  const isEditing = !!editRule;
  const [ruleMode, setRuleMode] = useState<RuleMode>(
    editRule?.condition_type === 'ai' ? 'ai' : 'threshold'
  );
  const [name, setName] = useState(editRule?.name || '');
  const [subscriptionId, setSubscriptionId] = useState<number | ''>(editRule?.subscription_id || '');
  const [conditionType, setConditionType] = useState(
    editRule?.condition_type === 'ai' ? 'ai' : (editRule?.condition_type || 'price_above')
  );
  const [threshold, setThreshold] = useState(editRule ? String(editRule.threshold) : '');
  const [selectedChannels, setSelectedChannels] = useState<number[]>(
    editRule ? JSON.parse(editRule.channel_ids || '[]') : []
  );
  const [cooldownSecs, setCooldownSecs] = useState(editRule ? String(editRule.cooldown_secs) : '300');
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [channels, setChannels] = useState<ChannelRow[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  // AI mode fields - initialize from editRule if available
  const aiConfig = editRule?.ai_config ? JSON.parse(editRule.ai_config) : null;
  const [prompt, setPrompt] = useState(aiConfig?.prompt || '');
  const [historyWindow, setHistoryWindow] = useState(aiConfig?.history_window || 20);
  const [analysisInterval, setAnalysisInterval] = useState(aiConfig?.analysis_interval_secs || 300);

  // AI provider config check
  const [aiProviderConfigured, setAiProviderConfigured] = useState<boolean | null>(null);

  const handleModeChange = (mode: RuleMode) => {
    setRuleMode(mode);
    if (mode === 'ai') {
      setConditionType('ai');
    } else {
      setConditionType('price_above');
    }
  };

  useEffect(() => {
    invoke<Subscription[]>('list_subscriptions').then(setSubscriptions).catch(console.error);
    invoke<ChannelRow[]>('list_notification_channels').then(setChannels).catch(console.error);
    invoke<{ base_url: string; model: string; has_api_key: boolean } | null>('get_ai_provider_config')
      .then(config => setAiProviderConfigured(config !== null))
      .catch(() => setAiProviderConfigured(false));
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError(t.notifications.nameRequired); return; }
    if (subscriptionId === '') { setError(t.notifications.subscriptionRequired); return; }
    if (selectedChannels.length === 0) { setError(t.notifications.channelRequired); return; }

    if (ruleMode === 'threshold') {
      if (!threshold.trim() || isNaN(Number(threshold))) { setError(t.notifications.thresholdRequired); return; }
    }

    if (ruleMode === 'ai') {
      if (!prompt.trim()) { setError(t.notifications.promptRequired); return; }
      if (prompt.trim().length > 2000) { setError(t.notifications.promptTooLong); return; }
      if (historyWindow < 1 || historyWindow > 100) { setError(t.notifications.historyWindowInvalid); return; }
      if (analysisInterval < 30) { setError(t.notifications.intervalInvalid); return; }
    }

    setSaving(true);
    try {
      if (isEditing) {
        await invoke('update_notification_rule', {
          id: editRule!.id,
          rule: {
            name: name.trim(),
            condition_type: ruleMode === 'ai' ? 'ai' : conditionType,
            threshold: ruleMode === 'ai' ? null : Number(threshold),
            channel_ids: selectedChannels,
            cooldown_secs: Number(cooldownSecs) || 300,
            ai_config: ruleMode === 'ai' ? {
              prompt,
              history_window: historyWindow,
              analysis_interval_secs: analysisInterval,
            } : null,
          }
        });
      } else if (ruleMode === 'ai') {
        await invoke('create_notification_rule', {
          rule: {
            name: name.trim(),
            subscription_id: subscriptionId,
            condition_type: 'ai',
            threshold: 0.0,
            channel_ids: selectedChannels,
            cooldown_secs: Number(cooldownSecs) || 300,
            ai_config: {
              prompt,
              history_window: historyWindow,
              analysis_interval_secs: analysisInterval,
            },
          }
        });
      } else {
        await invoke('create_notification_rule', {
          rule: {
            name: name.trim(),
            subscription_id: subscriptionId,
            condition_type: conditionType,
            threshold: Number(threshold),
            channel_ids: selectedChannels,
            cooldown_secs: Number(cooldownSecs) || 300,
          }
        });
      }
      onSaved();
      onClose();
    } catch (e: unknown) {
      setError(typeof e === 'string' ? e : (isEditing ? '更新規則失敗' : '建立規則失敗'));
    } finally {
      setSaving(false);
    }
  };

  const toggleChannel = (id: number) => {
    setSelectedChannels(prev =>
      prev.includes(id) ? prev.filter(c => c !== id) : [...prev, id]
    );
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="rule-form-modal" onClick={e => e.stopPropagation()}>
        <div className="rule-form-header">
          <h3>{isEditing ? t.notifications.editRule : t.notifications.createRule}</h3>
          <button className="btn-close" onClick={onClose}>✕</button>
        </div>
        <form onSubmit={handleSubmit} className="rule-form">
          {error && <div className="rule-form-error">{error}</div>}

          <div className="form-field">
            <span>{t.notifications.ruleMode}</span>
            <div className="rule-mode-toggle">
              <button
                type="button"
                className={`mode-btn ${ruleMode === 'threshold' ? 'active' : ''}`}
                onClick={() => handleModeChange('threshold')}
              >
                {t.notifications.thresholdRule}
              </button>
              <button
                type="button"
                className={`mode-btn ${ruleMode === 'ai' ? 'active' : ''}`}
                onClick={() => handleModeChange('ai')}
              >
                {t.notifications.aiRule}
              </button>
            </div>
          </div>

          <label className="form-field">
            <span>{t.notifications.ruleName}</span>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="例：BTC 突破 65K" />
          </label>

          <label className="form-field">
            <span>{t.notifications.subscription}</span>
            <select value={subscriptionId} onChange={e => setSubscriptionId(Number(e.target.value) || '')}>
              <option value="">{t.notifications.selectSubscription}</option>
              {subscriptions.map(s => (
                <option key={s.id} value={s.id}>{s.symbol} ({s.selected_provider_id})</option>
              ))}
            </select>
          </label>

          {ruleMode === 'threshold' ? (
            <>
              <label className="form-field">
                <span>{t.notifications.conditionType}</span>
                <select value={conditionType} onChange={e => setConditionType(e.target.value)}>
                  {CONDITION_TYPES.map(ct => (
                    <option key={ct.value} value={ct.value}>{ct.label}</option>
                  ))}
                </select>
              </label>

              <label className="form-field">
                <span>{t.notifications.threshold}</span>
                <input type="number" step="any" value={threshold} onChange={e => setThreshold(e.target.value)}
                  placeholder={conditionType.includes('pct') ? '例：5.0 (%)' : '例：65000'} />
              </label>
            </>
          ) : (
            <>
              {aiProviderConfigured === false && (
                <div className="ai-provider-warning">
                  {t.notifications.aiProviderWarning}
                </div>
              )}

              <label className="form-field">
                <span>{t.notifications.promptLabel}</span>
                <textarea
                  className="ai-prompt-textarea"
                  value={prompt}
                  onChange={e => setPrompt(e.target.value)}
                  placeholder={t.notifications.promptPlaceholder}
                  maxLength={2000}
                  rows={4}
                />
                <span className="ai-prompt-counter">{prompt.length} / 2000</span>
              </label>

              <div className="form-field">
                <span>{t.notifications.historyWindow}</span>
                <div className="ai-slider-row">
                  <input
                    type="range"
                    className="ai-slider"
                    min={1}
                    max={100}
                    value={historyWindow}
                    onChange={e => setHistoryWindow(Number(e.target.value))}
                  />
                  <span className="ai-slider-value">{historyWindow}</span>
                </div>
              </div>

              <label className="form-field">
                <span>{t.notifications.analysisInterval}</span>
                <select
                  value={analysisInterval}
                  onChange={e => setAnalysisInterval(Number(e.target.value))}
                >
                  {ANALYSIS_INTERVAL_OPTIONS.map(opt => (
                    <option key={opt.value} value={opt.value}>{opt.label}</option>
                  ))}
                </select>
              </label>
            </>
          )}

          <div className="form-field">
            <span>{t.notifications.channels_label}</span>
            {channels.length === 0 ? (
              <p className="form-hint">{t.notifications.noChannelsHint}</p>
            ) : (
              <div className="channel-checkboxes">
                {channels.map(ch => (
                  <label key={ch.id} className="channel-checkbox">
                    <input type="checkbox" checked={selectedChannels.includes(ch.id)}
                      onChange={() => toggleChannel(ch.id)} />
                    <span>{ch.name} ({ch.channel_type})</span>
                  </label>
                ))}
              </div>
            )}
          </div>

          <label className="form-field">
            <span>{t.notifications.cooldown}</span>
            <input type="number" value={cooldownSecs} onChange={e => setCooldownSecs(e.target.value)} min="0" />
          </label>

          <div className="rule-form-actions">
            <button type="button" className="btn-cancel" onClick={onClose}>{t.common.cancel}</button>
            <button type="submit" className="btn-save" disabled={saving || (ruleMode === 'ai' && aiProviderConfigured === false)}>
              {saving ? t.common.saving : (isEditing ? t.notifications.updateRule : t.notifications.createRule)}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
