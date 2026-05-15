import { useState, useCallback } from 'react';
import { RuleList } from './RuleList';
import { RuleForm } from './RuleForm';
import { ChannelSettings } from './ChannelSettings';
import { NotificationHistory } from './NotificationHistory';
import './NotificationPage.css';

type NotificationTab = 'rules' | 'channels' | 'history';

export function NotificationPage() {
  const [activeTab, setActiveTab] = useState<NotificationTab>('rules');
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [ruleListKey, setRuleListKey] = useState(0);

  const handleRuleSaved = useCallback(() => {
    setRuleListKey(prev => prev + 1);
  }, []);

  return (
    <div className="notification-page">
      <div className="notification-tabs">
        <button
          className={`notification-tab ${activeTab === 'rules' ? 'active' : ''}`}
          onClick={() => setActiveTab('rules')}
        >
          規則列表
        </button>
        <button
          className={`notification-tab ${activeTab === 'channels' ? 'active' : ''}`}
          onClick={() => setActiveTab('channels')}
        >
          通道設定
        </button>
        <button
          className={`notification-tab ${activeTab === 'history' ? 'active' : ''}`}
          onClick={() => setActiveTab('history')}
        >
          歷史紀錄
        </button>
      </div>

      <div className="notification-content">
        {activeTab === 'rules' && (
          <RuleList key={ruleListKey} onAddRule={() => setShowRuleForm(true)} />
        )}
        {activeTab === 'channels' && (
          <ChannelSettings />
        )}
        {activeTab === 'history' && (
          <NotificationHistory />
        )}
      </div>

      {showRuleForm && (
        <RuleForm onClose={() => setShowRuleForm(false)} onSaved={handleRuleSaved} />
      )}
    </div>
  );
}
