/**
 * 共用編輯面板外殼 — AssetEditPanel 和 DexEditPanel 共用
 * 提供 modal backdrop、容器、底部按鈕列
 */
import { useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { t } from '../../lib/i18n';

interface EditPanelShellProps {
  className?: string;
  error?: string | null;
  saving?: boolean;
  isCustomView?: boolean;
  onSave: () => void;
  onDelete: () => void;
  onClose: () => void;
  children: ReactNode;
}

export function EditPanelShell({ className, error, saving, isCustomView, onSave, onDelete, onClose, children }: EditPanelShellProps) {
  const ref = useRef<HTMLDivElement>(null);
  useEscapeKey(onClose);

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className={`modal-container ${className || 'asset-edit-panel'}`} ref={ref} role="dialog" aria-modal="true" onClick={e => e.stopPropagation()}>
        {children}
        {error && <div className="edit-error">{error}</div>}
        <div className="edit-actions">
          <button className="edit-btn delete" onClick={onDelete}>{isCustomView ? t.subs.removeDisplay : t.common.delete}</button>
          <div className="edit-actions-right">
            <button className="edit-btn cancel" onClick={onClose} disabled={saving}>{t.common.cancel}</button>
            <button className="edit-btn save" onClick={onSave} disabled={saving}>{saving ? t.common.saving : t.common.save}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body
  );
}
