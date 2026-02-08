# PR #78 対応計画

## PR 概要

**タイトル**: feat: 画像検索API失敗時にサブウィンドウでGoogle画像検索を開けるように  
**ブランチ**: `image-search-sub-window` → `main`  
**状態**: Open  
**CI**: mergeable_state: unstable

**変更内容**（2つの機能を1つのPRに含む）:

1. **画像検索フォールバック（メイン機能）**
   - API失敗時・検索結果0件時にサブウィンドウでGoogle画像検索を開ける
   - URL手動入力欄を追加
   - WebviewWindow 作成権限を追加

2. **バックアップ・復元に emails テーブルを追加**
   - エクスポート: ストリーミング + NDJSON 形式
   - インポート: emails.ndjson（新形式）および emails.json（レガシー互換）

**変更ファイル**:

- `src-tauri/capabilities/default.json`
- `src/components/orders/image-search-dialog.tsx`
- `src/components/orders/image-search-dialog.test.tsx`
- `src-tauri/src/metadata_export.rs`
- `src/components/screens/backup.tsx`
- `docs/BACKUP.md`

---

## レビューコメント一覧

### resolved（対応済み）

| #   | 優先度 | 指摘                                                                                 | 状態                                                                                   |
| --- | ------ | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| 1   | P1     | 検索結果0件時も `apiSearchFailed=true` の不整合（API成功でも「利用できません」表示） | Resolved                                                                               |
| 2   | P2     | WebviewWindow label の `Date.now()` のみで衝突リスク                                 | Resolved                                                                               |
| 3   | P0     | emails.json の `read_zip_entry` 未使用・サイズチェック・エラー処理                   | Resolved（※現行は stream_deserialize + read_zip_entry_optional_with_limit で対応済み） |
| 4   | P1     | PRタイトル/説明にバックアップ（emails）の記載なし                                    | Resolved                                                                               |
| 5   | P1     | urlToSave の優先順位（手動入力 vs 選択URL）                                          | Resolved                                                                               |
| 6   | P1     | 0件時の文言分岐                                                                      | Resolved                                                                               |

### 未対応（現行ブランチへの指摘）

| #   | 優先度 | 指摘                                         | 行      | 概要                                                                                                                                |
| --- | ------ | -------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 7   | P0     | emails.ndjson の OOM                         | 437–445 | `.lines().collect::<Vec<_>>()` で全行をメモリに載せるため、バックアップサイズ次第で OOM                                             |
| 8   | P0     | 長大行のメモリ確保                           | 441–456 | `BufRead::lines()` は改行まで 1 行丸ごと `String` に確保してから返すため、`MAX_NDJSON_LINE_SIZE` チェック前に大きなメモリ確保が発生 |
| 9   | P0     | stream_deserialize_json_array のバッファ流失 | 641     | `Deserializer::from_reader` を都度作り直すと、内部バッファの先読み分が失われてパースが壊れる可能性                                  |

---

## 対応方針

### 方針A: 段階的対応（推奨）

**Phase 1（即マージ可能な範囲）**: 画像検索フォールバックのみ

- 画像検索フォールバック機能 + 関連権限・テストはそのまま
- **emails バックアップ・復元は別PRに分離**し、本PRから削除

→ レビュー指摘の多くが metadata_export.rs の emails 周りに集中しているため、分離すると対応が容易になる。

**Phase 2（別PR）**: emails バックアップ・復元の改善

- P0 指摘をすべて解消してから別PRでマージ

### 方針B: 本PRで一括対応

- P0 指摘3件をすべて metadata_export.rs で修正
- 工数・リスクは大きいが、1PRで完結

---

## 実施タスク（方針B の場合）

### metadata_export.rs の修正

| #   | 優先度 | タスク                                                                                    | 対象          |
| --- | ------ | ----------------------------------------------------------------------------------------- | ------------- |
| 1   | P0     | emails.ndjson の読み込みを `read_until(b'\n')` で行単位にし、サイズ上限を確保前にチェック | 437–479行付近 |
| 2   | P0     | 全行を `Vec` に保持せず、1行ずつパース・INSERT するループに変更（OOM 回避）               | 同上          |
| 3   | P0     | `stream_deserialize_json_array` を `serde_json::from_reader::<_, Vec<T>>` に置き換え      | 581–641行     |

### 指摘7・8 の修正案（emails.ndjson インポート）

```rust
// 現在
let lines: Vec<String> = {
    let mut entry = zip_archive.by_name("emails.ndjson")?;
    BufReader::new(&mut entry)
        .lines()
        .collect::<Result<Vec<_>, _>>()?
};

// 修正後: read_until で行単位に読み、サイズ上限を確保前にチェックし、1行ずつ INSERT
let mut entry = zip_archive
    .by_name("emails.ndjson")
    .map_err(|e| format!("Failed to access emails.ndjson: {e}"))?;
let mut reader = BufReader::new(&mut entry);
let mut buf: Vec<u8> = Vec::with_capacity(4096);

loop {
    buf.clear();
    let bytes_read = reader
        .read_until(b'\n', &mut buf)
        .map_err(|e| format!("Failed to read emails.ndjson: {e}"))?;
    if bytes_read == 0 {
        break;
    }
    if buf.len() > MAX_NDJSON_LINE_SIZE + 1 {
        return Err(format!(
            "emails.ndjson line exceeds size limit (max {} bytes)",
            MAX_NDJSON_LINE_SIZE
        ));
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    if buf.is_empty() {
        continue;
    }
    let line = String::from_utf8(buf.clone())
        .map_err(|e| format!("Failed to decode as UTF-8: {e}"))?;
    let line = line.trim();
    if line.is_empty() {
        continue;
    }
    let row: JsonEmailRow = serde_json::from_str(line)
        .map_err(|e| format!("Failed to parse line: {e}"))?;
    // INSERT ...
    emails_inserted += ...;
}
```

**ポイント**:

- `read_until` で上限チェック後に `String` 化
- ループ内で1行ずつパース・INSERT し、全行を保持しない

### 指摘9 の修正案（emails.json レガシー）

```rust
// 現在: stream_deserialize_json_array（Deserializer 都度破棄、バッファ流失リスク）
let emails_rows: Vec<JsonEmailRow> = stream_deserialize_json_array(BufReader::new(&mut entry))?;

// 修正後: 1つの Deserializer で全体を読み取る
let emails_rows: Vec<JsonEmailRow> = serde_json::from_reader(BufReader::new(&mut entry))
    .map_err(|e| format!("Failed to parse emails.json: {e}"))?;
```

**注意**: `emails.json` は `MAX_EMAILS_JSON_ENTRY_SIZE`（50MB）でサイズ制限済み。`from_reader` で一括読み取りは許容範囲。

---

## 実行フロー

### 方針A（フォールバックのみ・推奨）

1. emails 関連の変更を本PRから revert（metadata_export.rs, backup.tsx, BACKUP.md）
2. PR 説明を画像検索フォールバックのみに更新
3. CI 通過後マージ
4. emails バックアップは別PRで P0 解消後に実装

### 方針B（一括対応）

1. 上記 metadata_export.rs の修正3件を適用
2. 既存テスト実行: `cargo test -p tauri-app metadata_export`
3. コミット・プッシュ
4. CI 通過を確認
5. レビュー再依頼

---

## 備考

- PR 説明には既に「2. バックアップ・復元に emails テーブルを追加」が含まれており、両機能の記載はある
- `docs/webview-cookie-sharing.md` は本PRとは無関係（WebView Cookie 共有の調査資料）
- 画像検索フォールバックの WebviewWindow は Google 画像検索を開くのみで、Cookie shared は不要
