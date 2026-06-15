import { useState, useEffect, useCallback } from 'react';
import { getTransport } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { silentLog } from '../../lib/errorLog';

interface AiProviderConfigResponse {
  base_url: string;
  model: string;
  has_api_key: boolean;
}

type ProviderType = 'ollama' | 'openai' | 'openrouter' | 'custom';

const PROVIDER_PRESETS: Record<ProviderType, { baseUrl: string; needsKey: boolean }> = {
  ollama: { baseUrl: 'http://localhost:11434/v1', needsKey: false },
  openai: { baseUrl: 'https://api.openai.com/v1', needsKey: true },
  openrouter: { baseUrl: 'https://openrouter.ai/api/v1', needsKey: true },
  custom: { baseUrl: '', needsKey: false },
};

/** 渲染時查表取得 provider 的 i18n 標籤（避免在模組常數中硬編碼中文） */
function providerLabel(type: ProviderType): string {
  switch (type) {
    case 'ollama': return t.notifications.providerOllama;
    case 'openai': return t.notifications.providerOpenai;
    case 'openrouter': return t.notifications.providerOpenrouter;
    case 'custom': return t.notifications.providerCustom;
  }
}

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
      const config = await getTransport().invoke<AiProviderConfigResponse | null>('get_ai_provider_config');
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
      silentLog('AiSettings.loadConfig', e);
    } finally {
      setLoading(false);
    }
  };

  const fetchModels = useCallback(async (url?: string) => {
    const targetUrl = url || baseUrl;
    if (!targetUrl.trim()) return;
    setLoadingModels(true);
    try {
      const models = await getTransport().invoke<string[]>('list_ai_models', {
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
      await getTransport().invoke('save_ai_provider_config', {
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
      const result = await getTransport().invoke<string>('test_ai_connection');
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
          <span>{t.notifications.providerType}</span>
          <div className="rule-mode-toggle">
            {(Object.keys(PROVIDER_PRESETS) as ProviderType[]).map((key) => (
              <button
                key={key}
                type="button"
                className={`mode-btn ${providerType === key ? 'active' : ''}`}
                onClick={() => handleProviderChange(key)}
              >
                {providerLabel(key)}
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
            <p className="form-hint">{t.notifications.ollamaBaseUrlHint}</p>
          )}
          {providerType === 'openai' && (
            <p className="form-hint">{t.notifications.openaiBaseUrlHint}</p>
          )}
          {providerType === 'openrouter' && (
            <p className="form-hint">{t.notifications.openrouterBaseUrlHint}</p>
          )}
        </label>

        <div className="form-field">
          <span>{t.notifications.modelLabel} {loadingModels && <small>（{t.common.loading}）</small>}</span>
          {modelList.length > 0 ? (
            <div className="ai-model-select-row">
              <select value={model} onChange={e => setModel(e.target.value)}>
                <option value="">{t.notifications.selectModel}</option>
                {modelList.map(m => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
              <button type="button" className="btn-refresh" onClick={() => fetchModels()} title={t.notifications.refresh}>🔄</button>
            </div>
          ) : (
            <input
              type="text"
              value={model}
              onChange={e => setModel(e.target.value)}
              placeholder={providerType === 'ollama' ? 'llama3.1:8b' : providerType === 'openai' ? 'gpt-4o' : providerType === 'openrouter' ? 'meta-llama/llama-3.1-8b-instruct:free' : t.notifications.model}
            />
          )}
          {providerType === 'ollama' && modelList.length === 0 && !loadingModels && (
            <p className="form-hint">{t.notifications.ollamaNoModelsHint}</p>
          )}
          {providerType === 'ollama' && (
            <p className="form-hint">{t.notifications.ollamaModelHint}</p>
          )}
          {providerType === 'openrouter' && (
            <p className="form-hint">{t.notifications.openrouterModelHint}</p>
          )}
        </div>

        {(providerType === 'openai' || providerType === 'openrouter' || providerType === 'custom') && (
          <label className="form-field">
            <span>API Key{providerType === 'custom' ? t.notifications.apiKeyOptionalSuffix : ''}</span>
            <input
              type="password"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
              placeholder="sk-..."
            />
            <p className="form-hint">{t.notifications.apiKeyBlankHint}</p>
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
