import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { t } from '../../lib/i18n';
import './BatchActions.css';

interface BatchActionsProps {
  /** 'spot' 顯示展開詳情 + 盤前盤後；'dex' 只顯示通用選項 */
  mode: 'spot' | 'dex';
  expandAll?: boolean;
  showPrePost?: boolean;
  onToggleExpandAll?: () => void;
  onTogglePrePost?: () => void;
  onClose: () => void;
}

export function BatchActions({ mode, expandAll, showPrePost, onToggleExpandAll, onTogglePrePost, onClose }: BatchActionsProps) {
  useEscapeKey(onClose);
  const [unattended, setUnattended] = useState(() => localStorage.getItem('sb_unattended') === '1');

  // 啟動時將 localStorage 的狀態同步到後端
  useEffect(() => {
    const saved = localStorage.getItem('sb_unattended') === '1';
    if (saved) invoke('set_unattended_polling', { enabled: true }).catch(() => {});
  }, []);

  const toggleUnattended = async () => {
    const next = !unattended;
    setUnattended(next);
    localStorage.setItem('sb_unattended', next ? '1' : '0');
    try {
      await invoke('set_unattended_polling', { enabled: next });
    } catch {
      setUnattended(!next); // rollback
      localStorage.setItem('sb_unattended', !next ? '1' : '0');
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-container ba-modal" role="dialog" aria-modal="true" aria-label={t.dashboard.batchActions} onClick={e => e.stopPropagation()}>
        <div className="ba-header">
          <h4 className="ba-title">{t.dashboard.batchActions}</h4>
          <button className="vsm-close" onClick={onClose} aria-label={t.common.close}>✕</button>
        </div>
        <div className="ba-body">
          {mode === 'spot' && onToggleExpandAll && (
            <label className="ba-row">
              <span className="ba-label">{t.dashboard.expandAll}</span>
              <button
                role="switch"
                aria-checked={expandAll}
                className={`ba-switch ${expandAll ? 'on' : ''}`}
                onClick={onToggleExpandAll}
              >
                <span className="ba-switch-thumb" />
              </button>
            </label>
          )}
          {mode === 'spot' && onTogglePrePost && (
            <label className="ba-row">
              <span className="ba-label">{t.dashboard.showPrePost}</span>
              <button
                role="switch"
                aria-checked={showPrePost}
                className={`ba-switch ${showPrePost ? 'on' : ''}`}
                onClick={onTogglePrePost}
              >
                <span className="ba-switch-thumb" />
              </button>
            </label>
          )}
          <label className="ba-row">
            <span className="ba-label">{t.dashboard.unattendedPolling}</span>
            <button
              role="switch"
              aria-checked={unattended}
              className={`ba-switch ${unattended ? 'on' : ''}`}
              onClick={toggleUnattended}
            >
              <span className="ba-switch-thumb" />
            </button>
          </label>
          {/* 預留空間：未來可在此加入更多操作 */}
        </div>
      </div>
    </div>
  );
}
