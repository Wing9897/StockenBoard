import { t, LOCALES, setLocale, type LocaleId } from '../../lib/i18n';
import { useLocale } from '../../hooks/useLocale';
import './Settings.css';

export function LanguagePicker() {
  const { locale } = useLocale();

  const handleSelect = (id: LocaleId) => {
    setLocale(id);
  };

  return (
    <div className="settings-section lang-section">
      <h3>{t.settings.language}</h3>
      <div className="lang-grid">
        {LOCALES.map(loc => (
          <div
            key={loc.id}
            className={`lang-card ${locale === loc.id ? 'active' : ''}`}
            onClick={() => handleSelect(loc.id)}
            role="button"
            aria-pressed={locale === loc.id}
            tabIndex={0}
            onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') handleSelect(loc.id); }}
          >
            <span className="lang-flag">{loc.flag}</span>
            <span className="lang-label">{loc.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
