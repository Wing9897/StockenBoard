/**
 * Provider 編輯 Modal — 從 ProviderSettings.tsx 抽出
 */
import type { ProviderInfo } from '../../types';
import { TYPE_COLORS, getTypeLabels } from './providerConstants';
import { t } from '../../lib/i18n';

interface ProviderRow {
  id: string; name: string; provider_type: string;
  api_key?: string; api_secret?: string; api_url?: string;
  refresh_interval: number; connection_type: string;
  supports_websocket: number;
  record_from_hour: number | null; record_to_hour: number | null;
}

interface FormData {
  api_key: string; api_secret: string; api_url: string;
  refresh_interval: number; connection_type: string;
  record_from_hour: number | null; record_to_hour: number | null;
}

interface ProviderModalProps {
  provider: ProviderRow;
  info: ProviderInfo | undefined;
  formData: FormData;
  useKeyMode: boolean;
  getDesc: (id: string) => string;
  showModeToggle: boolean;
  canUseFree: boolean;
  onFormChange: (data: FormData) => void;
  onModeSwitch: (toKey: boolean) => void;
  onSave: () => void;
  onClose: () => void;
}

const TZ_LABEL = (() => {
  const o = -new Date().getTimezoneOffset();
  const h = Math.floor(Math.abs(o) / 60);
  const m = Math.abs(o) % 60;
  return `UTC${o >= 0 ? '+' : '-'}${h}${m ? ':' + String(m).padStart(2, '0') : ''}`;
})();

export function ProviderModal({
  provider, info, formData, useKeyMode, getDesc,
  showModeToggle, canUseFree,
  onFormChange, onModeSwitch, onSave, onClose,
}: ProviderModalProps) {
  const TYPE_LABELS = getTypeLabels();
  const set = (patch: Partial<FormData>) => onFormChange({ ...formData, ...patch });

  return (
    <div className="modal-backdrop ps-modal-backdrop" onClick={onClose}>
      <div className="modal-container ps-modal" role="dialog" aria-modal="true" aria-label={provider.name} onClick={e => e.stopPropagation()}>
        <div className="ps-modal-head">
          <div>
            <h4 className="ps-modal-title">{provider.name}</h4>
            <span className="ps-modal-type" style={{ color: TYPE_COLORS[provider.provider_type] }}>{TYPE_LABELS[provider.provider_type]}</span>
          </div>
          <button className="ps-modal-close" onClick={onClose} aria-label={t.common.close}>&#x2715;</button>
        </div>
        {info && (
          <div className="ps-modal-meta">
            <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.plan}</span><span className="ps-meta-value">{getDesc(provider.id)}</span></div>
            <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.connection}</span><span className="ps-meta-value">{provider.connection_type === 'websocket' ? t.providers.websocket : t.providers.restApi}</span></div>
            <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.format}</span><span className="ps-meta-value mono">{info.symbol_format}</span></div>
          </div>
        )}
        <div className="ps-modal-body">
          {showModeToggle && (
            <div className="form-group">
              <label>{t.providers.useMode}</label>
              <div className="mode-toggle">
                {canUseFree && (
                  <button type="button" className={`mode-btn ${!useKeyMode ? 'active' : ''}`} onClick={() => onModeSwitch(false)}>
                    {t.providers.freeMode} {info && <span className="mode-interval">{info.free_interval / 1000}{t.providers.seconds}</span>}
                  </button>
                )}
                <button type="button" className={`mode-btn ${useKeyMode ? 'active' : ''}`} onClick={() => onModeSwitch(true)}>
                  {t.providers.apiKeyMode} {info && <span className="mode-interval">{info.key_interval / 1000}{t.providers.seconds}</span>}
                </button>
              </div>
            </div>
          )}
          {useKeyMode && (info?.requires_api_key || info?.optional_api_key) && (
            <div className="form-group">
              <label>{t.apiKey.label} {info?.optional_api_key && !info?.requires_api_key && <span className="optional-badge">{t.providers.boostRate}</span>}</label>
              <input type="password" value={formData.api_key} onChange={e => set({ api_key: e.target.value })} placeholder={t.apiKey.placeholder} />
            </div>
          )}
          {useKeyMode && info?.requires_api_secret && (
            <div className="form-group">
              <label>{t.apiKey.secretLabel}</label>
              <input type="password" value={formData.api_secret} onChange={e => set({ api_secret: e.target.value })} placeholder={t.apiKey.secretPlaceholder} />
            </div>
          )}
          {info?.provider_type === 'dex' && (
            <div className="form-group">
              <label>{t.providers.apiUrl} <span className="optional-badge">{t.providers.apiUrlOptional}</span></label>
              <input value={formData.api_url} onChange={e => set({ api_url: e.target.value })} placeholder={t.providers.apiUrlPlaceholder} />
            </div>
          )}
          <div className="form-group">
            <label>{t.providers.refreshInterval} {info && <span className="optional-badge">{t.providers.refreshHint((useKeyMode ? info.key_interval : info.free_interval) / 1000)}</span>}</label>
            <input type="number" value={formData.refresh_interval} onChange={e => set({ refresh_interval: parseInt(e.target.value) || 5000 })} min={5000} step={1000} />
          </div>
          {provider.supports_websocket === 1 && (
            <div className="form-group">
              <label>{t.providers.connectionMethod}</label>
              <select value={formData.connection_type} onChange={e => set({ connection_type: e.target.value })}>
                <option value="rest">{t.providers.restApi}</option>
                <option value="websocket">{t.providers.websocket}</option>
              </select>
            </div>
          )}
          <div className="form-group">
            <label>{t.history.recordHours}</label>
            <div className="record-hours-row">
              <select
                value={formData.record_from_hour != null && formData.record_to_hour != null ? 'custom' : 'all'}
                onChange={e => {
                  if (e.target.value === 'all') set({ record_from_hour: null, record_to_hour: null });
                  else set({ record_from_hour: 16, record_to_hour: 9 });
                }}
              >
                <option value="all">{t.history.recordHoursAll}</option>
                <option value="custom">{t.history.recordHoursCustom}</option>
              </select>
              {formData.record_from_hour != null && formData.record_to_hour != null && (
                <div className="record-hours-pickers">
                  <span>{t.history.recordHoursFrom}</span>
                  <select value={formData.record_from_hour} onChange={e => set({ record_from_hour: Number(e.target.value) })}>
                    {Array.from({ length: 24 }, (_, i) => <option key={i} value={i}>{String(i).padStart(2, '0')}:00</option>)}
                  </select>
                  <span>{t.history.recordHoursTo}</span>
                  <select value={formData.record_to_hour} onChange={e => set({ record_to_hour: Number(e.target.value) })}>
                    {Array.from({ length: 25 }, (_, i) => <option key={i} value={i}>{i === 24 ? '24:00' : `${String(i).padStart(2, '0')}:00`}</option>)}
                  </select>
                </div>
              )}
            </div>
            <span className="form-hint priority">{t.history.recordHoursProviderHint} · {t.history.recordHoursPriority} · {t.history.recordHoursHint} ({TZ_LABEL})</span>
          </div>
        </div>
        <div className="ps-modal-foot">
          <button className="btn-cancel" onClick={onClose}>{t.common.cancel}</button>
          <button className="btn-save" onClick={onSave}>{t.common.save}</button>
        </div>
      </div>
    </div>
  );
}
