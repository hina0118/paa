//! 汎用的なバッチ処理エンジン
//!
//! # 概要
//! メール同期、メールパース、商品名パースなどの各バッチ処理で共通して使用される
//! 処理ロジックを一元化し、進捗通知やレート制限を統一的に管理します。
//!
//! # 使用例
//! ```ignore
//! use crate::batch_runner::{BatchTask, BatchRunner, BatchProgressEvent};
//!
//! struct MyTask;
//!
//! #[async_trait]
//! impl BatchTask for MyTask {
//!     type Input = String;
//!     type Output = i32;
//!     type Context = MyContext;
//!
//!     fn name(&self) -> &str { "マイタスク" }
//!     fn event_name(&self) -> &str { "my-task-progress" }
//!
//!     async fn process(&self, input: Self::Input, ctx: &Self::Context) -> Result<Self::Output, String> {
//!         // 1件分の処理
//!         Ok(input.len() as i32)
//!     }
//! }
//!
//! let runner = BatchRunner::new(MyTask, 10, 1000);
//! let result = runner.run(&app_handle, inputs, &context, || false).await?;
//! ```

use async_trait::async_trait;
use serde::Serialize;
use std::time::Duration;
use tauri::Emitter;
use tokio::time::sleep;

/// バッチ処理の1タスクを定義するトレイト
///
/// このトレイトを実装することで、`BatchRunner`による統一的なバッチ処理が可能になります。
///
/// # フック
/// - `before_batch`: バッチ処理前に呼び出される（キャッシュ一括取得等に使用）
/// - `process_batch`: バッチ単位での処理（デフォルトは1件ずつ `process` を呼び出す）
/// - `after_batch`: バッチ処理後に呼び出される（一括DB保存、メタデータ更新等に使用）
#[async_trait]
pub trait BatchTask: Send + Sync {
    /// 入力データの型
    type Input: Send + Clone;
    /// 出力データの型
    type Output: Send;
    /// コンテキスト（DBプールやAPIクライアントなど）の型
    type Context: Send + Sync;

    /// タスク名（ログやUI表示用）
    fn name(&self) -> &str;

    /// イベント名（Tauriのemit用）
    fn event_name(&self) -> &str;

    /// 1件分の処理を実行
    ///
    /// # Arguments
    /// * `input` - 処理対象の入力データ
    /// * `context` - 処理に必要なコンテキスト（DBプール、APIクライアントなど）
    ///
    /// # Returns
    /// 処理結果、またはエラーメッセージ
    async fn process(
        &self,
        input: Self::Input,
        context: &Self::Context,
    ) -> Result<Self::Output, String>;

    /// バッチ処理前のフック（オプション）
    ///
    /// キャッシュの一括取得など、バッチ処理前に行いたい処理を実装します。
    /// デフォルトは何もしません。
    ///
    /// # Arguments
    /// * `inputs` - このバッチで処理する入力データのスライス
    /// * `context` - 処理に必要なコンテキスト
    async fn before_batch(
        &self,
        _inputs: &[Self::Input],
        _context: &Self::Context,
    ) -> Result<(), String> {
        Ok(())
    }

    /// バッチ単位での処理（オプション）
    ///
    /// デフォルトは1件ずつ `process` を呼び出しますが、
    /// チャンク単位でAPI呼び出しを行いたい場合などにオーバーライドします。
    ///
    /// # Arguments
    /// * `inputs` - このバッチで処理する入力データ
    /// * `context` - 処理に必要なコンテキスト
    ///
    /// # Returns
    /// 各入力に対する処理結果のベクタ（入力と同じ順序）
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.process(input, context).await);
        }
        results
    }

    /// バッチ処理後のフック（オプション）
    ///
    /// 一括DB保存やメタデータ更新など、バッチ処理後に行いたい処理を実装します。
    /// デフォルトは何もしません。
    ///
    /// # Arguments
    /// * `batch_number` - 現在のバッチ番号（1から開始）
    /// * `results` - このバッチの処理結果
    /// * `context` - 処理に必要なコンテキスト
    async fn after_batch(
        &self,
        _batch_number: usize,
        _results: &[Result<Self::Output, String>],
        _context: &Self::Context,
    ) -> Result<(), String> {
        Ok(())
    }
}

/// バッチ処理の進捗イベント（フロントエンドへの通知用）
#[derive(Debug, Clone, Serialize)]
pub struct BatchProgressEvent {
    /// タスク名（"メール同期", "メールパース", "商品名パース" など）
    pub task_name: String,
    /// 現在のバッチ番号（1から開始）
    pub batch_number: usize,
    /// このバッチで処理した件数
    pub batch_size: usize,
    /// 全体の処理対象件数
    pub total_items: usize,
    /// これまでに処理した件数
    pub processed_count: usize,
    /// 成功件数
    pub success_count: usize,
    /// 失敗件数
    pub failed_count: usize,
    /// 進捗率（0.0 ~ 100.0）
    pub progress_percent: f32,
    /// 状態メッセージ
    pub status_message: String,
    /// 処理完了フラグ
    pub is_complete: bool,
    /// エラーメッセージ（エラー時のみ）
    pub error: Option<String>,
}

impl BatchProgressEvent {
    /// 進捗イベントを作成
    pub fn progress(
        task_name: &str,
        batch_number: usize,
        batch_size: usize,
        total_items: usize,
        processed_count: usize,
        success_count: usize,
        failed_count: usize,
        status_message: String,
    ) -> Self {
        let progress_percent = if total_items > 0 {
            (processed_count as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };
        Self {
            task_name: task_name.to_string(),
            batch_number,
            batch_size,
            total_items,
            processed_count,
            success_count,
            failed_count,
            progress_percent,
            status_message,
            is_complete: false,
            error: None,
        }
    }

    /// 完了イベントを作成
    pub fn complete(
        task_name: &str,
        total_items: usize,
        success_count: usize,
        failed_count: usize,
        status_message: String,
    ) -> Self {
        Self {
            task_name: task_name.to_string(),
            batch_number: 0,
            batch_size: 0,
            total_items,
            processed_count: total_items,
            success_count,
            failed_count,
            progress_percent: 100.0,
            status_message,
            is_complete: true,
            error: None,
        }
    }

    /// エラーイベントを作成
    pub fn error(
        task_name: &str,
        total_items: usize,
        processed_count: usize,
        success_count: usize,
        failed_count: usize,
        error_message: String,
    ) -> Self {
        let progress_percent = if total_items > 0 {
            (processed_count as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };
        Self {
            task_name: task_name.to_string(),
            batch_number: 0,
            batch_size: 0,
            total_items,
            processed_count,
            success_count,
            failed_count,
            progress_percent,
            status_message: error_message.clone(),
            is_complete: true,
            error: Some(error_message),
        }
    }

    /// キャンセルイベントを作成
    pub fn cancelled(
        task_name: &str,
        total_items: usize,
        processed_count: usize,
        success_count: usize,
        failed_count: usize,
    ) -> Self {
        let progress_percent = if total_items > 0 {
            (processed_count as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };
        Self {
            task_name: task_name.to_string(),
            batch_number: 0,
            batch_size: 0,
            total_items,
            processed_count,
            success_count,
            failed_count,
            progress_percent,
            status_message: "処理がキャンセルされました".to_string(),
            is_complete: true,
            error: Some("Cancelled by user".to_string()),
        }
    }
}

/// バッチ処理の結果
#[derive(Debug, Clone)]
pub struct BatchResult<O> {
    /// 処理結果のリスト
    pub outputs: Vec<O>,
    /// 成功件数
    pub success_count: usize,
    /// 失敗件数
    pub failed_count: usize,
}

/// バッチ処理エンジン
///
/// `BatchTask`を実装したタスクを、指定されたバッチサイズとディレイで実行します。
pub struct BatchRunner<T: BatchTask> {
    task: T,
    batch_size: usize,
    delay_ms: u64,
}

impl<T: BatchTask> BatchRunner<T> {
    /// 新しいBatchRunnerを作成
    ///
    /// # Arguments
    /// * `task` - 実行するタスク
    /// * `batch_size` - 1バッチあたりの処理件数
    /// * `delay_ms` - バッチ間のディレイ（ミリ秒）
    pub fn new(task: T, batch_size: usize, delay_ms: u64) -> Self {
        Self {
            task,
            batch_size,
            delay_ms,
        }
    }

    /// バッチ処理を実行
    ///
    /// # Arguments
    /// * `app_handle` - Tauriアプリケーションハンドル（進捗イベント送信用）
    /// * `inputs` - 処理対象の入力データリスト
    /// * `context` - 処理に必要なコンテキスト
    /// * `should_cancel` - キャンセルチェック関数（trueを返すと処理を中断）
    ///
    /// # Returns
    /// バッチ処理の結果
    pub async fn run(
        &self,
        app_handle: &tauri::AppHandle,
        inputs: Vec<T::Input>,
        context: &T::Context,
        should_cancel: impl Fn() -> bool,
    ) -> Result<BatchResult<T::Output>, String> {
        let total_items = inputs.len();
        let task_name = self.task.name();
        let event_name = self.task.event_name();

        log::info!(
            "[{}] Starting batch processing: {} items, batch_size={}, delay={}ms",
            task_name,
            total_items,
            self.batch_size,
            self.delay_ms
        );

        if total_items == 0 {
            let event = BatchProgressEvent::complete(task_name, 0, 0, 0, "処理対象がありません".to_string());
            let _ = app_handle.emit(event_name, event);
            return Ok(BatchResult {
                outputs: Vec::new(),
                success_count: 0,
                failed_count: 0,
            });
        }

        let mut outputs: Vec<T::Output> = Vec::with_capacity(total_items);
        let mut success_count: usize = 0;
        let mut failed_count: usize = 0;
        let mut processed_count: usize = 0;
        let mut batch_number: usize = 0;

        for chunk in inputs.chunks(self.batch_size) {
            // キャンセルチェック
            if should_cancel() {
                log::info!("[{}] Processing cancelled by user", task_name);
                let event = BatchProgressEvent::cancelled(
                    task_name,
                    total_items,
                    processed_count,
                    success_count,
                    failed_count,
                );
                let _ = app_handle.emit(event_name, event);
                return Ok(BatchResult {
                    outputs,
                    success_count,
                    failed_count,
                });
            }

            batch_number += 1;

            // 2バッチ目以降はディレイを入れる（レート制限対策）
            if batch_number > 1 && self.delay_ms > 0 {
                log::debug!(
                    "[{}] Waiting {}ms before batch {}",
                    task_name,
                    self.delay_ms,
                    batch_number
                );
                sleep(Duration::from_millis(self.delay_ms)).await;
            }

            log::info!(
                "[{}] Processing batch {}: {} items",
                task_name,
                batch_number,
                chunk.len()
            );

            let batch_size = chunk.len();

            // before_batch フックを呼び出し
            if let Err(e) = self.task.before_batch(chunk, context).await {
                log::error!("[{}] before_batch failed: {}", task_name, e);
                let event = BatchProgressEvent::error(
                    task_name,
                    total_items,
                    processed_count,
                    success_count,
                    failed_count,
                    format!("before_batch エラー: {}", e),
                );
                let _ = app_handle.emit(event_name, event);
                return Err(e);
            }

            // process_batch でバッチ処理を実行
            let chunk_vec: Vec<T::Input> = chunk.to_vec();
            let batch_results = self.task.process_batch(chunk_vec, context).await;

            // 結果を集計
            let mut batch_success = 0;
            let mut batch_failed = 0;
            for result in &batch_results {
                match result {
                    Ok(_) => {
                        success_count += 1;
                        batch_success += 1;
                    }
                    Err(e) => {
                        log::warn!("[{}] Item processing failed: {}", task_name, e);
                        failed_count += 1;
                        batch_failed += 1;
                    }
                }
                processed_count += 1;
            }

            // after_batch フックを呼び出し
            if let Err(e) = self.task.after_batch(batch_number, &batch_results, context).await {
                log::error!("[{}] after_batch failed: {}", task_name, e);
                let event = BatchProgressEvent::error(
                    task_name,
                    total_items,
                    processed_count,
                    success_count,
                    failed_count,
                    format!("after_batch エラー: {}", e),
                );
                let _ = app_handle.emit(event_name, event);
                return Err(e);
            }

            // 成功した結果を outputs に追加
            for result in batch_results {
                if let Ok(output) = result {
                    outputs.push(output);
                }
            }

            // 進捗イベントを送信
            let event = BatchProgressEvent::progress(
                task_name,
                batch_number,
                batch_size,
                total_items,
                processed_count,
                success_count,
                failed_count,
                format!(
                    "バッチ {} 完了: {} 件成功, {} 件失敗",
                    batch_number, batch_success, batch_failed
                ),
            );
            let _ = app_handle.emit(event_name, event);

            log::info!(
                "[{}] Batch {} complete: {} success, {} failed",
                task_name,
                batch_number,
                batch_success,
                batch_failed
            );
        }

        // 完了イベントを送信
        let event = BatchProgressEvent::complete(
            task_name,
            total_items,
            success_count,
            failed_count,
            format!(
                "処理完了: {} 件成功, {} 件失敗",
                success_count, failed_count
            ),
        );
        let _ = app_handle.emit(event_name, event);

        log::info!(
            "[{}] Batch processing complete: {} success, {} failed",
            task_name,
            success_count,
            failed_count
        );

        Ok(BatchResult {
            outputs,
            success_count,
            failed_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // テスト用のモックタスク
    struct MockTask {
        fail_indices: Vec<usize>,
    }

    #[async_trait]
    impl BatchTask for MockTask {
        type Input = usize;
        type Output = String;
        type Context = ();

        fn name(&self) -> &str {
            "テストタスク"
        }

        fn event_name(&self) -> &str {
            "test-progress"
        }

        async fn process(&self, input: Self::Input, _ctx: &Self::Context) -> Result<Self::Output, String> {
            if self.fail_indices.contains(&input) {
                Err(format!("Failed for index {}", input))
            } else {
                Ok(format!("Result for {}", input))
            }
        }
    }

    #[test]
    fn test_batch_progress_event_progress() {
        let event = BatchProgressEvent::progress(
            "テスト",
            1,
            10,
            100,
            10,
            8,
            2,
            "テストメッセージ".to_string(),
        );
        assert_eq!(event.task_name, "テスト");
        assert_eq!(event.batch_number, 1);
        assert_eq!(event.batch_size, 10);
        assert_eq!(event.total_items, 100);
        assert_eq!(event.processed_count, 10);
        assert_eq!(event.success_count, 8);
        assert_eq!(event.failed_count, 2);
        assert!((event.progress_percent - 10.0).abs() < 0.01);
        assert!(!event.is_complete);
        assert!(event.error.is_none());
    }

    #[test]
    fn test_batch_progress_event_complete() {
        let event = BatchProgressEvent::complete("テスト", 100, 95, 5, "完了".to_string());
        assert_eq!(event.task_name, "テスト");
        assert_eq!(event.total_items, 100);
        assert_eq!(event.success_count, 95);
        assert_eq!(event.failed_count, 5);
        assert!((event.progress_percent - 100.0).abs() < 0.01);
        assert!(event.is_complete);
        assert!(event.error.is_none());
    }

    #[test]
    fn test_batch_progress_event_error() {
        let event = BatchProgressEvent::error("テスト", 100, 50, 45, 5, "エラー発生".to_string());
        assert_eq!(event.task_name, "テスト");
        assert_eq!(event.total_items, 100);
        assert_eq!(event.processed_count, 50);
        assert!((event.progress_percent - 50.0).abs() < 0.01);
        assert!(event.is_complete);
        assert_eq!(event.error, Some("エラー発生".to_string()));
    }

    #[test]
    fn test_batch_progress_event_cancelled() {
        let event = BatchProgressEvent::cancelled("テスト", 100, 30, 25, 5);
        assert_eq!(event.task_name, "テスト");
        assert!(event.is_complete);
        assert_eq!(event.error, Some("Cancelled by user".to_string()));
        assert_eq!(event.status_message, "処理がキャンセルされました");
    }

    #[test]
    fn test_batch_runner_new() {
        let task = MockTask { fail_indices: vec![] };
        let runner = BatchRunner::new(task, 10, 1000);
        assert_eq!(runner.batch_size, 10);
        assert_eq!(runner.delay_ms, 1000);
    }
}
