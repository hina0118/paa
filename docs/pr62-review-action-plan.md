# PR #62 レビューコメント対応計画

**PR**: [#62 feat: 商品名正規化によるitems/images/product_master連携](https://github.com/hina0118/paa/pull/62)  
**作成日**: 2026-02-04  
**未対応コメント数**: **6件**（すべて未解決）

---

## 概要

PR 62 に対する GitHub Copilot のレビューコメントを整理し、対応計画を作成しました。  
商品名正規化による items/images/product_master 連携機能に関する指摘です。

---

## 未対応コメント一覧

| #   | ファイル                                                   | 行   | 優先度            | 指摘内容                                                                                                                               | 対応方針                                           |
| --- | ---------------------------------------------------------- | ---- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| 1   | `src-tauri/migrations/003_images_item_name_normalized.sql` | 19   | **P0: Critical**  | **UNIQUE(item_id)制約が競合** — `item_name_normalized`による複数アイテムへの画像関連付けと競合。同じitem_idに1レコードしか持てない     | 001 に統合し、正しい制約で定義                     |
| 2   | `src-tauri/src/lib.rs`                                     | 1394 | **P0: Critical**  | **画像保存ロジックのデータ整合性** — UNIQUE(item_id)が残っているため、新規挿入時に制約違反の可能性。既存レコードのマージロジックが必要 | マイグレーション対応後、必要に応じてロジック見直し |
| 3   | `src/lib/orders-queries.ts`                                | 95   | **P1: Important** | **同一正規化名の複数アイテムが同じ画像にマッチ** — 意図した動作か要確認。UX・パフォーマンスの確認                                      | 意図確認。意図通りであればドキュメント化           |
| 4   | `src/lib/orders-queries.ts`                                | 96   | **P1: Important** | **既存itemsのitem_name_normalizedがNULL** — 既存レコードは正規化名が未設定のためJOINが機能しない                                       | マイグレーションで既存itemsを更新                  |
| 5   | `src-tauri/migrations/003_images_item_name_normalized.sql` | 16   | **P2: Nitpick**   | **WHERE句が冗長** — `WHERE item_name_normalized IS NULL` は新規カラム追加直後は全レコードNULLのため冗長                                | 001 統合により該当なし（003 廃止）                 |
| 6   | `src-tauri/src/repository.rs`                              | 729  | **P1: Important** | **空文字列の正規化結果** — 記号のみ・空白のみの商品名で空文字列が返り、複数商品が同一キーにマッピングされる                            | 空文字列の場合はNULLに変換                         |

---

## 対応計画（優先順）

### マイグレーション方針

**リリース前のため、003 を新規追加せず 001_init.sql に統合する。**

---

### Phase 1: P0 Critical — マイグレーション・制約の修正

#### 1.1 001_init.sql の images テーブル修正（003 を統合）

**現状の問題**:

- `001_init.sql` で `images` テーブルに `UNIQUE (item_id)` と `idx_images_item_id` が定義されている
- 新設計では `item_name_normalized` をキーに画像を管理（1正規化名 = 1画像、複数itemが共有）
- `UNIQUE(item_id)` が残ると、同じitem_idで複数レコード作成時に制約違反

**対応内容**:

1. `UNIQUE(item_id)` 制約を削除
2. `item_name_normalized` カラムを追加し、UNIQUE 制約を追加（NULL は複数許可）

**001_init.sql への統合案**（images テーブル定義を置き換え）:

```sql
-- images (file_name のみ、app_data_dir/images/ に実体保存)
-- item_name_normalized: パース再実行時にも画像を維持するため、正規化商品名で関連付け
CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL,
    item_name_normalized TEXT,
    file_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    UNIQUE (item_name_normalized)
);
CREATE INDEX IF NOT EXISTS idx_images_item_id ON images(item_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_images_item_name_normalized ON images(item_name_normalized) WHERE item_name_normalized IS NOT NULL;
```

※ SQLite の `UNIQUE` 制約は NULL を複数許可するため、`item_name_normalized` が NULL のレコードは複数持てる。非NULLの場合は1件のみ。  
※ 003 マイグレーションは削除し、lib.rs のマイグレーションリストからも削除する。

---

#### 1.2 既存 items の item_name_normalized 更新（コメント #4 対応）

**問題**: 既存の items レコードは `item_name_normalized` が NULL のまま。orders-queries の JOIN が機能しない。

**対応**: 001 統合後は新規インストールのみ想定されるため、既存データ移行は不要の可能性が高い。  
リリース前で DB を再作成する前提であれば、既存 items の更新処理は不要。

**万が一既存 DB がある場合**: アプリ起動時に一度だけ Rust で `UPDATE items SET item_name_normalized = normalize_product_name(item_name) WHERE item_name_normalized IS NULL` を実行する処理を検討。

---

### Phase 2: P1 Important — ロジック・エッジケース

#### 2.1 repository.rs: 空文字列を NULL に変換（コメント #6）

**問題**: `normalize_product_name("【特典】")` → 空文字列。複数商品が同一空文字列にマッピングされる。

**対応**:

```rust
// repository.rs の INSERT 時
let item_name_normalized = {
    let n = normalize_product_name(&item.name);
    if n.is_empty() { None } else { Some(n) }
};
```

`normalize_product_name` の戻り値を `Option<String>` に変えるか、repository 側で空文字列を `None` に変換する。

---

#### 2.2 orders-queries: 同一正規化名の複数アイテムが同じ画像にマッチ（コメント #3）

**確認事項**: これは**意図した動作**かどうか。

- **意図通りとする場合**: 同じ商品名の異なる注文のアイテムが同じ画像を共有する。UX的に妥当であれば、PR説明またはコードコメントで明記。
- **意図と異なる場合**: item_id を併用する等、JOIN 条件の見直しが必要。

**推奨**: 商品名正規化の目的が「パース再実行時も画像を維持」であり、同じ正規化名＝同じ商品とみなす設計であれば、**意図通り**と判断し、PR説明に追記。

---

### Phase 3: P2 Nitpick

#### 3.1 マイグレーション003の WHERE 句（コメント #5）

**001 統合により該当なし**。003 を廃止し 001 でスキーマを定義するため、既存データ移行用の UPDATE 文は存在しない。

---

## 対応順序の推奨

| 順  | タスク                                                                                             | 優先度 | 依存                                  |
| --- | -------------------------------------------------------------------------------------------------- | ------ | ------------------------------------- |
| 1   | 001_init.sql に images の item_name_normalized を統合（003 廃止、lib.rs からマイグレーション削除） | P0     | -                                     |
| 2   | 既存 items の item_name_normalized 更新                                                            | P1     | 1（リリース前・DB再作成前提なら不要） |
| 3   | repository.rs: 空文字列→NULL 変換                                                                  | P1     | -                                     |
| 4   | lib.rs: 画像保存ロジックの見直し（制約変更後の整合性確認）                                         | P0     | 1                                     |
| 5   | orders-queries: 同一正規化名の画像共有が意図通りか確認・ドキュメント化                             | P1     | -                                     |

---

## 補足: 001 統合時の images テーブル

- `CREATE UNIQUE INDEX ... WHERE item_name_normalized IS NOT NULL` は、NULL を複数許可しつつ非NULLを一意にする partial unique index
- `item_id` は NOT NULL のまま（どの item に紐づくかの追跡用）、`item_name_normalized` が画像共有の論理キー

---

## 次のアクション

1. **feature/items-normalized-name ブランチ**で上記 Phase 1 から順に実装
2. 001_init.sql に images テーブル定義を統合し、003 マイグレーションファイルおよび lib.rs の参照を削除
3. `cargo test` および DB 再作成後の手動確認
4. 全対応後、レビューコメントに返信して解決済みにマーク
