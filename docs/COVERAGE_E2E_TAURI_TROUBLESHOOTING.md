# coverage-e2e-tauri 失敗時の解析手順

CI の `coverage-e2e-tauri` ジョブが失敗した場合、Windows 環境からでも解析できるよう、以下を実施しています。

## 1. 失敗時にアップロードされるアーティファクト

ジョブ失敗時、以下のアーティファクトが自動アップロードされます（14日間保持）:

| アーティファクト名         | 内容                                                              |
| -------------------------- | ----------------------------------------------------------------- |
| `coverage-e2e-tauri-debug` | `coverage-e2e-tauri/` ディレクトリ、`coverage-e2e-tauri-test.log` |

### ダウンロード方法

1. GitHub Actions の該当ワークフロー実行を開く
2. 失敗した `Coverage E2E Tauri (Rust)` ジョブをクリック
3. 右側の **Artifacts** から `coverage-e2e-tauri-debug` をダウンロード

## 2. 解析の進め方

### 2.1 テストログの確認

`coverage-e2e-tauri-test.log` を開き、以下を確認:

- **E2E テストの失敗**: どの spec が失敗したか、エラーメッセージ
- **tauri-driver のエラー**: `tauri-driver error:` で検索
- **ビルドエラー**: `Tauri build failed` で検索

### 2.2 profraw ファイルの確認

`coverage-e2e-tauri/` または `src-tauri/target/` 内の `*.profraw` を確認（wdio が target/ に出力するよう設定）:

| 状態               | 想定原因                                                                                        |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| ファイルなし       | ビルドが `-Cinstrument-coverage` で行われていない、または LLVM_PROFILE_FILE が渡っていない      |
| 0 バイトのファイル | ビルドは instrument 済みだが、プロセス終了時に profraw がフラッシュされていない（強制終了など） |
| 正のサイズ         | 正常。マージ/レポート生成ステップの失敗を疑う                                                   |

### 2.3 CI ログの確認ポイント

**Merge profraw and generate coverage report** ステップの出力:

```
=== coverage-e2e-tauri contents ===
Profraw file count: N
```

- `N = 0`: profraw が生成されていない → ビルド・環境変数の確認
- `N > 0` かつ 0 バイト: プロセス終了の問題
- `N > 0` かつ正のサイズ: 後続の `cargo llvm-cov report` のエラーを確認

## 3. ローカルでの再現（Windows）

Windows では profraw が 0 バイトになることがありますが、テスト実行までは可能です。

```powershell
$env:MSEDGEDRIVER_PATH = "c:\path\to\msedgedriver.exe"
$env:PAA_E2E_COVERAGE = "1"
$env:RUSTFLAGS = "-Cinstrument-coverage"
npm run test:e2e:tauri
```

- テスト失敗の再現は可能
- カバレッジ数値の取得は CI（Linux）で行う

## 4. よくある失敗パターン

| 症状                           | 対処                                                                                                                                                       |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `No profraw files found`       | カバレッジ時は application に `scripts/run-paa-with-coverage.sh` を指定。tauri-driver は子プロセスに env を渡さないためラッパーで LLVM_PROFILE_FILE を設定 |
| `llvm-profdata not found`      | `rustup component add llvm-tools-preview` が実行されているか確認                                                                                           |
| `cargo llvm-cov report` エラー | profraw を `src-tauri/target/` にコピーしてから report を実行しているか確認。`--no-run` を付与して E2E の profraw を使用すること（テスト再実行を避ける）   |
| E2E テスト失敗（PAA 要素なし） | タイミング問題の可能性。`expectSidebarVisible` の待機時間を延長する検討                                                                                    |

## 5. 関連ファイル

- `.github/workflows/coverage.yml`: coverage-e2e-tauri ジョブ定義
- `wdio.tauri.conf.ts`: ビルド・tauri-driver 起動時の環境変数、カバレッジ時の application 指定
- `scripts/run-paa-with-coverage.sh`: LLVM_PROFILE_FILE を設定して paa を起動するラッパー（CI 用）
- `docs/E2E_TESTING.md`: Tauri E2E の基本手順
