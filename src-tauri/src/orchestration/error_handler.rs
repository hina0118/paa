//! 共通エラーハンドリング。
//!
//! オーケストレーター内で繰り返される「log → エラーイベント送信」パターンを
//! `ErrorReporter` に集約する。状態クリーンアップ（`set_error` / `finish`）は
//! 各状態型の API が異なるため呼び出し元に残す。

use crate::batch_runner::{BatchEventEmitter, BatchProgressEvent};

/// バッチ処理中のエラーを log + イベント送信する小さなヘルパー。
pub(crate) struct ErrorReporter<'a, A: BatchEventEmitter> {
    emitter: &'a A,
    task_name: &'static str,
    event_name: &'static str,
}

impl<'a, A: BatchEventEmitter> ErrorReporter<'a, A> {
    pub fn new(emitter: &'a A, task_name: &'static str, event_name: &'static str) -> Self {
        Self {
            emitter,
            task_name,
            event_name,
        }
    }

    /// エラーをログに記録し、`BatchProgressEvent::error` を送信する。
    pub fn report(
        &self,
        message: &str,
        total_items: usize,
        processed: usize,
        success: usize,
        failed: usize,
    ) {
        log::error!("{}", message);
        let error_event = BatchProgressEvent::error(
            self.task_name,
            total_items,
            processed,
            success,
            failed,
            message.to_string(),
        );
        self.emitter.emit_event(self.event_name, error_event);
    }

    /// カウンタがすべて 0 の場合の省略版。
    pub fn report_zero(&self, message: &str) {
        self.report(message, 0, 0, 0, 0);
    }
}
