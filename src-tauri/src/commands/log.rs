use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// ログバッファ用グローバルMutex
//
// 注意: グローバルMutexの使用はロック競合のリスクがあります。
// 現在の実装では適切なエラーハンドリングにより安全性を確保していますが、
// 将来的にはTauriのステート管理機能への移行を検討してください。
//
// パフォーマンスに関する考慮事項:
// - ログ記録の度にMutexロックを取得しますが、ロック保持時間は短く抑えられています
// - MAX_LOG_ENTRIESを超えた古いログは自動的に削除され、メモリ使用量を制限しています
// - 通常のアプリケーション使用では十分なパフォーマンスを提供します
static LOG_BUFFER: Mutex<Option<VecDeque<LogEntry>>> = Mutex::new(None);
pub(crate) const MAX_LOG_ENTRIES: usize = 1000;

/// ログバッファを初期化
///
/// アプリケーション起動時に一度だけ呼び出してください。
/// 複数回呼び出しても安全ですが、既存のログは破棄されます。
pub fn init_log_buffer() {
    match LOG_BUFFER.lock() {
        Ok(mut buffer) => {
            *buffer = Some(VecDeque::with_capacity(MAX_LOG_ENTRIES));
        }
        Err(e) => {
            eprintln!("Failed to initialize log buffer: {e}");
            // ログバッファの初期化に失敗してもアプリケーションは継続
            // ログ機能は利用できないが、クラッシュは回避
        }
    }
}

/// ログエントリを追加
///
/// # パラメータ
/// - `level`: ログレベル（例: "INFO", "ERROR", "DEBUG"）
/// - `message`: ログメッセージ
///
/// # パフォーマンス
/// この関数はログ記録の度にMutexロックを取得しますが、
/// ロック保持時間は最小限（数マイクロ秒）に抑えられています。
/// 通常のログ記録頻度では問題になりません。
pub fn add_log_entry(level: &str, message: &str) {
    match LOG_BUFFER.lock() {
        Ok(mut buffer) => {
            if let Some(ref mut logs) = *buffer {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now()
                        .with_timezone(&chrono_tz::Asia::Tokyo)
                        .format("%Y-%m-%d %H:%M:%S%.3f")
                        .to_string(),
                    level: level.to_string(),
                    message: message.to_string(),
                };

                logs.push_back(entry);

                if logs.len() > MAX_LOG_ENTRIES {
                    logs.pop_front();
                }
            }
            // ログバッファが未初期化の場合は静かに無視
            // アプリケーション起動時の初期化前に呼ばれる可能性がある
        }
        Err(e) => {
            // ロック取得失敗時は標準エラー出力に出力
            // ログシステム自体が問題を抱えているため、通常のログ機能は使えない
            eprintln!("Failed to lock log buffer for adding entry: {e}");
        }
    }
}

/// ログエントリを取得
///
/// # パラメータ
/// - `level_filter`: ログレベルでフィルタリング（例: "ERROR", "INFO"）。Noneの場合は全てのレベルを返す
/// - `limit`: 返却する最大件数。フィルタリング後のログに対して適用される
///
/// # 戻り値
/// 新しい順（最新が先頭）でログエントリのリストを返す
///
/// # 注意
/// limitパラメータはフィルタリング後のログに適用されます。
/// 例：limit=100, `level_filter="ERROR"の場合、ERRORログから最大100件を返します`。
#[tauri::command]
pub fn get_logs(
    level_filter: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<LogEntry>, String> {
    let buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {e}"))?;

    if let Some(ref logs) = *buffer {
        let mut filtered_logs: Vec<LogEntry> = logs
            .iter()
            .filter(|entry| {
                if let Some(ref filter) = level_filter {
                    &entry.level == filter
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        filtered_logs.reverse();

        if let Some(limit) = limit {
            filtered_logs.truncate(limit);
        }

        Ok(filtered_logs)
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_initialization() {
        init_log_buffer();
        add_log_entry("INFO", "Test message");
        let logs = get_logs(None, None);
        assert!(logs.is_ok());
    }

    #[test]
    fn test_log_buffer_multiple_initialization() {
        init_log_buffer();
        init_log_buffer();
        init_log_buffer();

        add_log_entry("INFO", "Test after multiple init");
        let logs = get_logs(None, None);
        assert!(logs.is_ok());
    }

    #[test]
    fn test_add_log_entry_safe() {
        add_log_entry("DEBUG", "Safe logging test");
        add_log_entry("INFO", "Another safe log");
        add_log_entry("ERROR", "Error log test");
    }

    #[test]
    fn test_log_buffer_max_entries() {
        init_log_buffer();

        for i in 0..(MAX_LOG_ENTRIES + 100) {
            add_log_entry("INFO", &format!("Log entry {i}"));
        }

        let logs = get_logs(None, None).unwrap();
        assert!(logs.len() <= MAX_LOG_ENTRIES);
    }

    #[test]
    fn test_get_logs_with_filter() {
        init_log_buffer();

        add_log_entry("INFO", "Info message");
        add_log_entry("ERROR", "Error message");
        add_log_entry("DEBUG", "Debug message");

        let error_logs = get_logs(Some("ERROR".to_string()), None).unwrap();
        assert!(error_logs.iter().all(|log| log.level == "ERROR"));
    }

    #[test]
    fn test_get_logs_with_limit() {
        init_log_buffer();

        for i in 0..10 {
            add_log_entry("LIMIT_TEST", &format!("Message {i}"));
        }

        let logs = get_logs(Some("LIMIT_TEST".to_string()), Some(5)).unwrap();
        assert!(
            logs.len() <= 5,
            "limit should restrict results to at most 5 entries"
        );
        assert!(logs.iter().all(|log| log.level == "LIMIT_TEST"));
    }
}
