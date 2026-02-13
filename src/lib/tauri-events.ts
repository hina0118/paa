/**
 * Tauri イベント名の定数定義
 * Rust側の定数と一致させること
 */

/**
 * クリップボード監視でURL検知時にemitされるイベント
 * @see src-tauri/src/clipboard_watcher.rs - CLIPBOARD_URL_DETECTED_EVENT
 */
export const CLIPBOARD_URL_DETECTED_EVENT = 'clipboard-url-detected';
