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

  useEffect(() => {
    loadApiPort();
  }, []);

  const loadApiPort = async () => {
    try {
      const port = await invoke<number>('get_api_port');
      setApiPort(port);
      setTempPort(port.toString());
    } catch (err) {
      console.error('載入 API port 失敗:', err);
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

  return (
    <div className="ps-section">
      <h3 className="ps-title">{t.api.title}</h3>
      
      <div style={{ padding: '20px', background: 'var(--surface0)', borderRadius: '12px', border: '1px solid var(--surface1)' }}>
        {/* 說明 */}
        <p style={{ margin: '0 0 16px 0', color: 'var(--text)', lineHeight: '1.6' }}>
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
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 10px 0', fontSize: '15px', color: 'var(--text)' }}>🐍 {t.api.pythonExample}</h4>
          <div style={{ position: 'relative' }}>
            <pre style={{ 
              margin: 0, 
              padding: '14px', 
              background: 'var(--mantle)', 
              borderRadius: '6px', 
              overflow: 'auto',
              fontSize: '12px',
              lineHeight: '1.5',
              color: 'var(--text)'
            }}>
              {pythonExample}
            </pre>
            <button 
              onClick={() => copyCode(pythonExample, 'python')}
              style={{
                position: 'absolute',
                top: '8px',
                right: '8px',
                padding: '4px 10px',
                background: 'var(--surface0)',
                border: '1px solid var(--surface1)',
                borderRadius: '4px',
                color: 'var(--text)',
                cursor: 'pointer',
                fontSize: '11px'
              }}
            >
              {copied === 'python' ? t.api.copied : t.api.copy}
            </button>
          </div>
        </div>

        {/* 歷史數據範例 */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 10px 0', fontSize: '15px', color: 'var(--text)' }}>📈 {t.api.historyExample}</h4>
          <div style={{ position: 'relative' }}>
            <pre style={{ 
              margin: 0, 
              padding: '14px', 
              background: 'var(--mantle)', 
              borderRadius: '6px', 
              overflow: 'auto',
              fontSize: '12px',
              lineHeight: '1.5',
              color: 'var(--text)'
            }}>
              {historyExample}
            </pre>
            <button 
              onClick={() => copyCode(historyExample, 'history')}
              style={{
                position: 'absolute',
                top: '8px',
                right: '8px',
                padding: '4px 10px',
                background: 'var(--surface0)',
                border: '1px solid var(--surface1)',
                borderRadius: '4px',
                color: 'var(--text)',
                cursor: 'pointer',
                fontSize: '11px'
              }}
            >
              {copied === 'history' ? t.api.copied : t.api.copy}
            </button>
          </div>
        </div>

        {/* curl 範例 */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 10px 0', fontSize: '15px', color: 'var(--text)' }}>💻 {t.api.curlExample}</h4>
          <div style={{ position: 'relative' }}>
            <pre style={{ 
              margin: 0, 
              padding: '14px', 
              background: 'var(--mantle)', 
              borderRadius: '6px', 
              overflow: 'auto',
              fontSize: '12px',
              lineHeight: '1.5',
              color: 'var(--text)'
            }}>
              {curlExample}
            </pre>
            <button 
              onClick={() => copyCode(curlExample, 'curl')}
              style={{
                position: 'absolute',
                top: '8px',
                right: '8px',
                padding: '4px 10px',
                background: 'var(--surface0)',
                border: '1px solid var(--surface1)',
                borderRadius: '4px',
                color: 'var(--text)',
                cursor: 'pointer',
                fontSize: '11px'
              }}
            >
              {copied === 'curl' ? t.api.copied : t.api.copy}
            </button>
          </div>
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
