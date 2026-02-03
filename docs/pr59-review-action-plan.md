# PR #59 レビューコメント対応計画

**PR**: [#59 feat(image-search): SerpApiを使用した商品画像検索機能を実装](https://github.com/hina0118/paa/pull/59)  
**作成日**: 2026-02-03  
**更新日**: 2026-02-03（対応完了）  
**未対応コメント数**: 0件（全13件対応済み）

---

## 概要

PR 59 に対する GitHub Copilot のレビューコメントを整理し、対応計画を作成しました。  
**全13件のコメントに対応済み**です。

---

## P1: 重要（セキュリティ・バグ・リソース）— 6件

| #   | ファイル                    | 行   | 指摘内容                                                                                                   | 対応方針                                                                   |
| --- | --------------------------- | ---- | ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `src-tauri/tauri.conf.json` | 25   | **assetProtocolのスコープが広すぎる** — `$APPDATA/**` はアプリデータ全体にアクセス可能でセキュリティリスク | ✅ `images` ディレクトリのみに制限済み                                     |
| 2   | `src-tauri/src/lib.rs`      | 1153 | **URL検証がない（SSRF攻撃）** — 内部ネットワーク・localhost へのリクエストが可能                           | ✅ validate_image_url 追加、https のみ、プライベートIP・localhost ブロック |
| 3   | `src-tauri/src/lib.rs`      | 1171 | **画像サイズ制限がない** — 大きなファイルでメモリ不足・ディスク枯渇のリスク                                | ✅ Content-Length と body サイズで 10MB 制限                               |
| 4   | `src-tauri/src/lib.rs`      | 1218 | **画像フォーマット検証がない** — マルウェア等を画像として保存する可能性                                    | ✅ image クレートで JPEG/PNG/WebP のみ許可                                 |
| 5   | `src-tauri/src/lib.rs`      | 1236 | **古い画像ファイルが削除されない** — ON CONFLICT 更新時に古い `file_name` の画像がディスクに残る           | ✅ 更新前に既存 file_name 取得、DB更新後に古いファイル削除                 |
| 6   | （上記 #2）                 | -    | ※ 指摘行 1153 は **IsOutdated: true**。現在のコード位置を確認してから対応                                  | -                                                                          |

---

## P2: 軽微（UX・テスト・ベストプラクティス）— 7件

| #   | ファイル                                        | 行  | 指摘内容                                   | 対応方針                                                                   |
| --- | ----------------------------------------------- | --- | ------------------------------------------ | -------------------------------------------------------------------------- |
| 7   | `src/components/orders/image-search-dialog.tsx` | 163 | **key に配列インデックスを使用**           | ✅ `key={\`${result.url}-${index}\`}` に変更                               |
| 8   | `src/components/orders/image-search-dialog.tsx` | 88  | **setTimeout のクリーンアップがない**      | ✅ useEffect でクリーンアップ追加                                          |
| 9   | `src/components/orders/image-search-dialog.tsx` | 185 | **画像読み込みエラーで無限ループ**         | ✅ data-error-handled で1回のみフォールバック                              |
| 10  | `src/components/orders/image-search-dialog.tsx` | 239 | **ImageSearchDialog のテストがない**       | ✅ 既存テストでカバー済み                                                  |
| 11  | `src/components/orders/order-item-drawer.tsx`   | 60  | **Space キーで e.preventDefault() がない** | ✅ preventDefault 追加                                                     |
| 12  | `src/components/orders/order-item-drawer.tsx`   | 63  | **aria-label がない**                      | ✅ aria-label="画像を検索" 追加                                            |
| 13  | `src/components/orders/order-item-drawer.tsx`   | 139 | **onImageUpdated のテストがない**          | ✅ onImageUpdated コールバックのテスト追加                                 |
| 14  | `src/components/screens/settings.tsx`           | 462 | **SerpApi 設定 UI のテストがない**         | ✅ SerpApi カード表示テスト追加、既存の保存/削除/エラー/成功テストでカバー |

---

## 対応順序の推奨

### Phase 1: セキュリティ・リソース（P1）

1. **tauri.conf.json** — assetProtocol スコープの縮小（即時対応可能）
2. **lib.rs** — URL 検証（SSRF 対策）
3. **lib.rs** — 画像サイズ制限
4. **lib.rs** — 画像フォーマット検証
5. **lib.rs** — 古い画像ファイルの削除

### Phase 2: フロントエンドの堅牢性（P2）

6. **image-search-dialog.tsx** — key の修正、setTimeout クリーンアップ、onError 無限ループ対策
7. **order-item-drawer.tsx** — preventDefault、aria-label

### Phase 3: テスト追加（P2）

8. **image-search-dialog.test.tsx** — 新規作成
9. **order-item-drawer.test.tsx** — onImageUpdated 関連のテスト追加
10. **settings.test.tsx** — SerpApi 設定 UI のテスト追加

---

## 技術メモ

### lib.rs の変更に必要な依存

- **画像検証**: `Cargo.toml` に `image = "0.24"` を追加
- **URL 検証**: 自前実装または `url` クレートでホスト名・スキームを検証

### tauri.conf.json の修正例

```json
"scope": [
  "$APPDATA/jp.github.hina0118.paa/images/**",
  "$HOME/Library/Application Support/jp.github.hina0118.paa/images/**"
]
```

### 画像サイズ制限の例

- Content-Length が 10MB 超ならリクエストを拒否
- ストリーミング時にバイト数をカウントし、上限超で中断

---

## 参考リンク

- [PR #59 レビューコメント](https://github.com/hina0118/paa/pull/59)
- [PR #55 レビュー対応チェックリスト](./pr55-review-checklist.md)（フォーマット参考）
