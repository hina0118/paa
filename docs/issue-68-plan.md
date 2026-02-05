# Issue #68 対応計画

**Issue**: [#68 PR 67 review follow-up](https://github.com/hina0118/paa/issues/68)  
**作成日**: 2026-02-05  
**関連**: PR #67, Issue #54, #65, #66

---

## 概要

PR #67 のレビューで指摘された項目のうち、今回の PR では対応しない方針としたものを記録する issue です。  
以下の2つの項目に対応します：

1. **P1: Important** - `get_existing_message_ids()` のメモリ使用量の最適化
2. **P2: Nitpick** - `sync_gmail_incremental_with_client` の扱い（deprecated の明示または移行）

---

## P1: Important - `get_existing_message_ids()` のメモリ使用量最適化

### 現状分析

**ファイル**: `src-tauri/src/repository.rs` (L1022-1029)

**問題点**:

- `get_existing_message_ids()` がすべての `message_id` をメモリに読み込む
- 数万件のメールを持つユーザーでは大量のメモリを消費する可能性がある
- `lib.rs` の `start_sync` (L208) で `HashSet<String>` として使用されている

**現在の実装**:

```rust
async fn get_existing_message_ids(&self) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT message_id FROM emails")
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get existing message IDs: {e}"))?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}
```

**使用箇所**: `src-tauri/src/lib.rs` (L208-224)

```rust
let existing_ids: HashSet<String> = match email_repo.get_existing_message_ids().await {
    Ok(ids) => ids.into_iter().collect(),
    // ...
};
```

### 対応方針

**オプション1: SQL で NOT IN を使用（推奨）**

Gmail API から取得した `all_ids` に対して、SQL で直接フィルタリングする方法。

**メリット**:

- メモリ使用量を大幅に削減（既存IDをメモリに読み込まない）
- パフォーマンス向上（DB側でフィルタリング）
- 実装が比較的シンプル

**デメリット**:

- SQL の `NOT IN` は大量の値に対してはパフォーマンスが低下する可能性がある
- SQLite の `NOT IN` は NULL 値の扱いに注意が必要

**実装案**:

```rust
// EmailRepository に新しいメソッドを追加
async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String>;

// 実装（SqliteEmailRepository）
async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String> {
    if message_ids.is_empty() {
        return Ok(Vec::new());
    }

    // SQLite の制限（SQLITE_MAX_VARIABLE_NUMBER = 999）を考慮してチャンク処理
    const CHUNK_SIZE: usize = 900; // 安全マージン
    let mut new_ids = Vec::new();

    for chunk in message_ids.chunks(CHUNK_SIZE) {
        let placeholders = (0..chunk.len()).map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT ? WHERE ? NOT IN (SELECT message_id FROM emails WHERE message_id IN ({placeholders}))",
            placeholders = placeholders
        );

        // 実際には、各IDを個別にチェックする方がシンプル
        // または、一時テーブルを使用する方法も検討可能
    }

    // より実用的な実装: 一時テーブルを使用
    // 1. 一時テーブルに message_ids を INSERT
    // 2. NOT IN でフィルタリング
    // 3. 一時テーブルを DROP
}
```

**オプション2: ストリーミング処理**

`sqlx` の `fetch()` を使用して、メモリに一度に読み込まずに処理する方法。

**メリット**:

- メモリ使用量を一定に保てる
- 既存の `HashSet` ベースの実装を維持できる

**デメリット**:

- 実装がやや複雑
- パフォーマンスはオプション1より劣る可能性がある

**実装案**:

```rust
async fn get_existing_message_ids_streaming(&self) -> Result<HashSet<String>, String> {
    use futures::TryStreamExt;

    let mut rows = sqlx::query_as::<_, (String,)>("SELECT message_id FROM emails")
        .fetch(&self.pool);

    let mut existing_ids = HashSet::new();
    while let Some(row) = rows.try_next().await
        .map_err(|e| format!("Failed to stream message IDs: {e}"))? {
        existing_ids.insert(row.0);
    }

    Ok(existing_ids)
}
```

**オプション3: 一時テーブルを使用（最適化）**

大量の `message_ids` に対して効率的にフィルタリングする方法。

**実装案**:

```rust
async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String> {
    if message_ids.is_empty() {
        return Ok(Vec::new());
    }

    // 一時テーブルを作成
    sqlx::query("CREATE TEMP TABLE IF NOT EXISTS temp_message_ids (message_id TEXT PRIMARY KEY)")
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create temp table: {e}"))?;

    // 既存データをクリア
    sqlx::query("DELETE FROM temp_message_ids")
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to clear temp table: {e}"))?;

    // チャンクごとに INSERT（SQLite の制限を考慮）
    const CHUNK_SIZE: usize = 900;
    for chunk in message_ids.chunks(CHUNK_SIZE) {
        let mut query_builder = sqlx::QueryBuilder::new("INSERT INTO temp_message_ids (message_id) VALUES ");
        for (i, id) in chunk.iter().enumerate() {
            if i > 0 {
                query_builder.push(", ");
            }
            query_builder.push("(").push_bind(id).push(")");
        }
        let query = query_builder.build();
        query.execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to insert into temp table: {e}"))?;
    }

    // NOT IN でフィルタリング
    let new_ids: Vec<(String,)> = sqlx::query_as(
        "SELECT message_id FROM temp_message_ids WHERE message_id NOT IN (SELECT message_id FROM emails)"
    )
    .fetch_all(&self.pool)
    .await
    .map_err(|e| format!("Failed to filter new IDs: {e}"))?;

    Ok(new_ids.into_iter().map(|(id,)| id).collect())
}
```

### 推奨実装

**オプション3（一時テーブル）を推奨**:

- 大量の `message_ids` に対して効率的
- SQLite の制限を考慮した実装が可能
- メモリ使用量を最小限に抑えられる

### 実装手順

1. **EmailRepository トレイトに新しいメソッドを追加**
   - `filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String>`

2. **SqliteEmailRepository に実装**
   - 一時テーブルを使用した実装
   - エラーハンドリングとクリーンアップ処理

3. **lib.rs の start_sync を更新**
   - `get_existing_message_ids()` の呼び出しを削除
   - `filter_new_message_ids()` を使用するように変更

4. **テスト追加**
   - 大量のメッセージIDでのテスト
   - エッジケース（空配列、既存IDのみ、新規IDのみ）のテスト

5. **既存の `get_existing_message_ids()` の扱い**
   - 他の使用箇所がないか確認
   - なければ削除、あれば deprecated マークを付与

---

## P2: Nitpick - `sync_gmail_incremental_with_client` の扱い

### 現状分析

**ファイル**: `src-tauri/src/gmail/client.rs` (L856-885, L896)

**問題点**:

- `sync_gmail_incremental_with_client` は `BatchRunner<GmailSyncTask>` に置き換えられた
- しかし、`fetch_gmail_emails` コマンド (`lib.rs` L457) で `sync_gmail_incremental` が使用されており、それが内部で `sync_gmail_incremental_with_client` を呼び出している
- `sync_gmail_incremental` と `sync_gmail_incremental_with_client` は古い実装として残っている
- 非推奨であることを明示するか、`BatchRunner` に移行する必要がある

**使用箇所**:

- `src-tauri/src/lib.rs` (L448-465): `fetch_gmail_emails` コマンドが `sync_gmail_incremental` を呼び出し
- `src-tauri/src/gmail/client.rs` (L876): `sync_gmail_incremental` が `sync_gmail_incremental_with_client` を呼び出し

**関数の関係**:

```rust
// lib.rs
fetch_gmail_emails()
  → sync_gmail_incremental()  // 古い実装
    → sync_gmail_incremental_with_client()  // 古い実装（テスト可能なバージョン）

// lib.rs (新しい実装)
start_sync()
  → BatchRunner<GmailSyncTask>  // 新しい実装
```

**確認結果**:

- `sync_gmail_incremental_with_client` は `sync_gmail_incremental` の内部実装として使用されている
- これはテスト可能にするための設計（トレイトベース）だが、`BatchRunner` に移行されたため、両方とも deprecated にする必要がある

### 対応方針

**オプション1: `sync_gmail_incremental` と `sync_gmail_incremental_with_client` を deprecated にする（推奨）**

`fetch_gmail_emails` コマンドがフロントエンドで使用されているかどうかを確認し、使用されていれば deprecated マークを付与。使用されていなければ `BatchRunner` に移行。

**実装案**:

```rust
#[deprecated(
    since = "0.1.0",
    note = "Use BatchRunner<GmailSyncTask> via start_sync command instead. This function is kept for backward compatibility only."
)]
pub async fn sync_gmail_incremental(
    // ...
) -> Result<(), String> {
    // 既存の実装を維持
}

#[deprecated(
    since = "0.1.0",
    note = "Use BatchRunner<GmailSyncTask> instead. This function is kept for backward compatibility only."
)]
pub async fn sync_gmail_incremental_with_client(
    // ...
) -> Result<(), String> {
    // 既存の実装を維持
}
```

**オプション2: `fetch_gmail_emails` を `BatchRunner` に移行**

`fetch_gmail_emails` コマンドを `start_sync` と同様に `BatchRunner<GmailSyncTask>` を使用するように変更し、古い実装を削除。

**メリット**:

- コードの一貫性が向上
- 古い実装を完全に削除できる
- メンテナンスコストが削減される

**デメリット**:

- `fetch_gmail_emails` の使用箇所を確認する必要がある
- フロントエンドへの影響を確認する必要がある

### 推奨実装

**オプション1（deprecated マーク）を推奨**:

- `fetch_gmail_emails` がフロントエンドで使用されているかどうかを確認
- 使用されていれば deprecated マークを付与して後方互換性を維持
- 使用されていなければ `BatchRunner` に移行して古い実装を削除
- 将来的に完全に削除することを検討

### 実装手順

1. **使用箇所の確認** ✅ 完了
   - `sync_gmail_incremental` と `sync_gmail_incremental_with_client` の使用箇所を grep で確認（完了: `lib.rs` L457, `client.rs` L876）
   - `fetch_gmail_emails` コマンドが実際にフロントエンドで使用されているか確認（完了: 使用されていない）
     - フロントエンドでは `start_sync` コマンドを使用（`sync-provider.tsx` L79）
     - `fetch_gmail_emails` は未使用のため、将来的に削除可能

2. **deprecated マークの付与**
   - `sync_gmail_incremental` に `#[deprecated]` 属性を追加
   - `sync_gmail_incremental_with_client` に `#[deprecated]` 属性を追加
   - ドキュメントコメントに移行先（`BatchRunner<GmailSyncTask>` または `start_sync` コマンド）を明記

3. **`fetch_gmail_emails` コマンドの扱い（オプション）**
   - `fetch_gmail_emails` が未使用であるため、deprecated マークを付与するか、直接削除を検討
   - 削除する場合は、`lib.rs` の `setup` 関数から登録を削除

4. **テストの確認**
   - 既存のテストが動作することを確認
   - deprecated 警告が出ることを確認（コンパイル時に警告が表示される）

---

## 実装順序

### Phase 1: P1 の実装（メモリ最適化）

1. ✅ Issue 68 の確認と計画作成
2. ⏳ `EmailRepository` に `filter_new_message_ids()` メソッドを追加
3. ⏳ `SqliteEmailRepository` に実装（一時テーブル方式）
4. ⏳ `lib.rs` の `start_sync` を更新
5. ⏳ テスト追加・既存テストの確認
6. ⏳ `get_existing_message_ids()` の使用箇所確認と削除/deprecated 化

### Phase 2: P2 の実装（deprecated マーク）

1. ⏳ `sync_gmail_incremental_with_client` の使用箇所確認
2. ⏳ `fetch_gmail_emails` コマンドの使用状況確認
3. ⏳ `sync_gmail_incremental_with_client` に deprecated マークを付与
4. ⏳ テスト確認

---

## 完了条件

### P1: メモリ最適化

- [ ] `filter_new_message_ids()` メソッドが実装されている
- [ ] `start_sync` が新しいメソッドを使用している
- [ ] 大量のメッセージID（10,000件以上）でのテストが成功している
- [ ] メモリ使用量が削減されていることを確認（プロファイリングまたはログ）
- [ ] 既存のテストが全て通る
- [ ] `get_existing_message_ids()` が削除または deprecated になっている

### P2: deprecated マーク

- [x] `sync_gmail_incremental` と `sync_gmail_incremental_with_client` の使用箇所確認（完了）
- [x] `fetch_gmail_emails` の使用状況確認（完了: 未使用）
- [ ] `sync_gmail_incremental` に `#[deprecated]` 属性が付与されている
- [ ] `sync_gmail_incremental_with_client` に `#[deprecated]` 属性が付与されている
- [ ] ドキュメントコメントに移行先（`BatchRunner<GmailSyncTask>` または `start_sync` コマンド）が明記されている
- [ ] `fetch_gmail_emails` コマンドに deprecated マークを付与または削除（未使用のため）
- [ ] 既存のテストが動作することを確認

---

## 注意事項

1. **後方互換性**
   - `fetch_gmail_emails` コマンドがフロントエンドで使用されている場合は、動作を維持する必要がある

2. **パフォーマンステスト**
   - P1 の実装後、実際のデータ量でのパフォーマンステストを実施
   - メモリ使用量の測定を実施

3. **SQLite の制限**
   - SQLite の `SQLITE_MAX_VARIABLE_NUMBER` (デフォルト 999) を考慮
   - チャンク処理を実装する必要がある

4. **エラーハンドリング**
   - 一時テーブルの作成・削除時のエラーハンドリング
   - トランザクションの適切な管理

---

## 見積もり

- **Phase 1 (P1)**: 約2-3時間
  - メソッド追加・実装: 1時間
  - `lib.rs` 更新: 30分
  - テスト追加・確認: 1時間
  - プロファイリング・検証: 30分

- **Phase 2 (P2)**: 約30-45分
  - 使用箇所確認（フロントエンド含む）: 15-20分
  - deprecated マーク付与（2関数）: 15分
  - テスト確認: 10分

**合計**: 約2.5-3.5時間
