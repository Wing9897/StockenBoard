"""StockenBoard 完整檢查腳本"""
import subprocess, re, sys, os
from pathlib import Path

OK = 0
WARN = 0
ERR = 0

def ok(msg):
    global OK; OK += 1; print(f"  ✓ {msg}")
def warn(msg):
    global WARN; WARN += 1; print(f"  ⚠ {msg}")
def err(msg):
    global ERR; ERR += 1; print(f"  ✗ {msg}")

print("=" * 60)
print("StockenBoard 完整檢查")
print("=" * 60)

# ── 1. i18n locale files ──
print("\n[1] i18n locale 檔案檢查")
LOCALE_DIR = Path("src/lib/i18n")
LOCALES = ["zh_TW", "zh_CN", "en", "ja", "ko"]

EXPECTED_EXTRA_KEYS = [
    "open_price", "prev_close", "52w_high", "52w_low", "exchange",
    "weighted_avg_price", "trade_count", "quote_volume", "avg_price",
    "name", "pe_ratio", "eps", "circulating_supply", "cmc_rank",
    "change_7d_pct", "chain", "token", "est_gas", "question",
    "end_date", "outcomes", "pool_tvl", "volume_24h", "token_from",
    "token_to", "route_path", "gas_estimate", "amount_out",
]

EXPECTED_PROVIDER_KEYS = [
    "binance", "coinbase", "coingecko", "coinmarketcap", "cryptocompare",
    "yahoo", "marketstack", "eodhd", "mboum", "alpaca", "finnhub",
    "alphavantage", "polygon", "tiingo", "fmp", "twelvedata",
    "polymarket", "bitquery", "kraken", "bybit", "kucoin", "okx",
    "gateio", "bitfinex", "htx", "mexc", "coinpaprika", "coinapi",
    "fcsapi", "jupiter", "okx_dex", "raydium", "subgraph",
]

for loc in LOCALES:
    fpath = LOCALE_DIR / f"{loc}.ts"
    if not fpath.exists():
        err(f"{loc}.ts 不存在")
        continue
    content = fpath.read_text(encoding="utf-8")
    # Check extraFields — keys are unquoted TS object properties like `open_price: '...'`
    # except keys starting with digits need quotes like `'52w_high': '...'`
    missing_extra = [k for k in EXPECTED_EXTRA_KEYS if f"{k}:" not in content and f"'{k}':" not in content]
    if missing_extra:
        err(f"{loc}.ts extraFields 缺少: {missing_extra}")
    else:
        ok(f"{loc}.ts extraFields ({len(EXPECTED_EXTRA_KEYS)} keys)")
    # Check providerDesc — keys are unquoted TS object properties
    missing_prov = [k for k in EXPECTED_PROVIDER_KEYS if f"{k}:" not in content]
    if missing_prov:
        err(f"{loc}.ts providerDesc 缺少: {missing_prov}")
    else:
        ok(f"{loc}.ts providerDesc ({len(EXPECTED_PROVIDER_KEYS)} keys)")

# ── 2. ProviderSettings.tsx 磁碟驗證 ──
print("\n[2] ProviderSettings.tsx 磁碟驗證")
ps_path = Path("src/components/Settings/ProviderSettings.tsx")
ps_content = ps_path.read_text(encoding="utf-8")
checks = {
    "import { t }": "import { t }" in ps_content,
    "useLocale()": "useLocale()" in ps_content,
    "getDesc": "getDesc" in ps_content,
    "t.providerDesc": "t.providerDesc" in ps_content,
    "no hardcoded 加密貨幣": "加密貨幣" not in ps_content.split("//")[0],  # rough check
}
for label, passed in checks.items():
    if passed:
        ok(f"ProviderSettings: {label}")
    else:
        err(f"ProviderSettings: {label} FAILED")

# ── 3. DexPage.tsx 驗證 ──
print("\n[3] DexPage.tsx 驗證")
dp_path = Path("src/components/DexPage/DexPage.tsx")
dp_content = dp_path.read_text(encoding="utf-8")
if "useLocale()" in dp_content:
    ok("DexPage: useLocale() 存在")
else:
    err("DexPage: 缺少 useLocale()")
if "import { useLocale }" in dp_content:
    ok("DexPage: import useLocale 存在")
else:
    err("DexPage: 缺少 import useLocale")

# ── 4. App.tsx 驗證 ──
print("\n[4] App.tsx 驗證")
app_content = Path("src/App.tsx").read_text(encoding="utf-8")
toolbar_content = Path("src/components/DashboardToolbar/DashboardToolbar.tsx").read_text(encoding="utf-8")
if "useLocale()" in app_content:
    ok("App.tsx: useLocale() 存在")
else:
    err("App.tsx: 缺少 useLocale()")
# t.providers.all 已移至 DashboardToolbar 共用組件
if "t.providers.all" in app_content or "t.providers.all" in toolbar_content:
    ok("App.tsx/DashboardToolbar: default view 使用 t.providers.all")
else:
    err("App.tsx/DashboardToolbar: default view 未使用 t.providers.all")

# ── 5. AssetCard.tsx 驗證 ──
print("\n[5] AssetCard.tsx 驗證")
ac_content = Path("src/components/AssetCard/AssetCard.tsx").read_text(encoding="utf-8")
if "t.extraFields" in ac_content:
    ok("AssetCard: formatExtraKey 使用 t.extraFields")
else:
    err("AssetCard: formatExtraKey 未使用 t.extraFields")

# ── 6. Rust backend 驗證 ──
print("\n[6] Rust backend 驗證")
# db.rs: no 全部
db_content = Path("src-tauri/src/db.rs").read_text(encoding="utf-8")
if "全部" in db_content:
    err("db.rs: 仍有 '全部' (應為 'All')")
else:
    ok("db.rs: default view = 'All'")

# lib.rs: SCHEMA_VER matches migration version
lib_content = Path("src-tauri/src/lib.rs").read_text(encoding="utf-8")
schema_ver_match = re.search(r'SCHEMA_VER:\s*&str\s*=\s*"(\d+)"', lib_content)
migration_ver_match = re.search(r'version:\s*(\d+)', lib_content)
if schema_ver_match and migration_ver_match:
    sv = schema_ver_match.group(1)
    mv = migration_ver_match.group(1)
    if sv == mv:
        ok(f"lib.rs: SCHEMA_VER={sv} == migration version={mv}")
    else:
        err(f"lib.rs: SCHEMA_VER={sv} != migration version={mv}")
else:
    err("lib.rs: 無法解析 SCHEMA_VER 或 migration version")

# ensure_clean_db exists
if "ensure_clean_db" in lib_content:
    ok("lib.rs: ensure_clean_db 存在")
else:
    err("lib.rs: 缺少 ensure_clean_db")

# ── 7. Rust providers: no Chinese in free_tier_info ──
print("\n[7] Rust providers: free_tier_info 中文檢查")
PROVIDERS_DIR = Path("src-tauri/src/providers")
chinese_re = re.compile(r'[\u4e00-\u9fff]')
for rs_file in sorted(PROVIDERS_DIR.glob("*.rs")):
    content = rs_file.read_text(encoding="utf-8")
    file_ok = True
    # Check free_tier_info lines for Chinese
    for i, line in enumerate(content.splitlines(), 1):
        if "free_tier_info" in line and chinese_re.search(line):
            err(f"{rs_file.name}:{i} free_tier_info 含中文: {line.strip()[:80]}")
            file_ok = False
            break
    # Check extra field keys (HashMap insert keys) — pattern: .insert("key".to_string()
    # Only flag if the KEY itself is Chinese, not error messages in format!()
    for i, line in enumerate(content.splitlines(), 1):
        m = re.search(r'\.insert\(\s*"([^"]+)"\.to_string', line)
        if m and chinese_re.search(m.group(1)):
            err(f"{rs_file.name}:{i} extra key 含中文: {m.group(1)}")
            file_ok = False
            break
    if file_ok:
        ok(f"{rs_file.name}")

# ── 8. CSS 主題變數檢查 (無硬編碼顏色) ──
print("\n[8] CSS 主題變數檢查")
CSS_DIR = Path("src")
hex_re = re.compile(r'#[0-9a-fA-F]{3,8}\b')
rgba_re = re.compile(r'rgba\(')
var_fallback_re = re.compile(r'var\(--\w+,\s*#')
theme_css = Path("src/theme.css")

for css_file in sorted(CSS_DIR.rglob("*.css")):
    if css_file == theme_css:
        continue
    content = css_file.read_text(encoding="utf-8")
    rel = css_file.relative_to(Path("."))
    issues = []
    for i, line in enumerate(content.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("/*") or stripped.startswith("*") or stripped.startswith("//"):
            continue
        if hex_re.search(stripped):
            issues.append(f"  L{i}: 硬編碼 hex → {stripped[:80]}")
        if rgba_re.search(stripped):
            issues.append(f"  L{i}: 硬編碼 rgba → {stripped[:80]}")
        if var_fallback_re.search(stripped):
            issues.append(f"  L{i}: var() 帶 hex fallback → {stripped[:80]}")
    if issues:
        err(f"{rel}: {len(issues)} 處硬編碼顏色")
        for iss in issues[:5]:
            print(f"    {iss}")
    else:
        ok(f"{rel}")

# ── 9. TSX inline style 硬編碼顏色檢查 ──
print("\n[9] TSX inline style 硬編碼顏色檢查")
# ThemePicker.tsx 的 THEMES 色板是故意的，排除
EXEMPT_FILES = {"ThemePicker.tsx"}
inline_hex_re = re.compile(r"['\"]#[0-9a-fA-F]{3,8}['\"]")
inline_var_fallback_re = re.compile(r"var\(--\w+,\s*#[0-9a-fA-F]")

for tsx_file in sorted(CSS_DIR.rglob("*.tsx")):
    if tsx_file.name in EXEMPT_FILES:
        continue
    content = tsx_file.read_text(encoding="utf-8")
    rel = tsx_file.relative_to(Path("."))
    issues = []
    for i, line in enumerate(content.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("/*"):
            continue
        if inline_hex_re.search(stripped):
            issues.append(f"  L{i}: inline hex → {stripped[:90]}")
        if inline_var_fallback_re.search(stripped):
            issues.append(f"  L{i}: var() 帶 hex fallback → {stripped[:90]}")
    if issues:
        err(f"{rel}: {len(issues)} 處硬編碼顏色")
        for iss in issues[:5]:
            print(f"    {iss}")
    else:
        ok(f"{rel}")

# ── 10. SubscriptionManager i18n 檢查 ──
print("\n[10] SubscriptionManager i18n 檢查")
sm_content = Path("src/components/Settings/SubscriptionManager.tsx").read_text(encoding="utf-8")
if "t.providerDesc" in sm_content or "providerDesc" in sm_content:
    ok("SubscriptionManager: provider 描述使用 i18n")
else:
    # 檢查是否直接用 free_tier_info 而沒有 i18n fallback
    if ".free_tier_info" in sm_content:
        warn("SubscriptionManager: 使用 free_tier_info (有 i18n fallback 即可)")
    else:
        ok("SubscriptionManager: 無 free_tier_info 直接使用")

# ── 11. TypeScript 編譯 ──
print("\n[11] TypeScript 編譯 (tsc --noEmit)")
result = subprocess.run(
    ["npx", "tsc", "--noEmit"],
    capture_output=True, text=True, shell=True, cwd=".",
    encoding="utf-8", errors="replace"
)
if result.returncode == 0:
    ok("tsc --noEmit: 零錯誤")
else:
    err(f"tsc --noEmit 失敗:\n{result.stdout}\n{result.stderr}")

# ── 12. Vite build ──
print("\n[12] Vite build")
result = subprocess.run(
    ["npx", "vite", "build"],
    capture_output=True, text=True, shell=True, cwd=".",
    encoding="utf-8", errors="replace"
)
if result.returncode == 0:
    ok("vite build: 成功")
else:
    err(f"vite build 失敗:\n{result.stdout}\n{result.stderr}")

# ── Summary ──
print("\n" + "=" * 60)
print(f"結果: ✓ {OK} 通過  ⚠ {WARN} 警告  ✗ {ERR} 錯誤")
print("=" * 60)
sys.exit(1 if ERR > 0 else 0)
