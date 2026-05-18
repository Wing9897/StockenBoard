import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';

interface AiProviderConfigResponse {
  base_url: string;
  model: string;
  has_api_key: boolean;
}

type ProviderType = 'ollama' | 'openai' | 'openrouter' | 'custom';

const PROVIDER_PRESETS: Record<ProviderType, { baseUrl: string; needsKey: boolean; label: string }> = {
  ollama: { baseUrl: 'http://localhost:11434/v1', needsKey: false, label: 'Ollama (本地)' },
  openai: { baseUrl: 'https://api.openai.com/v1', needsKey: true, label: 'OpenAI' },
  openrouter: { baseUrl: 'https://openrouter.ai/api/v1', needsKey: true, label: 'OpenRouter' },
  custom: { baseUrl: '', needsKey: false, label: '自訂' },
};

export function AiSettings() {
  const [providerType, setProviderType] = useState<ProviderType>('ollama');
  const [baseUrl, setBaseUrl] = useState('http://localhost:11434/v1');
  const [model, setModel] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [feedback, setFeedback] = useState<{ type: 'success' | 'error'; msg: string } | null>(null);
  const [testResult, setTestResult] = useState<{ type: 'success' | 'error'; msg: string } | null>(null);

  // Model list from API
  const [modelList, setModelList] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);

  useEffect(() => { loadConfig(); }, []);

  const loadConfig = async () => {
    try {
      const config = await invoke<AiProviderConfigResponse | null>('get_ai_provider_config');
      if (config) {
        setBaseUrl(config.base_url);
        setModel(config.model);
        // Detect provider type from URL
        if (config.base_url.includes('localhost:11434')) {
          setProviderType('ollama');
        } else if (config.base_url.includes('api.openai.com')) {
          setProviderType('openai');
        } else if (config.base_url.includes('openrouter.ai')) {
          setProviderType('openrouter');
        } else {
          setProviderType('custom');
        }
      }
    } catch (e) {
      console.error('Failed to load AI provider config:', e);
    } finally {
      setLoading(false);
    }
  };

  const fetchModels = useCallback(async (url?: string) => {
    const targetUrl = url || baseUrl;
    if (!targetUrl.trim()) return;
    setLoadingModels(true);
    try {
      const models = await invoke<string[]>('list_ai_models', {
        baseUrl: targetUrl.trim(),
        apiKey: apiKey.trim() || null,
      });
      setModelList(models);
      // Auto-select first model if none selected
      if (!model && models.length > 0) {
        setModel(models[0]);
      }
    } catch {
      setModelList([]);
    } finally {
      setLoadingModels(false);
    }
  }, [baseUrl, apiKey, model]);

  const handleProviderChange = (type: ProviderType) => {
    setProviderType(type);
    const preset = PROVIDER_PRESETS[type];
    setBaseUrl(preset.baseUrl);
    setModelList([]);
    setModel('');
    if (preset.baseUrl) {
      fetchModels(preset.baseUrl);
    }
  };

  // Fetch models when base URL changes (with debounce on mount)
  useEffect(() => {
    if (!loading && baseUrl.trim()) {
      const timer = setTimeout(() => fetchModels(), 500);
      return () => clearTimeout(timer);
    }
  }, [loading, baseUrl]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setFeedback(null);

    if (!baseUrl.trim()) {
      setFeedback({ type: 'error', msg: t.notifications.baseUrlRequired });
      return;
    }
    if (!model.trim()) {
      setFeedback({ type: 'error', msg: t.notifications.modelRequired });
      return;
    }

    setSaving(true);
    try {
      await invoke('save_ai_provider_config', {
        base_url: baseUrl.trim(),
        model: model.trim(),
        api_key: apiKey.trim() || null,
      });
      setFeedback({ type: 'success', msg: t.notifications.saveSuccess });
      setApiKey('');
    } catch (e: unknown) {
      setFeedback({ type: 'error', msg: typeof e === 'string' ? e : t.notifications.saveFailed });
    } finally {
      setSaving(false);
    }
  };

  const handleTestConnection = async () => {
    setTestResult(null);
    setTesting(true);
    try {
      const result = await invoke<string>('test_ai_connection');
      setTestResult({ type: 'success', msg: `${t.notifications.testSuccess}: ${result}` });
    } catch (e: unknown) {
      setTestResult({ type: 'error', msg: typeof e === 'string' ? e : t.notifications.testFailed });
    } finally {
      setTesting(false);
    }
  };

  if (loading) return <div className="notification-placeholder"><p>{t.common.loading}</p></div>;

  return (
    <div className="ai-settings">
      <div className="rule-list-header">
        <h3>{t.notifications.aiSettingsTitle}</h3>
      </div>

      <form onSubmit={handleSave} className="ai-settings-form">
        {feedback && (
          <div className={feedback.type === 'success' ? 'ai-settings-success' : 'rule-form-error'}>
            {feedback.msg}
          </div>
        )}

        <div className="form-field">
          <span>服務類型</span>
          <div className="rule-mode-toggle">
            {(Object.entries(PROVIDER_PRESETS) as [ProviderType, typeof PROVIDER_PRESETS['ollama']][]).map(([key, preset]) => (
              <button
                key={key}
                type="button"
                className={`mode-btn ${providerType === key ? 'active' : ''}`}
                onClick={() => handleProviderChange(key)}
              >
                {preset.label}
              </button>
            ))}
          </div>
        </div>

        <label className="form-field">
          <span>{t.notifications.baseUrl}</span>
          <input
            type="text"
            value={baseUrl}
            onChange={e => setBaseUrl(e.target.value)}
            placeholder="http://localhost:11434/v1"
            disabled={providerType !== 'custom'}
          />
          {providerType === 'ollama' && (
            <p className="form-hint">本地 Ollama 預設端點，確保 Ollama 正在運行</p>
          )}
          {providerType === 'openai' && (
            <p className="form-hint">OpenAI 官方 API 端點</p>
          )}
          {providerType === 'openrouter' && (
            <p className="form-hint">OpenRouter 統一 API，支援數百種模型（需 API Key）</p>
          )}
        </label>

        <div className="form-field">
          <span>模型 {loadingModels && <small>（載入中...）</small>}</span>
          {modelList.length > 0 ? (
            <div className="ai-model-select-row">
              <select value={model} onChange={e => setModel(e.target.value)}>
                <option value="">-- 選擇模型 --</option>
                {modelList.map(m => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
              <button type="button" className="btn-refresh" onClick={() => fetchModels()} title="重新整理">🔄</button>
            </div>
          ) : (
            <input
              type="text"
              value={model}
              onChange={e => setModel(e.target.value)}
              placeholder={providerType === 'ollama' ? 'llama3.1:8b' : providerType === 'openai' ? 'gpt-4o' : providerType === 'openrouter' ? 'meta-llama/llama-3.1-8b-instruct:free' : '模型名稱'}
            />
          )}
          {providerType === 'ollama' && modelList.length === 0 && !loadingModels && (
            <p className="form-hint">無法取得模型列表，請確認 Ollama 正在運行，或手動輸入模型名稱</p>
          )}
          {providerType === 'ollama' && (
            <p className="form-hint">推薦：llama3.1:8b、qwen2.5:7b、mistral:7b 等通用模型（避免使用 code 專用模型）</p>
          )}
          {providerType === 'openrouter' && (
            <p className="form-hint">推薦：meta-llama/llama-3.1-8b-instruct:free、google/gemma-2-9b-it:free（免費模型）</p>
          )}
        </div>

        {(providerType === 'openai' || providerType === 'openrouter' || providerType === 'custom') && (
          <label className="form-field">
            <span>API Key{providerType === 'custom' ? '（可選）' : ''}</span>
            <input
              type="password"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
              placeholder="sk-..."
            />
            <p className="form-hint">留空表示不更新已儲存的 Key。</p>
          </label>
        )}

        <div className="rule-form-actions">
          <button type="submit" className="btn-save" disabled={saving}>
            {saving ? t.common.saving : t.common.save}
          </button>
          <button
            type="button"
            className="btn-test"
            disabled={testing}
            onClick={handleTestConnection}
          >
            {testing ? t.notifications.testing : t.notifications.testConnection}
          </button>
        </div>

        {testResult && (
          <div className={testResult.type === 'success' ? 'ai-settings-success' : 'rule-form-error'}>
            {testResult.msg}
          </div>
        )}
      </form>
    </div>
  );
}
