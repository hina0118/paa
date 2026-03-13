//! パース系バッチ処理の共通状態管理
//!
//! # 概要
//! メールパース・商品名パース・配送確認・駿河屋マイページ取得など、
//! ローカル DB データを処理するバッチ処理は共通のパターンを持つ。
//! - 多重起動ガード (`is_running`)
//! - キャンセル制御 (`should_cancel`)
//! - 直近エラー保持 (`last_error`)
//!
//! `BatchRunState` はそれらの共通実装を提供する。各バッチの状態型は
//! 本型をフィールドとして内包し、固有のエラーメッセージや追加メソッドを
//! 必要に応じてラップする。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// パース系バッチ処理の共通状態管理
///
/// `Arc` で包まれたフィールドを持つため `Clone` は浅いコピーを行い、
/// クローン間で状態が共有される。
#[derive(Clone, Default)]
pub struct BatchRunState {
    is_running: Arc<Mutex<bool>>,
    should_cancel: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl BatchRunState {
    pub fn new() -> Self {
        Self::default()
    }

    /// バッチを開始する。既に実行中なら `Err` を返す。
    ///
    /// 成功時はキャンセルフラグと `last_error` をリセットする。
    pub fn try_start(&self) -> Result<(), String> {
        let mut running = self
            .is_running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if *running {
            return Err("既に実行中です。".to_string());
        }
        *running = true;
        self.should_cancel.store(false, Ordering::SeqCst);
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
        Ok(())
    }

    /// バッチ完了時に呼ぶ。`is_running` とキャンセルフラグをリセットする。
    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.should_cancel.store(false, Ordering::SeqCst);
    }

    /// キャンセルを要求する。
    pub fn request_cancel(&self) {
        self.should_cancel.store(true, Ordering::SeqCst);
    }

    /// キャンセルフラグを返す。
    pub fn should_cancel(&self) -> bool {
        self.should_cancel.load(Ordering::SeqCst)
    }

    /// エラーメッセージを記録する。次回 `try_start` でクリアされる。
    pub fn set_error(&self, msg: &str) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg.to_string());
        }
    }

    /// エラーメッセージをクリアする。
    pub fn clear_error(&self) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
    }

    /// 直近のエラーメッセージを返す（なければ `None`）。
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|g| g.clone())
    }

    /// 実行中かどうかを返す。
    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|g| *g).unwrap_or(false)
    }

    /// `is_running` を `false` にリセットし、`last_error` をクリアする。
    ///
    /// キャンセルフラグはリセットしない点で `finish` と異なる。
    /// ウィンドウが強制終了された場合など、外部要因で状態をリセットする際に使用する。
    pub fn force_idle(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.clear_error();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let s = BatchRunState::new();
        assert!(!s.is_running());
        assert!(!s.should_cancel());
        assert!(s.last_error().is_none());
    }

    #[test]
    fn test_try_start_and_finish() {
        let s = BatchRunState::new();
        assert!(s.try_start().is_ok());
        assert!(s.is_running());
        assert!(s.try_start().is_err()); // 二重起動はエラー
        s.finish();
        assert!(!s.is_running());
        assert!(s.try_start().is_ok()); // finish 後は再起動可能
    }

    #[test]
    fn test_try_start_resets_cancel_and_error() {
        let s = BatchRunState::new();
        s.request_cancel();
        s.set_error("previous error");
        s.try_start().unwrap();
        assert!(!s.should_cancel());
        assert!(s.last_error().is_none());
    }

    #[test]
    fn test_cancel_and_finish() {
        let s = BatchRunState::new();
        s.try_start().unwrap();
        s.request_cancel();
        assert!(s.should_cancel());
        s.finish();
        assert!(!s.should_cancel());
    }

    #[test]
    fn test_set_and_clear_error() {
        let s = BatchRunState::new();
        s.set_error("something went wrong");
        assert_eq!(s.last_error(), Some("something went wrong".to_string()));
        s.clear_error();
        assert!(s.last_error().is_none());
    }

    #[test]
    fn test_force_idle() {
        let s = BatchRunState::new();
        s.try_start().unwrap();
        s.set_error("err");
        s.request_cancel();
        s.force_idle();
        assert!(!s.is_running());
        assert!(s.last_error().is_none());
        // force_idle はキャンセルフラグをリセットしない
        assert!(s.should_cancel());
    }

    #[test]
    fn test_clone_shares_state() {
        let s = BatchRunState::new();
        let c = s.clone();
        s.try_start().unwrap();
        assert!(c.is_running()); // クローンは同じ Arc を共有
        s.finish();
        assert!(!c.is_running());
    }
}
