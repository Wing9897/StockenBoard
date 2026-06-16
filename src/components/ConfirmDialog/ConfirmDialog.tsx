import { t } from '../../lib/i18n';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import './ConfirmDialog.css';

interface ConfirmDialogProps {
  title?: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
}

export function ConfirmDialog({ title, message, onConfirm, onCancel, confirmLabel, cancelLabel }: ConfirmDialogProps) {
  useEscapeKey(onCancel);

  return (
    <div className="modal-backdrop confirm-backdrop" onClick={onCancel}>
      <div className="modal-container confirm-modal" role="dialog" aria-modal="true" onClick={e => e.stopPropagation()}>
        {title && <h3 className="confirm-title">{title}</h3>}
        <p className="confirm-message">{message}</p>
        <div className="confirm-actions">
          <button className="view-editor-btn cancel" onClick={onCancel} aria-label={cancelLabel || t.common.cancel}>{t.common.cancel}</button>
          <button className="view-editor-btn confirm" onClick={onConfirm} aria-label={confirmLabel || t.common.confirm} autoFocus>{t.common.confirm}</button>
        </div>
      </div>
    </div>
  );
}
