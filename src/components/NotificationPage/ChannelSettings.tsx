import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface ChannelRow {
  id: number;
  channel_type: string;
  name: string;
  config: string;
  created_at: number;
}

export function ChannelSettings() {
  const [channels, setChannels] = useState<ChannelRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [channelType, setChannelType] = useState<'telegram' | 'webhook'>('telegram');
  const [name, setName] = useState('');
  const [botToken, setBotToken] = useState('');
  const [chatId, setChatId] = useState('');
  const [webhookUrl, setWebhookUrl] = useState('');
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<number | null>(null);
  const [error, setError] = useState('');
  const [testResult, setTestResult] = useState<{ id: number; success: boolean; msg: string } | null>(null);

  const fetchChannels = useCallback(async () => {
    try {
      const data = await invoke<ChannelRow[]>('list_notification_channels');
      setChannels(data);
    } catch (e) {
      console.error('Failed to fetch channels:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchChannels(); }, [fetchChannels]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError('請輸入通道名稱'); return; }

    let config: string;
    if (channelType === 'telegram') {
      if (!botToken.trim() || !chatId.trim()) { setError('Bot Token 和 Chat ID 不可為空'); return; }
      config = JSON.stringify({ bot_token: botToken.trim(), chat_id: chatId.trim() });
    } else {
      if (!webhookUrl.trim()) { setError('Webhook URL 不可為空'); return; }
      config = JSON.stringify({ url: webhookUrl.trim() });
    }

    setSaving(true);
    try {
      await invoke('save_notification_channel', {
        channel: { channel_type: channelType, name: name.trim(), config }
      });
      setShowForm(false);
      setName(''); setBotToken(''); setChatId(''); setWebhookUrl('');
      await fetchChannels();
    } catch (e: unknown) {
      setError(typeof e === 'string' ? e : '儲存失敗');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('確定要刪除此通道？')) return;
    try {
      await invoke('delete_notification_channel', { id });
      setChannels(prev => prev.filter(c => c.id !== id));
    } catch (e) {
      console.error('Failed to delete channel:', e);
    }
  };

  const handleTest = async (id: number) => {
    setTesting(id);
    setTestResult(null);
    try {
      await invoke('test_notification_channel', { id });
      setTestResult({ id, success: true, msg: '測試成功！' });
    } catch (e: unknown) {
      setTestResult({ id, success: false, msg: typeof e === 'string' ? e : '測試失敗' });
    } finally {
      setTesting(null);
    }
  };

  if (loading) return <div className="notification-placeholder"><p>載入中...</p></div>;

  return (
    <div className="channel-settings">
      <div className="rule-list-header">
        <h3>通知通道</h3>
        <button className="btn-add-rule" onClick={() => setShowForm(true)}>+ 新增通道</button>
      </div>

      {channels.length === 0 && !showForm ? (
        <div className="notification-placeholder"><p>尚無通知通道</p></div>
      ) : (
        <div className="rule-items">
          {channels.map(ch => (
            <div key={ch.id} className="rule-item">
              <div className="rule-info">
                <span className="rule-name">{ch.name}</span>
                <span className="rule-condition">{ch.channel_type === 'telegram' ? '📱 Telegram' : '🔗 Webhook'}</span>
              </div>
              <div className="rule-actions">
                <button className="btn-test" onClick={() => handleTest(ch.id)} disabled={testing === ch.id}>
                  {testing === ch.id ? '測試中...' : '測試'}
                </button>
                {testResult?.id === ch.id && (
                  <span className={testResult.success ? 'test-success' : 'test-fail'}>
                    {testResult.msg}
                  </span>
                )}
                <button className="btn-delete-rule" onClick={() => handleDelete(ch.id)} title="刪除">🗑</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {showForm && (
        <form onSubmit={handleSave} className="channel-form">
          {error && <div className="rule-form-error">{error}</div>}
          <label className="form-field">
            <span>通道名稱</span>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="例：我的 Telegram" />
          </label>
          <label className="form-field">
            <span>通道類型</span>
            <select value={channelType} onChange={e => setChannelType(e.target.value as 'telegram' | 'webhook')}>
              <option value="telegram">Telegram</option>
              <option value="webhook">Webhook</option>
            </select>
          </label>
          {channelType === 'telegram' ? (
            <>
              <label className="form-field">
                <span>Bot Token</span>
                <input type="text" value={botToken} onChange={e => setBotToken(e.target.value)} placeholder="123456:ABC-DEF..." />
              </label>
              <label className="form-field">
                <span>Chat ID</span>
                <input type="text" value={chatId} onChange={e => setChatId(e.target.value)} placeholder="例：-1001234567890" />
              </label>
            </>
          ) : (
            <label className="form-field">
              <span>Webhook URL</span>
              <input type="url" value={webhookUrl} onChange={e => setWebhookUrl(e.target.value)} placeholder="https://..." />
            </label>
          )}
          <div className="rule-form-actions">
            <button type="button" className="btn-cancel" onClick={() => setShowForm(false)}>取消</button>
            <button type="submit" className="btn-save" disabled={saving}>{saving ? '儲存中...' : '儲存'}</button>
          </div>
        </form>
      )}
    </div>
  );
}
