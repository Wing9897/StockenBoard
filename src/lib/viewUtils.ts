import type { ViewMode } from '../types';

/** 根據 viewMode 回傳 grid container 的 CSS class */
export function getGridClass(viewMode: ViewMode): string {
  if (viewMode === 'compact') return 'asset-grid compact';
  if (viewMode === 'list') return 'asset-list';
  return 'asset-grid';
}
