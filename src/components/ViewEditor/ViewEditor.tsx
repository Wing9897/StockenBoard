import { useState, useEffect, useRef } from 'react';
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
    if (!trimmed) {
      return '名稱不可為空白';
    }
    const isDuplicate = existingNames.some(
      (n) => n.trim().toLowerCase() === trimmed.toLowerCase()
    );
    if (isDuplicate) {
      return '此名稱已存在';
    }
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
    if (e.key === 'Enter') {
      handleConfirm();
    } else if (e.key === 'Escape') {
      onCancel();
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
    if (error) setError('');
  };

  const title = mode === 'create' ? '建立新頁面' : '重新命名頁面';

  return (
    <div className="view-editor-backdrop" onClick={onCancel}>
      <div
        className="view-editor-modal"
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
          placeholder="輸入頁面名稱"
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
            取消
          </button>
          <button className="view-editor-btn confirm" onClick={handleConfirm}>
            確認
          </button>
        </div>
      </div>
    </div>
  );
}
