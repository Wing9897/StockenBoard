/**
 * App.tsx 的 modal 狀態管理 — 減少主元件的 useState 數量
 */
import { useState, useCallback } from 'react';

export function useAppModals() {
  const [showSubscriptionManager, setShowSubscriptionManager] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [showBatchActions, setShowBatchActions] = useState(false);
  const [showAddSubscription, setShowAddSubscription] = useState(false);

  return {
    showSubscriptionManager, setShowSubscriptionManager,
    showViewManager, setShowViewManager,
    showBulkDelete, setShowBulkDelete,
    showBatchActions, setShowBatchActions,
    showAddSubscription, setShowAddSubscription,
    closeAll: useCallback(() => {
      setShowSubscriptionManager(false);
      setShowViewManager(false);
      setShowBulkDelete(false);
      setShowBatchActions(false);
      setShowAddSubscription(false);
    }, []),
  };
}
