import Database from '@tauri-apps/plugin-sql';

const DB_URL = 'sqlite:stockenboard.db';

/** 共用 DB instance — 全局單例，失敗時自動重試 */
let _dbPromise: Promise<InstanceType<typeof Database>> | null = null;

export function getDb(): Promise<InstanceType<typeof Database>> {
  if (!_dbPromise) {
    _dbPromise = Database.load(DB_URL).catch((err) => {
      // 清除快取，下次呼叫會重試
      _dbPromise = null;
      throw err;
    });
  }
  return _dbPromise;
}
