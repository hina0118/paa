-- 進捗テーブルを削除
-- 状態・進捗はメモリ（SyncState, ParseState）で管理し、設定は paa_config.json で管理する

DROP TABLE IF EXISTS sync_metadata;
DROP TABLE IF EXISTS parse_metadata;
