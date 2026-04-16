-- news_clips テーブルに AI 抽出イベント日付カラムを追加
-- events: JSON 配列 [{"date":"YYYY-MM-DD","label":"イベント説明"}, ...]
ALTER TABLE news_clips ADD COLUMN events TEXT NOT NULL DEFAULT '[]';
