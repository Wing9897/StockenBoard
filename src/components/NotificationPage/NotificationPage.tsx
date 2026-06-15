import { useState, useEffect, useCallback, useRef } from 'react';
import { transport } from '../../lib/transport';
import { RuleList } from './RuleList';
import { RuleForm } from './RuleForm';
import { ChannelSettings } from './ChannelSettings';
import { NotificationHistory } from './NotificationHistory';
import { AiSettings } from './AiSettings';
import { t } from '../../lib/i18n';
import { silentLog } from '../../lib/errorLog';
import type { EditRuleData } from '../../types';
import './NotificationPage.css';

type NotificationTab = 'rules' | 'channels' | 'history' | 'ai-settings';

/** Global Cooldown settings block shown above the rule list */
function GlobalCooldownSettings() {
  const [cooldown, setCooldown] = useState(30);
  const [loading, setLoading] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    transport.invoke<number>('get_notification_global_cooldown')
      .then(val => setCooldown(val))
      .catch(e => silentLog('GlobalCooldown.load', e))
      .finally(() => setLoading(false));
  }, []);

  const handleChange = useCallback((value: number) => {
    setCooldown(value);
    // Debounce backend writes to avoid spamming while dragging the slider
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      transport.invoke('set_notification_global_cooldown', { secs: value })
        .catch(e => silentLog('GlobalCooldown.save', e));
    }, 300);
  }, []);

  if (loading) return null;

  return (
    <div className="global-cooldown-section">
      <div className="form-field">
        <span className="global-cooldown-label">{t.notifications.globalCooldown}</span>
        <p className="form-hint">{t.notifications.globalCooldownDesc}</p>
        <div className="ai-slider-row">
          <input
            type="range"
            className="ai-slider"
            min={0}
            max={3600}
            step={1}
            value={cooldown}
            onChange={e => handleChange(Number(e.target.value))}
          />
          <input
            type="number"
            className="ai-slider-value"
            min={0}
            max={3600}
            value={cooldown}
            onChange={e => {
              const v = Math.max(0, Math.min(3600, Number(e.target.value) || 0));
              handleChange(v);
            }}
          />
          <span className="global-cooldown-unit">{t.notifications.globalCooldownUnit}</span>
        </div>
      </div>
    </div>
  );
}

export function NotificationPage() {
  const [activeTab, setActiveTab] = useState<NotificationTab>('rules');
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [editingRule, setEditingRule] = useState<EditRuleData | undefined>(undefined);
  const [ruleListKey, setRuleListKey] = useState(0);

  const handleRuleSaved = useCallback(() => {
    setRuleListKey(prev => prev + 1);
  }, []);

  const handleAddRule = useCallback(() => {
    setEditingRule(undefined);
    setShowRuleForm(true);
  }, []);

  const handleEditRule = useCallback((rule: EditRuleData) => {
    setEditingRule(rule);
    setShowRuleForm(true);
  }, []);

  const handleCloseForm = useCallback(() => {
    setShowRuleForm(false);
    setEditingRule(undefined);
  }, []);

  return (
    <div className="notification-page">
      <div className="notification-tabs">
        <button
          className={`notification-tab ${activeTab === 'rules' ? 'active' : ''}`}
          onClick={() => setActiveTab('rules')}
        >
          {t.notifications.rules}
        </button>
        <button
          className={`notification-tab ${activeTab === 'channels' ? 'active' : ''}`}
          onClick={() => setActiveTab('channels')}
        >
          {t.notifications.channels}
        </button>
        <button
          className={`notification-tab ${activeTab === 'history' ? 'active' : ''}`}
          onClick={() => setActiveTab('history')}
        >
          {t.notifications.history}
        </button>
        <button
          className={`notification-tab ${activeTab === 'ai-settings' ? 'active' : ''}`}
          onClick={() => setActiveTab('ai-settings')}
        >
          {t.notifications.aiSettings}
        </button>
      </div>

      <div className="notification-content">
        {activeTab === 'rules' && (
          <>
            <GlobalCooldownSettings />
            <RuleList key={ruleListKey} onAddRule={handleAddRule} onEditRule={handleEditRule} />
          </>
        )}
        {activeTab === 'channels' && (
          <ChannelSettings />
        )}
        {activeTab === 'history' && (
          <NotificationHistory />
        )}
        {activeTab === 'ai-settings' && (
          <AiSettings />
        )}
      </div>

      {showRuleForm && (
        <RuleForm onClose={handleCloseForm} onSaved={handleRuleSaved} editRule={editingRule} />
      )}
    </div>
  );
}
