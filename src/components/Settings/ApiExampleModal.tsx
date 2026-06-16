/**
 * API code-example modal — extracted from ApiGuide.
 * Displays Python, History, or Curl code examples with copy-to-clipboard.
 */
import { useState } from 'react';
import { useLocale } from '../../hooks/useLocale';
import './ApiGuide.css';

interface ApiExampleModalProps {
  activeModal: 'python' | 'history' | 'curl' | null;
  onClose: () => void;
  apiBase: string;
}

export function ApiExampleModal({ activeModal, onClose, apiBase }: ApiExampleModalProps) {
  const { t } = useLocale();
  const [copied, setCopied] = useState('');

  if (!activeModal) return null;

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopied(id);
    setTimeout(() => setCopied(''), 2000);
  };

  const pythonExample = `import requests

# ${t.api.pricesEndpoint}
response = requests.get("${apiBase}/prices")
prices = response.json()['prices']

for item in prices:
    symbol = item['symbol']
    price = item['price']
    change = item['change_24h']
    print(f"{symbol}: \${'{'}price{'}'} ({change:+.2f}%)")`;

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
    <div className="modal-backdrop dm-picker-backdrop api-guide-modal-backdrop" onClick={onClose}>
      <div className="modal-container dm-picker-modal api-guide-modal-container" onClick={e => e.stopPropagation()}>
        <div className="dm-picker-header">
          <h4 className="dm-picker-title">{title}</h4>
          <button className="vsm-close" onClick={onClose}>✕</button>
        </div>
        <div className="api-guide-modal-body">
          <div>
            <div className="api-guide-modal-section-header">
              <div className="api-guide-modal-section-title">
                {activeModal === 'python' || activeModal === 'history' ? 'Python Script' : 'Bash / Curl'}
              </div>
              <button
                className="api-guide-modal-copy-btn"
                onClick={() => copyCode(code, 'modal-code')}
              >
                {copied === 'modal-code' ? t.api.copied : t.api.copy}
              </button>
            </div>
            <pre className="api-guide-modal-code">
              {code}
            </pre>
          </div>

          <div>
            <div className="api-guide-modal-section-header">
              <div className="api-guide-modal-section-title">{t.api.responseExample}</div>
              <button
                className="api-guide-modal-copy-btn"
                onClick={() => copyCode(response, 'modal-resp')}
              >
                {copied === 'modal-resp' ? t.api.copied : t.api.copy}
              </button>
            </div>
            <pre className="api-guide-modal-response">
              {response}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}
