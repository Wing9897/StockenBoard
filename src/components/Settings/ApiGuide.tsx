/**
 * API 使用說明組件
 */
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function ApiGuide() {
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
      alert('Port 必須在 1024-65535 之間');
      return;
    }
    
    try {
      await invoke('set_api_port', { port });
      setApiPort(port);
      setEditingPort(false);
      alert('Port 已更新，請重啟應用程式以生效');
    } catch (err) {
      alert(`儲存失敗: ${err}`);
    }
  };

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopied(id);
    setTimeout(() => setCopied(''), 2000);
  };

  const apiBase = `http://localhost:${apiPort}/api`;

  const pythonExample = `import requests

# 獲取所有價格
response = requests.get("${apiBase}/prices")
prices = response.json()['prices']

for item in prices:
    symbol = item['symbol']
    price = item['price']
    change = item['change_24h']
    print(f"{symbol}: ${'{'}price{'}'} ({change:+.2f}%)")`;

  const historyExample = `import requests
from datetime import datetime, timedelta

# 獲取最近 24 小時的歷史數據
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
print(f"獲取 {len(history)} 筆歷史數據")`;

  const curlExample = `# 獲取系統狀態
curl ${apiBase}/status

# 獲取所有訂閱
curl ${apiBase}/subscriptions

# 獲取所有價格
curl ${apiBase}/prices

# 獲取特定價格
curl ${apiBase}/prices/binance/BTCUSDT`;

  return (
    <div className="ps-section">
      <h3 className="ps-title">API 使用說明</h3>
      
      <div style={{ marginBottom: '24px', padding: '16px', background: 'var(--surface0)', borderRadius: '8px', border: '1px solid var(--surface1)' }}>
        <p style={{ margin: '0 0 12px 0', color: 'var(--text)' }}>
          StockenBoard 提供 HTTP API 讓外部程式（如 AI、Python 腳本）訪問實時和歷史數據。
        </p>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ color: 'var(--subtext0)' }}>API 地址:</span>
          <code style={{ padding: '4px 8px', background: 'var(--mantle)', borderRadius: '4px', color: 'var(--blue)' }}>
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
              修改 Port
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
                  background: 'var(--mantle)',
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
                儲存
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
                取消
              </button>
            </div>
          )}
        </div>
      </div>

      <div style={{ marginBottom: '24px' }}>
        <h4 style={{ margin: '0 0 12px 0', fontSize: '16px', color: 'var(--text)' }}>📡 API 端點</h4>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '14px' }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--surface1)' }}>
              <th style={{ padding: '8px', textAlign: 'left', color: 'var(--subtext1)' }}>端點</th>
              <th style={{ padding: '8px', textAlign: 'left', color: 'var(--subtext1)' }}>說明</th>
            </tr>
          </thead>
          <tbody>
            <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
              <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)' }}>GET /api/status</td>
              <td style={{ padding: '8px', color: 'var(--text)' }}>系統狀態</td>
            </tr>
            <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
              <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)' }}>GET /api/subscriptions</td>
              <td style={{ padding: '8px', color: 'var(--text)' }}>所有訂閱列表</td>
            </tr>
            <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
              <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)' }}>GET /api/prices</td>
              <td style={{ padding: '8px', color: 'var(--text)' }}>所有最新價格</td>
            </tr>
            <tr style={{ borderBottom: '1px solid var(--surface0)' }}>
              <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)' }}>GET /api/prices/:provider/:symbol</td>
              <td style={{ padding: '8px', color: 'var(--text)' }}>特定價格</td>
            </tr>
            <tr>
              <td style={{ padding: '8px', fontFamily: 'monospace', color: 'var(--blue)' }}>GET /api/history</td>
              <td style={{ padding: '8px', color: 'var(--text)' }}>歷史數據查詢</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div style={{ marginBottom: '24px' }}>
        <h4 style={{ margin: '0 0 12px 0', fontSize: '16px', color: 'var(--text)' }}>🐍 Python 範例</h4>
        <div style={{ position: 'relative' }}>
          <pre style={{ 
            margin: 0, 
            padding: '16px', 
            background: 'var(--mantle)', 
            borderRadius: '8px', 
            overflow: 'auto',
            fontSize: '13px',
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
              padding: '4px 12px',
              background: 'var(--surface0)',
              border: '1px solid var(--surface1)',
              borderRadius: '4px',
              color: 'var(--text)',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            {copied === 'python' ? '✓ 已複製' : '複製'}
          </button>
        </div>
      </div>

      <div style={{ marginBottom: '24px' }}>
        <h4 style={{ margin: '0 0 12px 0', fontSize: '16px', color: 'var(--text)' }}>📈 歷史數據範例</h4>
        <div style={{ position: 'relative' }}>
          <pre style={{ 
            margin: 0, 
            padding: '16px', 
            background: 'var(--mantle)', 
            borderRadius: '8px', 
            overflow: 'auto',
            fontSize: '13px',
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
              padding: '4px 12px',
              background: 'var(--surface0)',
              border: '1px solid var(--surface1)',
              borderRadius: '4px',
              color: 'var(--text)',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            {copied === 'history' ? '✓ 已複製' : '複製'}
          </button>
        </div>
      </div>

      <div style={{ marginBottom: '24px' }}>
        <h4 style={{ margin: '0 0 12px 0', fontSize: '16px', color: 'var(--text)' }}>💻 curl 範例</h4>
        <div style={{ position: 'relative' }}>
          <pre style={{ 
            margin: 0, 
            padding: '16px', 
            background: 'var(--mantle)', 
            borderRadius: '8px', 
            overflow: 'auto',
            fontSize: '13px',
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
              padding: '4px 12px',
              background: 'var(--surface0)',
              border: '1px solid var(--surface1)',
              borderRadius: '4px',
              color: 'var(--text)',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            {copied === 'curl' ? '✓ 已複製' : '複製'}
          </button>
        </div>
      </div>

      <div style={{ padding: '16px', background: 'var(--yellow-bg)', borderRadius: '8px', border: '1px solid var(--yellow)' }}>
        <h4 style={{ margin: '0 0 8px 0', fontSize: '14px', color: 'var(--yellow)' }}>⚠️ 注意事項</h4>
        <ul style={{ margin: 0, paddingLeft: '20px', color: 'var(--text)', fontSize: '14px' }}>
          <li>API 只監聽本地（127.0.0.1），只能從本機訪問</li>
          <li>需要先在 UI 中添加訂閱，API 才能訪問數據</li>
          <li>歷史數據需要啟用訂閱的「紀錄」功能</li>
          <li>建議輪詢間隔 ≥ 5 秒，避免過於頻繁</li>
        </ul>
      </div>
    </div>
  );
}
