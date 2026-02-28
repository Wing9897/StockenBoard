/**
 * 統一錯誤記錄工具 — 取代靜默 catch {}。
 * 開發模式下 console.warn，生產模式靜默。
 */
export function silentLog(context: string, err: unknown) {
  if (import.meta.env.DEV) {
    console.warn(`[StockenBoard:${context}]`, err);
  }
}
