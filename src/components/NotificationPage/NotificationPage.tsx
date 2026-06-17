import { useState, useCallback, useEffect } from 'react';
import { RuleList } from './RuleList';
import { RuleForm } from './RuleForm';
import { ChannelSettings } from './ChannelSettings';
import { AiSettings } from './AiSettings';
import { GlobalCooldownInline } from './GlobalCooldownInline';
import { t } from '../../lib/i18n';
import { STORAGE_KEYS } from '../../lib/storageKeys';
import type { EditRuleData } from '../../types';
import './NotificationPage.css';

type NotificationTab = 'rules' | 'channels' | 'ai-settings';

export function NotificationPage() {
  const [activeTab, setActiveTab] = useState<NotificationTab>(
    () => (localStorage.getItem(STORAGE_KEYS.NOTIFICATION_TAB) as NotificationTab) || 'rules'
  );
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [editingRule, setEditingRule] = useState<EditRuleData | undefined>(undefined);
  const [ruleListKey, setRuleListKey] = useState(0);

  useEffect(() => { localStorage.setItem(STORAGE_KEYS.NOTIFICATION_TAB, activeTab); }, [activeTab]);

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
          className={`notification-tab ${activeTab === 'ai-settings' ? 'active' : ''}`}
          onClick={() => setActiveTab('ai-settings')}
        >
          {t.notifications.aiSettings}
        </button>
        {activeTab === 'rules' && (
          <>
            <GlobalCooldownInline />
            <button className="btn-add-rule" onClick={handleAddRule}>+ {t.notifications.addRule}</button>
          </>
        )}
      </div>

      <div className="notification-content">
        {activeTab === 'rules' && (
          <RuleList key={ruleListKey} onEditRule={handleEditRule} />
        )}
        {activeTab === 'channels' && (
          <ChannelSettings />
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
