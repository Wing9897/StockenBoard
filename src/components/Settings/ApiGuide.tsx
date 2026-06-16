/**
 * API 使用說明組件
 */
import { useState, useEffect } from 'react';
import { getTransport } from '../../lib/transport';
import { useLocale } from '../../hooks/useLocale';
import { silentLog } from '../../lib/errorLog';
import { ApiExampleModal } from './ApiExampleModal';
import './ApiGuide.css';

interface ApiGuideProps {
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

export function ApiGuide({ onToast }: ApiGuideProps) {
  const { t } = useLocale();
  const [apiPort, setApiPort] = useState(8080);
  const [editingPort, setEditingPort] = useState(false);
  const [tempPort, setTempPort] = useState('8080');
  const [apiEnabled, setApiEnabled] = useState(false);
  const [activeModal, setActiveModal] = useState<'python' | 'history' | 'curl' | null>(null);

  useEffect(() => {
    loadApiSettings();
  }, []);

  const loadApiSettings = async () => {
    try {
      const port = await getTransport().invoke<number>('get_api_port');
      const enabled = await getTransport().invoke<boolean>('get_api_enabled');
      setApiPort(port);
      setTempPort(port.toString());
      setApiEnabled(enabled);
    } catch (err) {
      silentLog('ApiGuide.loadApiSettings', err);
    }
  };

  const toggleApiEnabled = async () => {
    try {
      const newEnabled = !apiEnabled;
      await getTransport().invoke('set_api_enabled', { enabled: newEnabled });
      setApiEnabled(newEnabled);
      onToast?.('info', newEnabled ? t.api.enabledMsg : t.api.disabledMsg);
    } catch (err) {
      onToast?.('error', `${t.api.saveFailed}: ${err}`);
    }
  };

  const saveApiPort = async () => {
    const port = parseInt(tempPort);
    if (isNaN(port) || port < 1024 || port > 65535) {
      onToast?.('error', t.api.portRange);
      return;
    }

    try {
      await getTransport().invoke('set_api_port', { port });
      setApiPort(port);
      setEditingPort(false);
      onToast?.('info', t.api.portSaved);
    } catch (err) {
      onToast?.('error', `${t.api.saveFailed}: ${err}`);
    }
  };

  const apiBase = `http://localhost:${apiPort}/api`;

  return (
    <div className="ps-section">
      <ApiExampleModal activeModal={activeModal} onClose={() => setActiveModal(null)} apiBase={apiBase} />
      <h3 className="ps-title">{t.api.title}</h3>

      <div className="api-guide-container">
        {/* 啟用開關 */}
        <div className="api-guide-toggle-row">
          <div>
            <div className="api-guide-toggle-label">
              {t.api.enableApi}
            </div>
            <div className="api-guide-toggle-desc">
              {t.api.enableDesc}
            </div>
          </div>
          <label className="api-guide-switch">
            <input
              type="checkbox"
              checked={apiEnabled}
              onChange={toggleApiEnabled}
            />
            <span className={`api-guide-switch-track ${apiEnabled ? 'active' : ''}`}>
              <span className="api-guide-switch-thumb" />
            </span>
          </label>
        </div>

        {/* 說明 */}
        <p className={`api-guide-desc ${apiEnabled ? '' : 'disabled'}`}>
          {t.api.description}
        </p>

        {/* API 地址 */}
        <div className="api-guide-address-bar">
          <div className="api-guide-address-bar-inner">
            <span className="api-guide-address-label">{t.api.address}:</span>
            <code className="api-guide-address-code">
              {apiBase}
            </code>
            {!editingPort ? (
              <button
                onClick={() => setEditingPort(true)}
                className="api-guide-port-btn"
              >
                {t.api.editPort}
              </button>
            ) : (
              <div className="api-guide-port-edit-row">
                <input
                  type="number"
                  value={tempPort}
                  onChange={(e) => setTempPort(e.target.value)}
                  min="1024"
                  max="65535"
                  className="api-guide-port-input"
                />
                <button
                  onClick={saveApiPort}
                  className="api-guide-port-save-btn"
                >
                  {t.common.save}
                </button>
                <button
                  onClick={() => {
                    setEditingPort(false);
                    setTempPort(apiPort.toString());
                  }}
                  className="api-guide-port-cancel-btn"
                >
                  {t.common.cancel}
                </button>
              </div>
            )}
          </div>
        </div>

        {/* API 端點 */}
        <div className="api-guide-endpoints-section">
          <h4 className="api-guide-endpoints-title">📡 {t.api.endpoints}</h4>
          <table className="api-guide-endpoints-table">
            <thead>
              <tr>
                <th>{t.api.endpointCol}</th>
                <th>{t.api.descCol}</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="api-guide-endpoint-cell mono">GET /api/status</td>
                <td className="api-guide-endpoint-cell">{t.api.statusEndpoint}</td>
              </tr>
              <tr>
                <td className="api-guide-endpoint-cell mono">GET /api/subscriptions</td>
                <td className="api-guide-endpoint-cell">{t.api.subsEndpoint}</td>
              </tr>
              <tr>
                <td className="api-guide-endpoint-cell mono">GET /api/prices</td>
                <td className="api-guide-endpoint-cell">{t.api.pricesEndpoint}</td>
              </tr>
              <tr>
                <td className="api-guide-endpoint-cell mono">GET /api/prices/:provider/:symbol</td>
                <td className="api-guide-endpoint-cell">{t.api.priceEndpoint}</td>
              </tr>
              <tr>
                <td className="api-guide-endpoint-cell mono">GET /api/history</td>
                <td className="api-guide-endpoint-cell">{t.api.historyEndpoint}</td>
              </tr>
            </tbody>
          </table>
        </div>

        {/* Python 範例 */}
        <div className="api-guide-example-row">
          <div className="api-guide-example-row-label">
            <span className="api-guide-example-row-icon">🐍</span>
            <span className="api-guide-example-row-text">{t.api.pythonExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('python')}
            className="api-guide-example-btn"
          >
            {t.api.viewExample}
          </button>
        </div>

        {/* 歷史數據範例 */}
        <div className="api-guide-example-row">
          <div className="api-guide-example-row-label">
            <span className="api-guide-example-row-icon">📈</span>
            <span className="api-guide-example-row-text">{t.api.historyExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('history')}
            className="api-guide-example-btn"
          >
            {t.api.viewExample}
          </button>
        </div>

        {/* curl 範例 */}
        <div className="api-guide-example-row">
          <div className="api-guide-example-row-label">
            <span className="api-guide-example-row-icon">💻</span>
            <span className="api-guide-example-row-text">{t.api.curlExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('curl')}
            className="api-guide-example-btn"
          >
            {t.api.viewExample}
          </button>
        </div>

        {/* 注意事項 */}
        <div className="api-guide-notes">
          <h4>⚠️ {t.api.notes}</h4>
          <ul>
            <li>{t.api.note1}</li>
            <li>{t.api.note2}</li>
            <li>{t.api.note3}</li>
            <li>{t.api.note4}</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
