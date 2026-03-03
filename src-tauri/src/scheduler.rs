//! 定期パイプライン実行スケジューラ
//!
//! アプリ常駐時にバックグラウンドで「差分同期 → メールパース → 商品名解析 → 配達状況確認」の
//! パイプラインを一定間隔で自動実行する。
//!
//! - `tokio::time::sleep` ベースの非同期ループ
//! - `SchedulerState` で有効/無効をトレイメニューからトグル可能
//! - パイプライン実行中は次の tick をスキップ（多重実行防止）
//! - 既存の `shutdown_signal` を共有し、quit 時にループを終了

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Notify;

/// スケジューラの状態イベント（フロントエンドへの通知用）
pub const SCHEDULER_STATUS_EVENT: &str = "scheduler-status-changed";
pub const SCHEDULER_PIPELINE_STARTED_EVENT: &str = "scheduler-pipeline-started";
pub const SCHEDULER_PIPELINE_COMPLETED_EVENT: &str = "scheduler-pipeline-completed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerStatusPayload {
    pub enabled: bool,
    pub interval_minutes: i64,
}

#[derive(Clone)]
pub struct SchedulerState {
    enabled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    interval_minutes: Arc<AtomicI64>,
}

impl SchedulerState {
    pub fn new(enabled: bool, interval_minutes: i64) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(enabled)),
            running: Arc::new(AtomicBool::new(false)),
            interval_minutes: Arc::new(AtomicI64::new(interval_minutes)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, val: bool) {
        self.enabled.store(val, Ordering::SeqCst);
    }

    pub fn toggle(&self) -> bool {
        let prev = self.enabled.fetch_xor(true, Ordering::SeqCst);
        !prev
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_running(&self, val: bool) {
        self.running.store(val, Ordering::SeqCst);
    }

    pub fn interval_minutes(&self) -> i64 {
        self.interval_minutes.load(Ordering::SeqCst)
    }

    pub fn set_interval_minutes(&self, val: i64) {
        self.interval_minutes.store(val, Ordering::SeqCst);
    }
}

/// 分数を人間可読な間隔表記に変換する（トレイメニュー表示用）。
pub fn format_interval(minutes: i64) -> String {
    if minutes >= 1440 && minutes % 1440 == 0 {
        let days = minutes / 1440;
        if days == 1 {
            "1日".to_string()
        } else {
            format!("{}日", days)
        }
    } else if minutes >= 60 && minutes % 60 == 0 {
        let hours = minutes / 60;
        format!("{}時間", hours)
    } else {
        format!("{}分", minutes)
    }
}

/// インターバルが経過するか、シャットダウンシグナルを受けるまで待機する。
///
/// 正常にインターバルが経過した場合は `true` を返す。
/// シャットダウンが検出された場合は `false` を返す。
/// `Notify::notified()` で待機するため、ポーリングと異なり通知を受けた瞬間に即時復帰する。
async fn wait_for_interval_or_shutdown(duration: Duration, shutdown: &Arc<Notify>) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => true,
        _ = shutdown.notified() => false,
    }
}

/// スケジューラのメインループ。`setup()` から `tauri::async_runtime::spawn` で起動する。
///
/// `tokio::time::sleep` ベースで毎 tick ごとに最新の `interval_minutes` を参照するため、
/// 設定画面やコマンドから間隔を変更すると再起動なしで次の tick から反映される。
pub async fn run_scheduler(
    app: tauri::AppHandle,
    state: SchedulerState,
    shutdown: Arc<Notify>,
) {
    log::info!(
        "[Scheduler] Started: interval={}min, enabled={}",
        state.interval_minutes(),
        state.is_enabled()
    );

    loop {
        let interval_min = state.interval_minutes().max(1);
        if !wait_for_interval_or_shutdown(
            Duration::from_secs(interval_min as u64 * 60),
            &shutdown,
        )
        .await
        {
            log::info!("[Scheduler] Shutdown signal received, exiting");
            break;
        }

        if !state.is_enabled() {
            log::debug!("[Scheduler] Disabled, skipping tick");
            continue;
        }

        if state.is_running() {
            log::info!("[Scheduler] Previous pipeline still running, skipping tick");
            continue;
        }

        state.set_running(true);

        let _ = app.emit(SCHEDULER_PIPELINE_STARTED_EVENT, ());

        log::info!("[Scheduler] Pipeline starting");
        crate::orchestration::run_pipeline(&app).await;
        log::info!("[Scheduler] Pipeline completed");

        let _ = app.emit(SCHEDULER_PIPELINE_COMPLETED_EVENT, ());

        state.set_running(false);
    }

    log::info!("[Scheduler] Loop exited");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_initial_state() {
        let state = SchedulerState::new(true, 30);
        assert!(state.is_enabled());
        assert!(!state.is_running());
        assert_eq!(state.interval_minutes(), 30);
    }

    #[test]
    fn new_disabled() {
        let state = SchedulerState::new(false, 15);
        assert!(!state.is_enabled());
        assert_eq!(state.interval_minutes(), 15);
    }

    #[test]
    fn set_enabled_toggles() {
        let state = SchedulerState::new(true, 30);
        state.set_enabled(false);
        assert!(!state.is_enabled());
        state.set_enabled(true);
        assert!(state.is_enabled());
    }

    #[test]
    fn toggle_flips_and_returns_new_value() {
        let state = SchedulerState::new(true, 30);
        let new_val = state.toggle();
        assert!(!new_val);
        assert!(!state.is_enabled());

        let new_val = state.toggle();
        assert!(new_val);
        assert!(state.is_enabled());
    }

    #[test]
    fn running_flag() {
        let state = SchedulerState::new(true, 30);
        assert!(!state.is_running());
        state.set_running(true);
        assert!(state.is_running());
        state.set_running(false);
        assert!(!state.is_running());
    }

    #[test]
    fn format_interval_days() {
        assert_eq!(format_interval(1440), "1日");
        assert_eq!(format_interval(2880), "2日");
    }

    #[test]
    fn format_interval_hours() {
        assert_eq!(format_interval(60), "1時間");
        assert_eq!(format_interval(360), "6時間");
    }

    #[test]
    fn format_interval_minutes() {
        assert_eq!(format_interval(30), "30分");
        assert_eq!(format_interval(90), "90分");
        assert_eq!(format_interval(1), "1分");
    }

    #[test]
    fn set_interval_minutes_updates_value() {
        let state = SchedulerState::new(true, 30);
        assert_eq!(state.interval_minutes(), 30);
        state.set_interval_minutes(1440);
        assert_eq!(state.interval_minutes(), 1440);
    }

    #[test]
    fn clone_shares_state() {
        let state = SchedulerState::new(true, 30);
        let clone = state.clone();

        state.set_enabled(false);
        assert!(!clone.is_enabled());

        clone.set_running(true);
        assert!(state.is_running());

        state.set_interval_minutes(60);
        assert_eq!(clone.interval_minutes(), 60);
    }
}
