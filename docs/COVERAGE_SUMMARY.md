# カバレッジ計測結果サマリー

計測日: 2026-02-05

## 1. フロントエンド（Vitest ユニットテスト）

| 指標       | カバレッジ | 閾値 |
| ---------- | ---------- | ---- |
| Statements | 88.15%     | 85%  |
| Branches   | 82.26%     | 82%  |
| Functions  | 82.32%     | 82%  |
| Lines      | 89.5%      | 85%  |

**コマンド**: `npm run test:frontend:coverage`

---

## 2. フロントエンド（Playwright E2E）

| 指標               | 値         |
| ------------------ | ---------- |
| 対象ファイル数     | 46         |
| 総関数数           | 333        |
| カバーされた関数数 | 180        |
| **関数カバレッジ** | **54.05%** |
| 目標               | 25%        |

**コマンド**: `npm run test:e2e`

---

## 3. バックエンド（Rust ユニット/統合テスト）

| 指標      | カバレッジ | 閾値 |
| --------- | ---------- | ---- |
| Lines     | 69.25%     | 65%  |
| Functions | 58.32%     | -    |
| Regions   | 71.80%     | -    |

**コマンド**: `cd src-tauri && cargo llvm-cov --all-features --workspace --text`

---

## 4. バックエンド（Tauri E2E）

**ローカル**: tauri-driver と msedgedriver をインストール後、以下で計測可能。

```powershell
# msedgedriver を PATH に追加（または MSEDGEDRIVER_PATH を設定）
$env:MSEDGEDRIVER_PATH = "c:\app\project\paa\msedgedriver\msedgedriver.exe"
$env:PAA_E2E_COVERAGE = "1"
$env:RUSTFLAGS = "-Cinstrument-coverage"
npm run test:e2e:tauri
```

**注意**: 一部テストが「PAA」要素の検索で失敗することがある（タイミング依存の可能性）。CI（ubuntu-latest）では `coverage-e2e-tauri` ジョブで自動計測。

---

## まとめ

| 種別           | 対象                          | カバレッジ | 閾値 |
| -------------- | ----------------------------- | ---------- | ---- |
| Vitest         | フロント（ユニット）          | 88–90%     | 85%  |
| Playwright E2E | フロント（E2E）               | 54%        | 25%  |
| cargo llvm-cov | バックエンド（ユニット/統合） | 69%        | 65%  |
| Tauri E2E      | バックエンド（E2E）           | CI で計測  | -    |
