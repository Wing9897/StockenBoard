import { useEffect } from 'react';
import { t } from '../../lib/i18n';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import './ConfirmDialog.css';

interface ConfirmDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ message, onConfirm, onCancel }: ConfirmDialogProps) {
  useEscapeKey(onCancel);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Enter') onConfirm(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onConfirm]);

  return (
    <div className="modal-backdrop confirm-backdrop" onClick={onCancel}>
      <div className="modal-container confirm-modal" role="dialog" aria-modal="true" onClick={e => e.stopPropagation()}>
        <p className="confirm-message">{message}</p>
        <div className="confirm-actions">
          <button className="view-editor-btn cancel" onClick={onCancel}>{t.common.cancel}</button>
          <button className="view-editor-btn confirm" onClick={onConfirm}>{t.common.confirm}</button>
        </div>
      </div>
    </div>
  );
}
