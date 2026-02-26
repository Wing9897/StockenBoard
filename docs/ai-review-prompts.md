# AI Code Review Prompt Library | AI 程式碼審查 Prompt 庫

> 20 個核心審查 prompt，每個涵蓋一整個範疇，同類檢查項目已整合為一段，減少碎片化。
> 適用於 GUI / 前端 / 後端 / 網路 / 系統等不同類型專案。
> 每個 prompt 可直接貼給 AI，觸發深度檢查並修復。

---

## 1. Code Quality Deep Audit | 代碼品質深度審計

> 範疇：代碼整合、優化、去複雜化、修復

對整個 codebase 做一次完整的深度審計，涵蓋代碼整合、優化、去複雜化、修復四個面向。具體要求：找出所有散落在多個檔案中的重複邏輯，抽取為共用函式、hook 或工具模組；識別過度複雜的函式（超過 80 行或巢狀超過 3 層）並拆分為職責單一的小單元；偵測並移除 dead code、未使用的 import/export、不可達的分支；找出邏輯 bug（off-by-one、錯誤的型別轉換、被靜默吞掉但應該浮出的錯誤）並修復；檢查不一致的模式（同一件事在不同地方用不同方式實作）並統一；驗證所有 async 錯誤處理是否正確（無 unhandled promise rejection、無該顯示給使用者卻被 silent catch 的錯誤）。修復後執行 tsc --noEmit、build、diagnostics 確認零錯誤零警告。

---

## 2. State Persistence & Consistency | 狀態持久化與一致性

> 範疇：前端狀態、後端記憶體、資料庫、localStorage、快取同步

審計所有層級的應用程式狀態（前端 React state、localStorage、後端記憶體狀態、資料庫），逐一驗證：每個使用者可見的設定和偏好是否正確持久化並在 app 重啟後恢復；前端狀態、後端記憶體、資料庫三者之間有沒有 desync 風險（例如某個開關存在 localStorage 但啟動時沒同步回後端）；所有快取（記憶體、磁碟、CDN）是否有正確的失效機制，不會提供過期資料；如果 app 在操作中途 crash 或被強制關閉，有沒有資料會處於不一致狀態（識別需要 transaction 或 rollback 的操作）；每筆資料的儲存位置是否合理（localStorage vs DB vs 檔案 vs 後端記憶體）。列出每個狀態項目、它的儲存位置、以及任何缺漏，然後修復所有問題。

---

## 3. Error Handling & Resilience | 錯誤處理與系統韌性

> 範疇：try/catch、輸入驗證、並發安全、網路容錯、邊界情況

審查整個技術棧（前端、後端、資料庫、網路）的錯誤處理。檢查：每個外部呼叫（API、DB 查詢、檔案 I/O、IPC invoke、網路請求）是否都有適當的 try/catch，且 catch 後的行為合理（背景任務 silent、使用者操作顯示 toast、暫時性失敗重試）；所有使用者輸入是否有驗證（空字串、超長字串、特殊字元、SQL injection、XSS、path traversal）；並發操作是否安全（快速連點按鈕、多個 async 操作競爭、重複提交是否有 debounce 或 loading lock）；網路斷線、API timeout、伺服器 500 時使用者看到的是什麼（有沒有 fallback UI 或重試機制）；所有 nullable 值、除以零、陣列越界、型別轉換邊界情況是否都有處理。找到的每個問題都要修復。

---

## 4. i18n & Localization | 多語言完整性

> 範疇：前端 UI、後端錯誤訊息、DB 預設值、通知、tooltip、格式化

審計整個 i18n 系統，不只是前端 UI，要涵蓋所有層級。檢查：前端元件、後端錯誤訊息、資料庫預設值、通知文字、tooltip、placeholder、aria-label、確認對話框中的每一個使用者可見字串是否都經過 i18n 系統，沒有 hardcoded 字串；每個 i18n key 在所有語言檔案中都有對應翻譯，沒有遺漏；動態組合的字串使用參數化函式（而非字串拼接），確保不同語言的語序都正確；日期、數字、貨幣的格式化是否尊重使用者的 locale 設定；切換語言後所有已渲染的元件是否即時更新，沒有殘留舊語言的文字；後端是否有回傳繞過前端 i18n 層的 locale-specific 字串。列出所有違規項目並修復。

---

## 5. Architecture & Dependencies | 架構、依賴結構與命名一致性

> 範疇：分層、循環依賴、職責單一、命名、magic number、資料夾結構

分析完整的架構和依賴圖。驗證：依賴方向是否嚴格單向（types <- lib <- hooks <- components），沒有循環依賴或反向 import；每個模組、函式、元件是否職責單一，沒有 god object 或超大函式；命名是否全 codebase 一致（同一個概念在所有地方用同一個名稱，沒有 provider vs source vs datasource 的混亂）；所有 magic number 和 magic string 是否已提取為具名常數或設定；每個模組的公開 API 表面是否最小化（沒有不必要的 export）；資料夾結構是否與邏輯架構匹配，沒有放錯位置的檔案。畫出依賴圖並標記所有違規。

---

## 6. Performance & Resource Usage | 效能與資源使用

> 範疇：演算法複雜度、re-render、記憶體洩漏、N+1 查詢、批次請求

對整個應用做效能審計。檢查：有沒有隱藏的 O(N²) 或更差的演算法（巢狀 filter/find、迴圈內重複查詢 DB、重複遍歷大陣列）；React 元件有沒有不必要的 re-render（缺少 memo、useMemo、useCallback 導致的瀑布式重繪）；大量資料（1000+ 筆）的情況下 UI 是否流暢（是否需要虛擬捲動、分頁、debounce）；所有 event listener、subscription、timer 是否在 unmount 時正確清理（記憶體洩漏風險）；網路請求有沒有不必要的重複呼叫（可以合併的批次請求、可以快取的重複查詢）；後端有沒有阻塞主執行緒的同步操作、不必要的 clone、或可以用 streaming 取代的大量記憶體分配；資料庫查詢是否有適當的索引，有沒有 N+1 查詢問題。找到的每個效能瓶頸都要修復或提出具體改善方案。

---

## 7. Security Audit | 安全性全面檢查

> 範疇：敏感資料、注入攻擊、路徑穿越、依賴漏洞、權限控制

對整個應用做安全性審計，涵蓋前端、後端、資料庫、檔案系統。檢查：所有敏感資料（API key、密碼、token、secret）的儲存和傳輸是否安全，有沒有明文暴露在 log、URL、前端 state、或版本控制中；所有檔案路徑操作是否防止 path traversal 攻擊，使用者提供的檔名是否有 sanitize；所有 SQL 查詢是否使用參數化查詢，沒有字串拼接 SQL；所有使用者輸入是否有防 XSS、防注入的處理；CORS、CSP、權限控制等安全設定是否正確；第三方依賴有沒有已知的安全漏洞（npm audit / cargo audit）；有沒有不安全的 eval、innerHTML、dangerouslySetInnerHTML 使用。列出所有安全風險並修復。

---

## 8. Data Integrity & DB Health | 資料完整性與 DB 健康度

> 範疇：外鍵、transaction、migration、孤兒資料、索引、並發寫入

審計資料庫 schema、migration、和所有資料操作的完整性。檢查：所有外鍵關係是否有正確的 CASCADE 或 RESTRICT（刪除父記錄時子記錄會怎樣）；所有需要原子性的多步驟操作是否包在 transaction 中；資料匯入/匯出格式是否向前相容（舊版匯出的資料能被新版正確匯入）；DB migration 的版本管理是否正確（schema version 和 migration version 是否同步）；有沒有孤兒資料的風險（例如刪除訂閱後 view_subscriptions 有沒有一起清理）；所有 DB 欄位的型別、nullable、default 值是否合理；有沒有缺少的索引導致查詢效能差；並發寫入是否安全（SQLite WAL mode、鎖定策略）。列出所有問題並修復。

---

## 9. UI/UX Consistency | UI/UX 一致性與完整性

> 範疇：操作流程、狀態回饋、modal 行為、空狀態、主題跟隨

審查整個 UI 的一致性和完整性。檢查：所有相似功能的操作流程是否一致（如新增 asset vs 新增 dex 的步驟、確認對話框的行為、錯誤顯示方式）；所有可互動元素是否都有 hover / active / disabled / loading / error 五種狀態的視覺回饋；所有 modal 和 dialog 是否都支援 Escape 關閉和點擊背景關閉；所有破壞性操作（刪除、覆蓋、批量操作）是否都有確認對話框；所有非同步操作是否都有 loading 指示器讓使用者知道操作正在進行；空狀態（無資料）、錯誤狀態、載入狀態是否都有適當的 UI 處理；所有列表是否支援排序、搜尋、篩選（如果資料量可能很大）；主題切換後所有元件的顏色、邊框、陰影是否都正確跟隨主題變數。列出所有不一致或缺失的地方並修復。

---

## 10. Accessibility | 無障礙與鍵盤操作

> 範疇：aria 屬性、鍵盤導航、色彩對比、螢幕閱讀器、focus 管理

審查整個應用的無障礙支援。檢查：所有互動元素是否有適當的 aria-label、role、aria-modal、aria-expanded、aria-selected 等屬性；純鍵盤操作能否完成所有功能（Tab 順序是否合理、Enter/Space 能否觸發按鈕、方向鍵能否在列表中導航）；所有顏色對比度是否足夠（WCAG AA 標準），有沒有僅靠顏色傳達資訊的地方（色盲使用者無法辨識）；動態內容更新（toast 通知、即時價格變動、錯誤訊息）是否用 aria-live 通知螢幕閱讀器；所有圖片和圖示是否有 alt text 或 aria-label；focus 管理是否正確（開啟 modal 時 focus 移入、關閉時 focus 回到觸發元素）；字體大小是否使用相對單位（rem/em），支援使用者放大。列出所有違規並修復。

---

## 11. Cross-Layer Integration | 跨層整合與型別安全

> 範疇：前後端介面、型別同步、event payload、預設值一致性

審查前後端之間的所有介面點。檢查：前端呼叫的每個後端 command/API，後端都有實作且參數型別和回傳型別前後端一致；前端的 TypeScript 型別定義和後端的 struct 定義是否同步（欄位名稱、型別、nullable 是否匹配）；所有 event（Tauri event、WebSocket message、DOM event）的 payload 格式，發送端和接收端是否一致；前端假設的預設值和後端假設的預設值是否一致（如預設 provider、預設 interval、預設排序）；錯誤碼和錯誤訊息的格式前後端是否統一（前端能否正確解析後端回傳的錯誤）；前端的 enum/union type 和後端的 enum 是否涵蓋相同的值集合。列出所有不匹配並修復。

---

## 12. CSS & Theme System | CSS 與主題系統

> 範疇：CSS 變數、未使用樣式、主題完整性、RWD、z-index、動畫

審查整個 CSS 和主題系統。檢查：所有顏色、字型、間距、圓角、陰影是否都使用 CSS 變數（而非 hardcoded 值），確保主題切換時全部正確跟隨；有沒有未使用的 CSS class、重複的樣式定義、或互相覆蓋的規則；所有主題（包含 anime 主題）的 CSS 變數是否完整，沒有某個主題缺少某個變數導致 fallback 到預設值；RWD 響應式設計是否正確（不同視窗寬度下版面是否正常）；z-index 的使用是否有統一的層級系統（modal > toast > dropdown > content）；動畫和 transition 是否流暢且不影響效能；有沒有 CSS 選擇器過於寬泛導致意外影響其他元件。列出所有問題並修復。

---

## 13. Build, Deploy & Cross-Platform | 建置、部署與跨平台

> 範疇：零 warning 建置、環境變數、平台差異、依賴版本

驗證整個建置和部署流程。檢查：tsc --noEmit、vite build、cargo check 是否全部零 error 零 warning（被忽略的 warning 是否其實是潛在問題）；所有環境變數、設定檔路徑、平台相關程式碼（Windows / macOS / Linux 的路徑分隔符、換行符、檔案系統大小寫敏感度）都有正確處理；package.json 和 Cargo.toml 的依賴版本是否合理（有沒有過時的依賴、有沒有不必要的依賴、版本鎖定策略是否正確）；build output 的大小是否合理（有沒有不必要的大型依賴被打包進去）；開發環境和生產環境的行為是否一致（有沒有只在 dev 模式下才能運作的功能）。列出所有問題並修復。

---

## 14. Testing & Testability | 測試與可測試性

> 範疇：測試覆蓋、可測試架構、mock 策略、邊界測試

審查整個應用的測試狀況和可測試性。檢查：核心業務邏輯（data layer、格式化函式、狀態管理）是否有單元測試覆蓋；data layer 是否與 React 解耦，可以獨立測試（不需要渲染元件就能測試 CRUD 邏輯）；有沒有難以測試的緊耦合程式碼（直接在元件內呼叫 API 而非透過注入的 hook）；關鍵的邊界情況（空陣列、null 值、超大數字、特殊字元）是否有測試；如果沒有測試，至少確認架構是可測試的（純函式、依賴注入、關注點分離），並為最關鍵的 3-5 個模組補上測試。

---

## 15. Logging & Observability | 日誌與可觀測性

> 範疇：錯誤日誌、操作追蹤、效能指標、除錯資訊

審查整個應用的日誌和可觀測性。檢查：所有被 catch 的錯誤是否有適當的日誌記錄（至少 eprintln 或 console.error），而不是完全靜默；關鍵操作（訂閱新增/刪除、API key 設定、資料匯入匯出）是否有操作日誌可供除錯；後端的 polling、WebSocket 連線、provider 切換等背景操作是否有足夠的日誌來診斷問題；有沒有過多的日誌輸出（如每次 price update 都 log）影響效能；錯誤日誌是否包含足夠的上下文資訊（哪個 provider、哪個 symbol、什麼操作）而不只是 generic error message。列出日誌不足或過多的地方並調整。

---

## 16. API Design & Contract | API 設計與契約

> 範疇：RESTful 設計、命名慣例、版本控制、錯誤回傳格式、文件

審查所有 API（包含 Tauri command、REST endpoint、IPC 介面）的設計品質。檢查：命名是否一致且語義清晰（get_ vs fetch_ vs load_ 是否有統一慣例）；參數設計是否合理（有沒有過多的可選參數、有沒有應該拆分的巨型 command）；回傳值的結構是否一致（成功和失敗的格式是否統一）；有沒有缺少的 API（前端需要但後端沒提供，導致前端用 workaround）；API 的版本控制策略是否存在（新增欄位時舊版前端會不會壞掉）；每個 API 的用途是否有文件或至少有清晰的函式簽名說明。列出所有設計問題並提出改善方案。

---

## 17. Concurrency & Async Patterns | 並發與非同步模式

> 範疇：race condition、deadlock、資源競爭、async 生命週期、取消機制

深入審查所有並發和非同步操作。檢查：有沒有 race condition（如使用者快速切換 view 時，舊的 async 載入結果覆蓋新的）；有沒有 deadlock 風險（後端的 RwLock、Mutex 是否有巢狀鎖定）；React 元件 unmount 後是否還有 async 操作試圖 setState（記憶體洩漏 + React warning）；長時間運行的操作（批量匯入、大量刪除）是否有取消機制或至少有進度回報；WebSocket 重連邏輯是否正確（斷線後自動重連、重連時不重複訂閱）；多個 polling 週期是否有可能重疊（上一次還沒完成下一次就開始了）。列出所有並發風險並修復。

---

## 18. Configuration & Environment | 設定管理與環境

> 範疇：硬編碼設定、環境差異、feature flag、預設值管理

審查所有設定和環境相關的程式碼。檢查：有沒有應該是可設定的值被硬編碼在程式碼中（如 API URL、timeout 時間、重試次數、分頁大小）；開發環境和生產環境的設定是否正確分離；有沒有 feature flag 機制來控制實驗性功能的開關；所有預設值是否合理且有文件說明（為什麼 default interval 是 30000ms？為什麼 default provider 是 binance？）；設定變更是否需要重啟 app 才能生效（應該即時生效的有沒有做到）；敏感設定（API key）和非敏感設定（主題偏好）的儲存方式是否有適當區分。列出所有問題並修復。

---

## 19. Documentation & Code Comments | 文件與代碼註解

> 範疇：函式文件、架構文件、API 文件、TODO/FIXME、README

審查整個專案的文件和註解品質。檢查：所有公開函式和模組是否有清晰的 JSDoc/rustdoc 說明其用途、參數、回傳值；複雜的業務邏輯是否有註解解釋「為什麼」這樣做（而不只是「做了什麼」）；有沒有過時的註解（程式碼已改但註解沒更新）；所有 TODO、FIXME、HACK 標記是否還有效（有沒有已經解決但忘記移除的）；README 是否準確反映目前的專案狀態（安裝步驟、開發指令、架構說明）；有沒有缺少的架構文件（新開發者能否只看文件就理解系統設計）。列出所有文件缺失或過時的地方並修復。

---

## 20. Full System Integration Test | 全系統整合驗證

> 範疇：端到端流程、資料流完整性、邊界場景、回歸測試

對整個系統做一次端到端的整合驗證。模擬以下完整流程並確認每一步都正確：新使用者首次啟動（DB 初始化、預設 view 建立、空狀態 UI）；新增第一個訂閱（驗證 -> 寫入 DB -> 觸發 polling -> 價格更新 -> UI 顯示）；切換 provider 並設定 API key（儲存 -> 同步後端 -> 重新 polling）；建立自訂 view 並管理訂閱（建立 -> 新增/移除訂閱 -> 切換 view -> 驗證篩選）；批量操作（批量新增 -> 批量刪除 -> 確認 DB 和 UI 一致）；資料匯出再匯入（匯出 -> 清空 -> 匯入 -> 驗證資料完整）；切換主題和語言（即時生效 -> 重啟後保持）；所有 edge case（零訂閱、單一訂閱、100+ 訂閱的效能）。列出每個流程中發現的問題並修復。

---

> **使用建議**：不需要一次全部執行。根據當前開發階段選擇最相關的 3-5 個 prompt 執行即可。
> 建議優先順序：功能開發中 → 1, 3, 11 | 準備發布 → 2, 7, 8, 13 | 維護期 → 5, 6, 17 | 全面體檢 → 20
