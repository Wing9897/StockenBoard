import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

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

interface RuleFormProps {
  onClose: () => void;
  onSaved: () => void;
}

const CONDITION_TYPES = [
  { value: 'price_above', label: '價格高於' },
  { value: 'price_below', label: '價格低於' },
  { value: 'change_pct_above', label: '24h漲幅超過' },
  { value: 'change_pct_below', label: '24h跌幅超過' },
];

export function RuleForm({ onClose, onSaved }: RuleFormProps) {
  const [name, setName] = useState('');
  const [subscriptionId, setSubscriptionId] = useState<number | ''>('');
  const [conditionType, setConditionType] = useState('price_above');
  const [threshold, setThreshold] = useState('');
  const [selectedChannels, setSelectedChannels] = useState<number[]>([]);
  const [cooldownSecs, setCooldownSecs] = useState('300');
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [channels, setChannels] = useState<ChannelRow[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    invoke<Subscription[]>('list_subscriptions').then(setSubscriptions).catch(console.error);
    invoke<ChannelRow[]>('list_notification_channels').then(setChannels).catch(console.error);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError('請輸入規則名稱'); return; }
    if (subscriptionId === '') { setError('請選擇訂閱'); return; }
    if (!threshold.trim() || isNaN(Number(threshold))) { setError('請輸入有效閾值'); return; }
    if (selectedChannels.length === 0) { setError('請至少選擇一個通道'); return; }

    setSaving(true);
    try {
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
      onSaved();
      onClose();
    } catch (e: unknown) {
      setError(typeof e === 'string' ? e : '建立規則失敗');
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
          <h3>新增通知規則</h3>
          <button className="btn-close" onClick={onClose}>✕</button>
        </div>
        <form onSubmit={handleSubmit} className="rule-form">
          {error && <div className="rule-form-error">{error}</div>}

          <label className="form-field">
            <span>規則名稱</span>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="例：BTC 突破 65K" />
          </label>

          <label className="form-field">
            <span>訂閱</span>
            <select value={subscriptionId} onChange={e => setSubscriptionId(Number(e.target.value) || '')}>
              <option value="">-- 選擇訂閱 --</option>
              {subscriptions.map(s => (
                <option key={s.id} value={s.id}>{s.symbol} ({s.selected_provider_id})</option>
              ))}
            </select>
          </label>

          <label className="form-field">
            <span>條件類型</span>
            <select value={conditionType} onChange={e => setConditionType(e.target.value)}>
              {CONDITION_TYPES.map(ct => (
                <option key={ct.value} value={ct.value}>{ct.label}</option>
              ))}
            </select>
          </label>

          <label className="form-field">
            <span>閾值</span>
            <input type="number" step="any" value={threshold} onChange={e => setThreshold(e.target.value)}
              placeholder={conditionType.includes('pct') ? '例：5.0 (%)' : '例：65000'} />
          </label>

          <div className="form-field">
            <span>通知通道</span>
            {channels.length === 0 ? (
              <p className="form-hint">尚無通道，請先至「通道設定」新增</p>
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
            <span>冷卻期（秒）</span>
            <input type="number" value={cooldownSecs} onChange={e => setCooldownSecs(e.target.value)} min="0" />
          </label>

          <div className="rule-form-actions">
            <button type="button" className="btn-cancel" onClick={onClose}>取消</button>
            <button type="submit" className="btn-save" disabled={saving}>
              {saving ? '儲存中...' : '建立規則'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
