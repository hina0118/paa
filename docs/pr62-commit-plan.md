# PR62 コミット計画

## 変更ファイル一覧

| ファイル                                          | 変更内容                                                               |
| ------------------------------------------------- | ---------------------------------------------------------------------- |
| `.cursorrules`                                    | Custom Commands (plan_pr, do_plan, plan_commit) 追加                   |
| `src-tauri/migrations/001_init.sql`               | images から item_id 削除、item_name_normalized を NOT NULL に          |
| `src-tauri/src/lib.rs`                            | save_image: item_name_normalized NULL 時はエラー、item_id ロジック削除 |
| `src/lib/utils.ts`                                | getProductMetadata ユーティリティ追加                                  |
| `src/lib/utils.test.ts`                           | getProductMetadata のテスト追加                                        |
| `src/components/orders/order-item-card.tsx`       | getProductMetadata で重複解消                                          |
| `src/components/orders/order-item-row.tsx`        | getProductMetadata で重複解消                                          |
| `src/lib/e2e-mock-db.ts`                          | IMAGES_SCHEMA から item_id 削除                                        |
| `src/lib/e2e-mock-db.test.ts`                     | カラム数・期待値更新                                                   |
| `src/components/tables/table-viewer.tsx`          | item_id → item_name_normalized ラベル変更                              |
| `README.md`                                       | images の説明更新                                                      |
| `docs/architecture/product-name-normalization.md` | item_id 削除、既存DBマイグレーション方針                               |
| `.serena/memories/database_schema.md`             | images スキーマ更新                                                    |
| `docs/pr62-review-action-plan.md`                 | 新規: レビュー対応計画                                                 |
| `docs/pr62-unused-items-analysis.md`              | 新規: 不要項目分析                                                     |

---

## 推奨コミット分割

### Commit 1: プロジェクト設定

```
chore: .cursorrules に Custom Commands 追加

- /plan_pr: PR 対応計画作成
- /do_plan: 対応計画実行
- /plan_commit: 対応内容をコミット
```

**対象**: `.cursorrules`

### Commit 2: PR62 本対応

```
feat: images から item_id を削除し item_name_normalized をリレーションキーに

- 001_init.sql: item_id, FOREIGN KEY, idx_images_item_id 削除
- lib.rs: item_name_normalized NULL 時は画像登録不可（エラー返却）
- getProductMetadata ユーティリティで order-item-card/row の重複解消
- e2e-mock-db, table-viewer, README, docs 更新
- pr62-review-action-plan.md, pr62-unused-items-analysis.md 追加
```

**対象**: 上記以外の全ファイル

---

## 1コミットにまとめる場合

```
feat(pr62): item_name_normalized をリレーションキーに、images から item_id 削除

- images: item_id 削除、item_name_normalized NOT NULL に
- 正規化できない商品には画像登録不可
- getProductMetadata で product 表示ロジックを共通化
- .cursorrules に Custom Commands 追加
- ドキュメント更新
```
