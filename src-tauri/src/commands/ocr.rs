//! 画面OCR検索コマンド
//!
//! 半透明オーバーレイウィンドウの表示・スクリーンキャプチャ・OCR処理を行う。
//! OCR結果はメインウィンドウに `ocr-result` イベントとして送信される。

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const OVERLAY_LABEL: &str = "screen-overlay";

/// Tauriイベント名
pub const OCR_RESULT_EVENT: &str = "ocr-result";

/// オーバーレイウィンドウを表示する
///
/// 既に存在する場合は再表示する。新規作成時はフルスクリーン・透明・最前面で作成する。
#[tauri::command]
pub async fn show_screen_overlay(app_handle: AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window(OVERLAY_LABEL) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let win = WebviewWindowBuilder::new(
        &app_handle,
        OVERLAY_LABEL,
        WebviewUrl::App("overlay.html".into()),
    )
    .title("")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .fullscreen(true)
    .build()
    .map_err(|e| format!("Failed to create overlay window: {e}"))?;

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    log::info!("Screen overlay window created");
    Ok(())
}

/// オーバーレイウィンドウを閉じる
#[tauri::command]
pub fn close_screen_overlay(app_handle: AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window(OVERLAY_LABEL) {
        win.close().map_err(|e| e.to_string())?;
        log::info!("Screen overlay window closed");
    }
    Ok(())
}

/// 指定した画面領域をキャプチャしてGemini Vision APIでOCR処理を行う
///
/// OCR結果はメインウィンドウに `ocr-result` イベントとして送信され、
/// コマンドの戻り値としても返す。
///
/// # 引数
/// * `x`, `y` - スクリーン座標（物理ピクセル）
/// * `width`, `height` - キャプチャサイズ（物理ピクセル）
#[tauri::command]
pub async fn capture_and_ocr(
    app_handle: AppHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    if width == 0 || height == 0 {
        return Err("Selection area is too small".to_string());
    }

    log::info!("Capturing region: x={x}, y={y}, w={width}, h={height}");

    // 1. スクリーンキャプチャ
    let png_bytes = capture_region(x, y, width, height)?;

    // 2. Gemini Vision OCR
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let api_key = crate::gemini::config::load_api_key(&app_data_dir).map_err(|_| {
        "Gemini APIキーが設定されていません。設定画面からAPIキーを登録してください。".to_string()
    })?;

    let text = crate::gemini::ocr_image_bytes(&api_key, &png_bytes).await?;

    // 3. メインウィンドウに結果を送信
    if let Some(main_win) = app_handle.get_webview_window("main") {
        main_win
            .emit(OCR_RESULT_EVENT, &text)
            .map_err(|e| format!("Failed to emit ocr-result: {e}"))?;
    }

    Ok(text)
}

/// 指定座標・サイズの画面領域をキャプチャしてPNGバイト列を返す
fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::DynamicImage;
    use xcap::Monitor;

    let monitors = Monitor::all().map_err(|e| format!("Failed to enumerate monitors: {e}"))?;

    // 座標を含むモニターを特定（見つからなければプライマリを使用）
    let monitor = monitors
        .iter()
        .find(|m| {
            let mx = m.x();
            let my = m.y();
            let mw = m.width() as i32;
            let mh = m.height() as i32;
            x >= mx && y >= my && x < mx + mw && y < my + mh
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| "No monitor found".to_string())?;

    let full_image = monitor
        .capture_image()
        .map_err(|e| format!("Failed to capture screen: {e}"))?;

    // モニター相対座標に変換してクロップ
    let rel_x = (x - monitor.x()).max(0) as u32;
    let rel_y = (y - monitor.y()).max(0) as u32;

    let dynamic_img = DynamicImage::ImageRgba8(full_image);
    let cropped = dynamic_img.crop_imm(rel_x, rel_y, width, height);

    let mut png_bytes: Vec<u8> = Vec::new();
    cropped
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("Failed to encode screenshot as PNG: {e}"))?;

    log::info!("Captured region: {} bytes (PNG)", png_bytes.len());
    Ok(png_bytes)
}
