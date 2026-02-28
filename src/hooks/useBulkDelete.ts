import { useCallback } from 'react';
import type { ToastActions } from '../types';
import { t } from '../lib/i18n';

interface UseBulkDeleteOptions {
  isCustomView: boolean;
  activeViewId: number;
  removeSubscriptions: (ids: number[]) => Promise<void>;
  removeSubscriptionFromView: (viewId: number, subId: number) => Promise<void>;
  requestConfirm: (message: string) => Promise<boolean>;
  toast: ToastActions;
  onDone: () => void;
}

/**
 * 共用 BulkDelete 確認邏輯 — App.tsx 和 DexPage.tsx 共用，
 * 消除兩處幾乎一模一樣的 onConfirm handler。
 */
export function useBulkDelete({
  isCustomView, activeViewId,
  removeSubscriptions, removeSubscriptionFromView,
  requestConfirm, toast, onDone,
}: UseBulkDeleteOptions) {
  return useCallback(async (ids: Set<number>) => {
    if (ids.size === 0) return;
    const confirmed = await requestConfirm(t.subs.bulkConfirm(ids.size));
    if (!confirmed) return;
    const count = ids.size;
    if (isCustomView) {
      for (const id of ids) {
        await removeSubscriptionFromView(activeViewId, id);
      }
      onDone();
      toast.info(t.subs.bulkRemoveView, t.subs.bulkRemovedView(count));
    } else {
      await removeSubscriptions([...ids]);
      onDone();
      toast.info(t.subs.bulkUnsubscribe, t.subs.bulkUnsubscribed(count));
    }
  }, [isCustomView, activeViewId, removeSubscriptions, removeSubscriptionFromView, requestConfirm, toast, onDone]);
}
