/**
 * 集中管理所有 localStorage key — 避免 magic string 分散在各檔案。
 */
export const STORAGE_KEYS = {
    // App
    ACTIVE_TAB: 'sb_active_tab',
    VIEW_MODE: 'sb_view_mode',
    EXPAND_ALL: 'sb_expand_all',
    HIDE_PREPOST: 'sb_hide_prepost',
    THEME: 'sb_theme',
    UNATTENDED: 'sb_unattended',

    // Views — Asset
    ACTIVE_VIEW_ID: 'sb_active_view_id',
    PINNED_VIEWS: 'sb_pinned_views',

    // Views — DEX
    DEX_ACTIVE_VIEW_ID: 'sb_dex_active_view_id',
    DEX_VIEW_MODE: 'sb_dex_view_mode',
    DEX_PINNED_VIEWS: 'sb_dex_pinned_views',

    /** 動態 key：主題背景透明度 */
    themeBgOpacity: (themeId: string) => `sb_theme_bg_opacity_${themeId}`,
} as const;
