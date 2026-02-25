import { useState, useEffect, useRef } from 'react';
import { t } from '../../lib/i18n';
import './ViewEditor.css';

interface ViewEditorProps {
  mode: 'create' | 'rename';
  currentName?: string;
  existingNames: string[];
  onConfirm: (name: string) => void;
  onCancel: () => void;
}

export function ViewEditor({
  mode,
  currentName = '',
  existingNames,
  onConfirm,
  onCancel,
}: ViewEditorProps) {
  const [name, setName] = useState(mode === 'rename' ? currentName : '');
  const [error, setError] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    if (mode === 'rename') {
      inputRef.current?.select();
    }
  }, [mode]);

  const validate = (value: string): string => {
    const trimmed = value.trim();
    if (!trimmed) return t.views.viewNameEmpty;
    const isDuplicate = existingNames.some(
      (n) => n.trim().toLowerCase() === trimmed.toLowerCase()
    );
    if (isDuplicate) return t.views.viewNameDuplicate;
    return '';
  };

  const handleConfirm = () => {
    const validationError = validate(name);
    if (validationError) {
      setError(validationError);
      return;
    }
    onConfirm(name.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleConfirm();
    else if (e.key === 'Escape') onCancel();
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
    if (error) setError('');
  };

  const title = mode === 'create' ? t.views.createView : t.views.renameView;

  return (
    <div className="modal-backdrop view-editor-backdrop" onClick={onCancel}>
      <div
        className="modal-container view-editor-modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="view-editor-title">{title}</h2>
        <input
          ref={inputRef}
          className={`view-editor-input ${error ? 'has-error' : ''}`}
          type="text"
          value={name}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder={t.views.viewNamePlaceholder}
          aria-invalid={!!error}
          aria-describedby={error ? 'view-editor-error' : undefined}
        />
        {error && (
          <p id="view-editor-error" className="view-editor-error" role="alert">
            {error}
          </p>
        )}
        <div className="view-editor-actions">
          <button className="view-editor-btn cancel" onClick={onCancel}>
            {t.common.cancel}
          </button>
          <button className="view-editor-btn confirm" onClick={handleConfirm}>
            {t.common.confirm}
          </button>
        </div>
      </div>
    </div>
  );
}
