# メール同期高速化: メタデータ先行取得による本文取得の最適化

## 課題

現在の同期フローでは、全新規メッセージに対して `format("full")` で Gmail API を呼び出し、MIME本文を含む完全なメッセージを取得している。しかし、保存判定 (`should_save_message()`) は **from_address と subject のみ** で行われており、本文(body)は判定に使われない。

→ 条件に合わないメールの本文も取得しているため、API レスポンスサイズ・ネットワーク帯域・デコード処理が無駄になっている。

## 改善方針

**2段階フェッチ方式**: メタデータ(ヘッダーのみ)で先にフィルタリングし、条件に合うメールのみ本文を取得する。

```
現在: message_id → get_message(full) → should_save_message → DB保存
改善: message_id → get_message_metadata(metadata) → should_save_message → [合致のみ] get_message(full) → DB保存
```

## 実装計画

### Step 1: `GmailClientTrait` に `get_message_metadata` メソッドを追加

**ファイル**: `src-tauri/src/gmail_client.rs`

```rust
/// メッセージのメタデータのみ取得（From, Subject等のヘッダー情報）
/// 本文(body)は含まない軽量なレスポンスを返す
async fn get_message_metadata(&self, message_id: &str) -> Result<GmailMessage, String>;
```

- 既存の `get_message` はそのまま残す（本文取得用）
- `get_message_metadata` は `GmailMessage` を返すが `body_plain`, `body_html` は常に `None`
- `#[cfg_attr(test, automock)]` により MockGmailClientTrait にも自動追加される

### Step 2: `GmailClient` に `get_message_metadata` の実装を追加

**ファイル**: `src-tauri/src/gmail/client.rs`

- 新規メソッド `get_message_metadata` を追加
- Gmail API を `format("metadata")` + `metadata_headers=["From", "Subject"]` で呼び出し
- ヘッダーから `from_address`, `subject` を抽出
- `snippet`, `internal_date` も取得可能
- `body_plain`, `body_html` は `None` を設定
- 既存の `get_message` (full版) はそのまま維持

### Step 3: `GmailClientTrait for GmailClient` の impl に追加

**ファイル**: `src-tauri/src/gmail/client.rs` (720行付近)

- `GmailClientTrait` の `get_message_metadata` を実装し、Step 2 の固有メソッドに委譲

### Step 4: `GmailSyncTask::process_batch` を2段階フェッチに変更

**ファイル**: `src-tauri/src/gmail/gmail_sync_task.rs`

現在の `process_batch`:

```
for input in inputs:
    get_message(id)  // full
```

変更後:

```
// Phase 1: メタデータ取得 + フィルタリング
let shop_settings = context.shop_settings_cache  // before_batch でロード済み
for input in inputs:
    metadata = get_message_metadata(id)
    if should_save_message(metadata, shop_settings):
        candidates.push(id)
    else:
        results.push(Ok(GmailSyncOutput { message: metadata, saved: false, filtered: true }))

// Phase 2: 候補のみ full 取得
for id in candidates:
    message = get_message(id)  // full（本文あり）
    results.push(Ok(GmailSyncOutput { message, saved: false, filtered: false }))
```

- `GmailSyncOutput` に `filtered: bool` フィールドを追加（メタデータのみでフィルタ除外されたことを示す）
- `after_batch` では `filtered == false` のメッセージのみ DB 保存対象にする

### Step 5: `GmailSyncOutput` にフィールド追加

**ファイル**: `src-tauri/src/gmail/gmail_sync_task.rs`

```rust
pub struct GmailSyncOutput {
    pub message: GmailMessage,
    pub saved: bool,
    pub filtered_out: bool,  // 追加: メタデータ段階でフィルタ除外された
}
```

### Step 6: `after_batch` の更新

**ファイル**: `src-tauri/src/gmail/gmail_sync_task.rs`

- `filtered_out == true` のメッセージは DB 保存処理をスキップ
- ログにメタデータフィルタで除外された件数を追記

### Step 7: E2E モック対応

**ファイル**: `src-tauri/src/e2e_mocks.rs`

- `E2EMockGmailClient` に `get_message_metadata` を実装（`get_message` と同様にエラーを返す）
- `GmailClientForE2E` の `GmailClientTrait` impl に `get_message_metadata` のデリゲーションを追加

### Step 8: テスト更新

以下のテストを追加・更新:

1. **`gmail_client.rs` テスト**: `MockGmailClientTrait` の `get_message_metadata` テスト
2. **`gmail_sync_task.rs` テスト**: `process_batch` が2段階で動作することのテスト
   - メタデータ段階でフィルタされたメッセージは `get_message` が呼ばれないこと
   - 条件に合うメッセージのみ `get_message` (full) が呼ばれること
3. **`e2e_mocks.rs` テスト**: 新メソッドのデリゲーションテスト
4. **`sync_logic.rs` テスト**: 変更なし（`should_save_message` のインターフェースは変わらない）

## 変更ファイル一覧

| ファイル                                 | 変更内容                                                  |
| ---------------------------------------- | --------------------------------------------------------- |
| `src-tauri/src/gmail_client.rs`          | `get_message_metadata` メソッドをトレイトに追加           |
| `src-tauri/src/gmail/client.rs`          | `get_message_metadata` の実装 + トレイト impl             |
| `src-tauri/src/gmail/gmail_sync_task.rs` | `process_batch` を2段階方式に変更、`GmailSyncOutput` 更新 |
| `src-tauri/src/e2e_mocks.rs`             | 新メソッドのモック実装 + デリゲーション                   |
| `src-tauri/src/batch_commands.rs`        | 変更なし                                                  |
| `src-tauri/src/logic/sync_logic.rs`      | 変更なし                                                  |
| `src-tauri/src/batch_runner.rs`          | 変更なし                                                  |

## 期待される効果

- **API レスポンスサイズの大幅削減**: `format("metadata")` はヘッダーのみで、`format("full")` の数分の1〜数十分の1
- **処理時間短縮**: 条件に合わないメールの MIME パース・文字エンコーディング変換がなくなる
- **ネットワーク帯域節約**: 不要な本文データの転送を回避
- **条件に合わないメールが多いほど効果大**: フィルタ除外率が高いケースで顕著な高速化

## リスクと考慮事項

- メタデータ取得 + full 取得の2回 API コールが必要（条件に合うメールのみ）
  → 条件に合わないメールが多い場合は全体として削減、少ない場合は微増
- `format("metadata")` でもヘッダー情報は `format("full")` と同一なので、フィルタリング精度に影響なし
