import { useState, useCallback } from 'react';
import { t } from '../../lib/i18n';
import './Settings.css';

const DEFAULTS = { fontSize: 14, cardGap: 10, cardColumns: 0 };
const LS_KEY = 'sb_ui_custom';

function load() {
  try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(LS_KEY) || '{}') }; }
  catch { return { ...DEFAULTS }; }
}

function apply(v: typeof DEFAULTS) {
  const r = document.documentElement;
  r.style.setProperty('--ui-font-size', `${v.fontSize}px`);
  r.style.setProperty('--ui-card-gap', `${v.cardGap}px`);
  // 卡片欄數：0=auto-fill, >0=固定欄數
  if (v.cardColumns > 0) {
    r.style.setProperty('--ui-card-cols', `repeat(${v.cardColumns}, 1fr)`);
  } else {
    r.style.removeProperty('--ui-card-cols');
  }
}

// 啟動時套用
apply(load());

export function UICustomizer() {
  const [vals, setVals] = useState(load);

  const update = useCallback((key: keyof typeof DEFAULTS, raw: number) => {
    setVals((prev: typeof DEFAULTS) => {
      const next = { ...prev, [key]: raw };
      localStorage.setItem(LS_KEY, JSON.stringify(next));
      apply(next);
      return next;
    });
  }, []);

  const reset = useCallback(() => {
    localStorage.removeItem(LS_KEY);
    setVals({ ...DEFAULTS });
    apply(DEFAULTS);
  }, []);

  return (
    <div className="settings-section">
      <h3>{t.settings.uiCustom}</h3>
      <div className="ui-custom-grid">
        <div className="ui-custom-item">
          <label>{t.settings.fontSize}</label>
          <div className="ui-custom-slider">
            <input type="range" min={10} max={20} step={1} value={vals.fontSize} onChange={e => update('fontSize', +e.target.value)} />
            <span>{vals.fontSize}px</span>
          </div>
        </div>
        <div className="ui-custom-item">
          <label>{t.settings.cardGap}</label>
          <div className="ui-custom-slider">
            <input type="range" min={2} max={24} step={2} value={vals.cardGap} onChange={e => update('cardGap', +e.target.value)} />
            <span>{vals.cardGap}px</span>
          </div>
        </div>
        <div className="ui-custom-item">
          <label>{t.settings.cardColumns}</label>
          <div className="ui-custom-slider">
            <input type="range" min={0} max={8} step={1} value={vals.cardColumns} onChange={e => update('cardColumns', +e.target.value)} />
            <span>{vals.cardColumns === 0 ? 'Auto' : vals.cardColumns}</span>
          </div>
        </div>
      </div>
      <button className="ui-custom-reset" onClick={reset}>{t.settings.resetDefaults}</button>
    </div>
  );
}
