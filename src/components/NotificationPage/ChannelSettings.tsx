import { useState, useEffect, useCallback } from 'react';
import { getTransport } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { silentLog } from '../../lib/errorLog';
import { useConfirm } from '../../hooks/useConfirm';
import { ConfirmDialog } from '../ConfirmDialog/ConfirmDialog';
import type { ChannelRow } from '../../types';

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
  const { confirmState, requestConfirm, handleConfirm, handleCancel } = useConfirm();

  const fetchChannels = useCallback(async () => {
    try {
      const data = await getTransport().invoke<ChannelRow[]>('list_notification_channels');
      setChannels(data);
    } catch (e) {
      silentLog('ChannelSettings.fetchChannels', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchChannels(); }, [fetchChannels]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError(t.notifications.channelNameRequired); return; }

    let config: string;
    if (channelType === 'telegram') {
      if (!botToken.trim() || !chatId.trim()) { setError(t.notifications.telegramFieldsRequired); return; }
      config = JSON.stringify({ bot_token: botToken.trim(), chat_id: chatId.trim() });
    } else {
      if (!webhookUrl.trim()) { setError(t.notifications.webhookUrlRequired); return; }
      config = JSON.stringify({ url: webhookUrl.trim() });
    }

    setSaving(true);
    try {
      await getTransport().invoke('save_notification_channel', {
        channel: { channel_type: channelType, name: name.trim(), config }
      });
      setShowForm(false);
      setName(''); setBotToken(''); setChatId(''); setWebhookUrl('');
      await fetchChannels();
    } catch (e: unknown) {
      setError(typeof e === 'string' ? e : t.notifications.saveFailed);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: number) => {
    const ok = await requestConfirm(t.notifications.deleteChannelConfirm);
    if (!ok) return;
    try {
      await getTransport().invoke('delete_notification_channel', { id });
      setChannels(prev => prev.filter(c => c.id !== id));
    } catch (e) {
      silentLog('ChannelSettings.delete', e);
    }
  };

  const handleTest = async (id: number) => {
    setTesting(id);
    setTestResult(null);
    try {
      await getTransport().invoke('test_notification_channel', { id });
      setTestResult({ id, success: true, msg: t.notifications.testOk });
    } catch (e: unknown) {
      setTestResult({ id, success: false, msg: typeof e === 'string' ? e : t.notifications.testFailed });
    } finally {
      setTesting(null);
    }
  };

  if (loading) return <div className="notification-placeholder"><p>{t.common.loading}</p></div>;

  return (
    <div className="channel-settings">
      <div className="rule-list-header">
        <h3>{t.notifications.channels_label}</h3>
        <button className="btn-add-rule" onClick={() => setShowForm(true)}>+ {t.notifications.addChannel}</button>
      </div>

      {channels.length === 0 && !showForm ? (
        <div className="notification-placeholder"><p>{t.notifications.noChannels}</p></div>
      ) : (
        <div className="rule-items">
          {channels.map(ch => (
            <div key={ch.id} className="rule-item">
              <div className="rule-info">
                <span className="rule-name">{ch.name}</span>
                <span className="rule-condition">
                  {ch.channel_type === 'telegram' ? '📱 Telegram'
                   : ch.channel_type === 'webhook' ? '🔗 Webhook'
                   : ch.channel_type === 'system' ? t.notifications.systemChannel
                   : t.notifications.localChannel}
                </span>
              </div>
              <div className="rule-actions">
                <button className="btn-test" onClick={() => handleTest(ch.id)} disabled={testing === ch.id}>
                  {testing === ch.id ? t.notifications.testing : t.notifications.test}
                </button>
                {testResult?.id === ch.id && (
                  <span className={testResult.success ? 'test-success' : 'test-fail'}>
                    {testResult.msg}
                  </span>
                )}
                {ch.channel_type !== 'local' && ch.channel_type !== 'system' && (
                  <button className="btn-delete-rule" onClick={() => handleDelete(ch.id)} title={t.common.delete}>🗑</button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {showForm && (
        <form onSubmit={handleSave} className="channel-form">
          {error && <div className="rule-form-error">{error}</div>}
          <label className="form-field">
            <span>{t.notifications.channelName}</span>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder={t.notifications.channelNamePlaceholder} />
          </label>
          <label className="form-field">
            <span>{t.notifications.channelType}</span>
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
                <input type="text" value={chatId} onChange={e => setChatId(e.target.value)} placeholder={t.notifications.chatIdPlaceholder} />
              </label>
            </>
          ) : (
            <label className="form-field">
              <span>Webhook URL</span>
              <input type="url" value={webhookUrl} onChange={e => setWebhookUrl(e.target.value)} placeholder="https://..." />
            </label>
          )}
          <div className="rule-form-actions">
            <button type="button" className="btn-cancel" onClick={() => setShowForm(false)}>{t.common.cancel}</button>
            <button type="submit" className="btn-save" disabled={saving}>{saving ? t.common.saving : t.common.save}</button>
          </div>
        </form>
      )}

      {confirmState && (
        <ConfirmDialog message={confirmState.message} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  );
}
