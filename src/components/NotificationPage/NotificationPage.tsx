import { useState, useCallback } from 'react';
import { RuleList } from './RuleList';
import { RuleForm } from './RuleForm';
import { ChannelSettings } from './ChannelSettings';
import { NotificationHistory } from './NotificationHistory';
import { AiSettings } from './AiSettings';
import { t } from '../../lib/i18n';
import './NotificationPage.css';

type NotificationTab = 'rules' | 'channels' | 'history' | 'ai-settings';

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
          <RuleList key={ruleListKey} onAddRule={handleAddRule} onEditRule={handleEditRule} />
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
