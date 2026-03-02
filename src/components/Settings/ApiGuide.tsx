/**
 * API 使用說明組件
 */
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useLocale } from '../../hooks/useLocale';

export function ApiGuide() {
  const { t } = useLocale();
  const [copied, setCopied] = useState('');
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
      const port = await invoke<number>('get_api_port');
      const enabled = await invoke<boolean>('get_api_enabled');
      setApiPort(port);
      setTempPort(port.toString());
      setApiEnabled(enabled);
    } catch (err) {
      console.error('載入 API 設定失敗:', err);
    }
  };

  const toggleApiEnabled = async () => {
    try {
      const newEnabled = !apiEnabled;
      await invoke('set_api_enabled', { enabled: newEnabled });
      setApiEnabled(newEnabled);
      alert(newEnabled ? t.api.enabledMsg : t.api.disabledMsg);
    } catch (err) {
      alert(`${t.api.saveFailed}: ${err}`);
    }
  };

  const saveApiPort = async () => {
    const port = parseInt(tempPort);
    if (isNaN(port) || port < 1024 || port > 65535) {
      alert(t.api.portRange);
      return;
    }

    try {
      await invoke('set_api_port', { port });
      setApiPort(port);
      setEditingPort(false);
      alert(t.api.portSaved);
    } catch (err) {
      alert(`${t.api.saveFailed}: ${err}`);
    }
  };

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopied(id);
    setTimeout(() => setCopied(''), 2000);
  };

  const apiBase = `http://localhost:${apiPort}/api`;

  const pythonExample = `import requests

# ${t.api.pricesEndpoint}
response = requests.get("${apiBase}/prices")
prices = response.json()['prices']

for item in prices:
    symbol = item['symbol']
    price = item['price']
    change = item['change_24h']
    print(f"{symbol}: ${'{'}price{'}'} ({change:+.2f}%)")`;

  const historyExample = `import requests
from datetime import datetime, timedelta

# ${t.api.historyEndpoint}
now = int(datetime.now().timestamp())
yesterday = now - 86400

response = requests.get("${apiBase}/history", params={
    "symbol": "BTCUSDT",
    "provider": "binance",
    "from": yesterday,
    "to": now,
    "limit": 1000
})

history = response.json()['records']
print(f"Records: {len(history)}")`;

  const curlExample = `# ${t.api.statusEndpoint}
curl ${apiBase}/status

# ${t.api.subsEndpoint}
curl ${apiBase}/subscriptions

# ${t.api.pricesEndpoint}
curl ${apiBase}/prices

# ${t.api.priceEndpoint}
curl ${apiBase}/prices/binance/BTCUSDT`;

  const renderModal = () => {
    if (!activeModal) return null;

    let title = '';
    let code = '';
    let response = '';

    if (activeModal === 'python') {
      title = t.api.pythonExample;
      code = pythonExample;
      response = `{\n  "prices": [\n    {\n      "symbol": "BTCUSDT",\n      "price": 65000.0,\n      "change_24h": 2.5,\n      "volume": 1200.5,\n      "provider": "binance",\n      "last_updated": 1700000000\n    }\n  ]\n}`;
    } else if (activeModal === 'history') {
      title = t.api.historyExample;
      code = historyExample;
      response = `{\n  "records": [\n    {\n      "id": 1,\n      "symbol": "BTCUSDT",\n      "price": 64500.0,\n      "volume": 0.0,\n      "timestamp": 1700000000,\n      "provider_id": "binance"\n    }\n  ]\n}`;
    } else if (activeModal === 'curl') {
      title = t.api.curlExample;
      code = curlExample;
      response = `{\n  "status": "ok",\n  "version": "0.1.0",\n  "uptime": 3600,\n  "active_providers": 3\n}`;
    }

    return (
      <div className="modal-backdrop dm-picker-backdrop" onClick={() => setActiveModal(null)} style={{ zIndex: 1000 }}>
        <div className="modal-container dm-picker-modal" onClick={e => e.stopPropagation()} style={{ maxWidth: '700px', width: '90%' }}>
          <div className="dm-picker-header">
            <h4 className="dm-picker-title">{title}</h4>
            <button className="vsm-close" onClick={() => setActiveModal(null)}>✕</button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', padding: '16px', maxHeight: '70vh', overflowY: 'auto' }}>
            <div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <div style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--text)' }}>{activeModal === 'python' || activeModal === 'history' ? 'Python Script' : 'Bash / Curl'}</div>
                <button
                  onClick={() => copyCode(code, 'modal-code')}
                  style={{
                    padding: '4px 10px',
                    background: 'var(--surface0)',
                    border: '1px solid var(--surface1)',
                    borderRadius: '4px',
                    color: 'var(--text)',
                    cursor: 'pointer',
                    fontSize: '11px'
                  }}
                >
                  {copied === 'modal-code' ? t.api.copied : t.api.copy}
                </button>
              </div>
              <pre style={{ margin: 0, padding: '14px', background: 'var(--mantle)', borderRadius: '6px', overflow: 'auto', fontSize: '12px', lineHeight: '1.5', color: 'var(--text)' }}>
                {code}
              </pre>
            </div>

            <div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <div style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--text)' }}>{t.api.responseExample}</div>
                <button
                  onClick={() => copyCode(response, 'modal-resp')}
                  style={{
                    padding: '4px 10px',
                    background: 'var(--surface0)',
                    border: '1px solid var(--surface1)',
                    borderRadius: '4px',
                    color: 'var(--text)',
                    cursor: 'pointer',
                    fontSize: '11px'
                  }}
                >
                  {copied === 'modal-resp' ? t.api.copied : t.api.copy}
                </button>
              </div>
              <pre style={{ margin: 0, padding: '14px', background: 'var(--base)', border: '1px solid var(--surface1)', borderRadius: '6px', overflow: 'auto', fontSize: '12px', lineHeight: '1.5', color: 'var(--green)' }}>
                {response}
              </pre>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="ps-section">
      {renderModal()}
      <h3 className="ps-title">{t.api.title}</h3>

      <div style={{ padding: '20px', background: 'var(--surface0)', borderRadius: '12px', border: '1px solid var(--surface1)' }}>
        {/* 啟用開關 */}
        <div style={{ marginBottom: '20px', padding: '14px', background: 'var(--mantle)', borderRadius: '8px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <div style={{ fontSize: '15px', fontWeight: '500', color: 'var(--text)', marginBottom: '4px' }}>
              {t.api.enableApi}
            </div>
            <div style={{ fontSize: '13px', color: 'var(--subtext0)' }}>
              {t.api.enableDesc}
            </div>
          </div>
          <label style={{ position: 'relative', display: 'inline-block', width: '48px', height: '26px', cursor: 'pointer' }}>
            <input
              type="checkbox"
              checked={apiEnabled}
              onChange={toggleApiEnabled}
              style={{ opacity: 0, width: 0, height: 0 }}
            />
            <span style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              background: apiEnabled ? 'var(--green)' : 'var(--surface2)',
              borderRadius: '13px',
              transition: 'background 0.2s',
            }}>
              <span style={{
                position: 'absolute',
                content: '',
                height: '20px',
                width: '20px',
                left: apiEnabled ? '25px' : '3px',
                bottom: '3px',
                background: 'white',
                borderRadius: '50%',
                transition: 'left 0.2s',
              }} />
            </span>
          </label>
        </div>

        {/* 說明 */}
        <p style={{ margin: '0 0 16px 0', color: 'var(--text)', lineHeight: '1.6', opacity: apiEnabled ? 1 : 0.5 }}>
          {t.api.description}
        </p>

        {/* API 地址 */}
        <div style={{ marginBottom: '24px', padding: '12px', background: 'var(--mantle)', borderRadius: '8px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flexWrap: 'wrap' }}>
            <span style={{ color: 'var(--subtext0)', fontSize: '14px' }}>{t.api.address}:</span>
            <code style={{ padding: '4px 8px', background: 'var(--base)', borderRadius: '4px', color: 'var(--blue)', fontSize: '14px' }}>
              {apiBase}
            </code>
            {!editingPort ? (
              <button
                onClick={() => setEditingPort(true)}
                style={{
                  padding: '4px 12px',
                  background: 'var(--surface1)',
                  border: '1px solid var(--surface2)',
                  borderRadius: '4px',
                  color: 'var(--text)',
                  cursor: 'pointer',
                  fontSize: '12px'
                }}
              >
                {t.api.editPort}
              </button>
            ) : (
              <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                <input
                  type="number"
                  value={tempPort}
                  onChange={(e) => setTempPort(e.target.value)}
                  min="1024"
                  max="65535"
                  style={{
                    width: '80px',
                    padding: '4px 8px',
                    background: 'var(--base)',
                    border: '1px solid var(--surface2)',
                    borderRadius: '4px',
                    color: 'var(--text)',
                    fontSize: '13px'
                  }}
                />
                <button
                  onClick={saveApiPort}
                  style={{
                    padding: '4px 12px',
                    background: 'var(--green)',
                    border: 'none',
                    borderRadius: '4px',
                    color: 'var(--base)',
                    cursor: 'pointer',
                    fontSize: '12px'
                  }}
                >
                  {t.common.save}
                </button>
                <button
                  onClick={() => {
                    setEditingPort(false);
                    setTempPort(apiPort.toString());
                  }}
                  style={{
                    padding: '4px 12px',
                    background: 'var(--surface1)',
                    border: '1px solid var(--surface2)',
                    borderRadius: '4px',
                    color: 'var(--text)',
                    cursor: 'pointer',
                    fontSize: '12px'
                  }}
                >
                  {t.common.cancel}
                </button>
              </div>
            )}
          </div>
        </div>

        {/* API 端點 */}
        <div style={{ marginBottom: '24px' }}>
          <h4 style={{ margin: '0 0 12px 0', fontSize: '15px', color: 'var(--text)' }}>📡 {t.api.endpoints}</h4>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--surface1)' }}>
                <th style={{ padding: '8px', textAlign: 'left', color: 'var(--subtext1)' }}>{t.api.endpointCol}</th>
                <th style={{ padding: '8px', textAlign: 'left', color: 'var(--subtext1)' }}>{t.api.descCol}</th>
              </tr>
            </thead>
            <tbody>
              <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
                <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)', fontSize: '12px' }}>GET /api/status</td>
                <td style={{ padding: '8px', color: 'var(--text)' }}>{t.api.statusEndpoint}</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
                <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)', fontSize: '12px' }}>GET /api/subscriptions</td>
                <td style={{ padding: '8px', color: 'var(--text)' }}>{t.api.subsEndpoint}</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
                <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)', fontSize: '12px' }}>GET /api/prices</td>
                <td style={{ padding: '8px', color: 'var(--text)' }}>{t.api.pricesEndpoint}</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
                <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)', fontSize: '12px' }}>GET /api/prices/:provider/:symbol</td>
                <td style={{ padding: '8px', color: 'var(--text)' }}>{t.api.priceEndpoint}</td>
              </tr>
              <tr>
                <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)', fontSize: '12px' }}>GET /api/history</td>
                <td style={{ padding: '8px', color: 'var(--text)' }}>{t.api.historyEndpoint}</td>
              </tr>
            </tbody>
          </table>
        </div>

        {/* Python 範例 */}
        <div style={{ marginBottom: '16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 16px', background: 'var(--mantle)', borderRadius: '8px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '16px' }}>🐍</span>
            <span style={{ fontSize: '14px', color: 'var(--text)', fontWeight: '500' }}>{t.api.pythonExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('python')}
            style={{ padding: '6px 14px', background: 'var(--blue)', color: 'var(--base)', border: 'none', borderRadius: '6px', cursor: 'pointer', fontSize: '13px', fontWeight: '500' }}
          >
            {t.api.viewExample || '查看指令與回傳範例'}
          </button>
        </div>

        {/* 歷史數據範例 */}
        <div style={{ marginBottom: '16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 16px', background: 'var(--mantle)', borderRadius: '8px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '16px' }}>📈</span>
            <span style={{ fontSize: '14px', color: 'var(--text)', fontWeight: '500' }}>{t.api.historyExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('history')}
            style={{ padding: '6px 14px', background: 'var(--blue)', color: 'var(--base)', border: 'none', borderRadius: '6px', cursor: 'pointer', fontSize: '13px', fontWeight: '500' }}
          >
            {t.api.viewExample || '查看指令與回傳範例'}
          </button>
        </div>

        {/* curl 範例 */}
        <div style={{ marginBottom: '24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 16px', background: 'var(--mantle)', borderRadius: '8px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '16px' }}>💻</span>
            <span style={{ fontSize: '14px', color: 'var(--text)', fontWeight: '500' }}>{t.api.curlExample}</span>
          </div>
          <button
            onClick={() => setActiveModal('curl')}
            style={{ padding: '6px 14px', background: 'var(--blue)', color: 'var(--base)', border: 'none', borderRadius: '6px', cursor: 'pointer', fontSize: '13px', fontWeight: '500' }}
          >
            {t.api.viewExample || '查看指令與回傳範例'}
          </button>
        </div>

        {/* 注意事項 */}
        <div style={{ padding: '14px', background: 'var(--yellow-bg)', borderRadius: '8px', border: '1px solid var(--yellow)' }}>
          <h4 style={{ margin: '0 0 8px 0', fontSize: '14px', color: 'var(--yellow)' }}>⚠️ {t.api.notes}</h4>
          <ul style={{ margin: 0, paddingLeft: '20px', color: 'var(--text)', fontSize: '13px', lineHeight: '1.6' }}>
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
