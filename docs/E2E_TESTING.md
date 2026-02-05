# E2Eテストガイド

このドキュメントでは、Playwrightを使用したE2E（End-to-End）テストの実行方法と追加方法を説明します。

## 概要

E2Eテストは、実際のブラウザを使用してアプリケーション全体の動作を検証します。ユニットテストや統合テストでは検証できない、画面遷移やユーザー操作の流れをテストできます。

## テストの種類

### 1. フロントエンドのみ（Playwright + Vite）

- **コマンド**: `npm run test:e2e`
- **対象**: Vite 開発サーバ + Chromium。Tauri アプリ（Rust）は起動しない。
- **用途**: フロントの UI・ナビゲーションを素早く検証。Tauri API（`invoke`）は動かないため、保存成功メッセージなどはスキップされる場合あり。

### 2. Tauri アプリ全体（WebdriverIO + tauri-driver）

- **コマンド**: `npm run test:e2e:tauri`
- **対象**: Tauri アプリを起動し、その WebView を WebDriver で操作。フロント + Rust の両方が動く。
- **用途**: 設定の保存など Tauri コマンド経由の動作まで含めた E2E 検証。
- **外部APIモック**: 実行時に `PAA_E2E_MOCK=1` が自動設定され、Gmail・Gemini・SerpApi の実際のAPI呼び出しがモックに置き換わる。CIやローカルで外部依存なしにテスト可能。
- **Rustカバレッジ**: `PAA_E2E_COVERAGE=1` と `RUSTFLAGS="-Cinstrument-coverage"` を設定して実行すると、E2E 実行時の Rust コードのカバレッジを収集できる。CI の `coverage-e2e-tauri` ジョブで自動実行される。

## テストスタック

### Playwright（フロントのみ）

- **テストフレームワーク**: [Playwright](https://playwright.dev/)
- **ブラウザ**: Chromium（デフォルト）
- **開発サーバ**: Vite（自動起動）

### WebdriverIO（Tauri 全体）

- **テストフレームワーク**: [WebdriverIO](https://webdriver.io/)
- **ドライバ**: [tauri-driver](https://crates.io/crates/tauri-driver)（Tauri アプリを WebDriver で操作）
- **Windows**: Microsoft Edge Driver（msedgedriver）が Edge のバージョンと一致している必要あり

## テスト実行

### ローカルでの実行

#### 基本的な実行

```bash
npm run test:e2e
```

このコマンドは以下を自動的に実行します：

1. Vite開発サーバを起動（`npm run dev`）
2. Playwrightテストを実行
3. テスト終了後に開発サーバを停止

#### Tauri アプリ全体の E2E（WebdriverIO）

Tauri アプリを起動してフロント＋Rust まで含めてテストする場合：

```bash
npm run test:e2e:tauri
```

**前提条件**

1. **tauri-driver** のインストール

   ```bash
   cargo install tauri-driver --locked
   ```

2. **Windows の場合**: Microsoft Edge Driver（msedgedriver）を Edge のバージョンに合わせて導入。手順は「[Windows: msedgedriver の用意](#windows-msedgedriver-の用意)」を参照。

3. **Linux の場合**: WebKitWebDriver（例: `webkit2gtk-driver`）が必要。

**実行の流れ**

1. `npm run tauri build -- --debug --no-bundle` で Tauri アプリをビルド
2. tauri-driver を起動
3. `tests/e2e-tauri/**/*.spec.ts` のスペックを WebdriverIO で実行

テストファイルは `tests/e2e-tauri/` にあります（Playwright の `tests/e2e/` とは別）。

##### Windows: msedgedriver の用意

Tauri の WebDriver は Windows では **msedgedriver** を使います。**Edge のバージョンと msedgedriver のバージョンが一致している必要があります**（一致しないと接続でハングすることがあります）。

**方法 A: 手動でダウンロードして PATH に通す**

1. **Edge のバージョンを確認**  
   Edge を開き、アドレスバーに `edge://version` と入力して「Microsoft Edge」のバージョン番号を確認（例: 131.0.2903.92）。

2. **同じバージョンの msedgedriver をダウンロード**  
   [Microsoft Edge WebDriver](https://developer.microsoft.com/microsoft-edge/tools/webdriver/) を開き、表示されている Edge のバージョンに合う「x64」用のドライバをダウンロード。

3. **解凍して配置**  
   ダウンロードした ZIP を解凍し、`msedgedriver.exe` を任意のフォルダに置く（例: `C:\tools\msedgedriver\`）。

4. **PATH に追加**
   - 「環境変数」を開く（Windows の検索で「環境変数を編集」など）。
   - 「ユーザー環境変数」または「システム環境変数」の「Path」を編集し、`msedgedriver.exe` を置いたフォルダを追加。
   - 新しいターミナルを開き、`msedgedriver --version` で通っているか確認。

**方法 B: msedgedriver-tool で自動ダウンロード**

1. **ツールのインストール**

   ```bash
   cargo install --git https://github.com/chippers/msedgedriver-tool
   ```

2. **実行して msedgedriver を取得**

   ```bash
   # Windows (PowerShell)
   & "$env:USERPROFILE\.cargo\bin\msedgedriver-tool.exe"
   ```

   実行後、ツールが Edge のバージョンを検出し、同じバージョンの `msedgedriver.exe` をダウンロードします。保存先はツールの表示を確認してください（多くの場合カレントディレクトリや `%USERPROFILE%\.cargo\bin` 付近）。

3. **PATH に通すか、環境変数で指定**
   - **PATH に通す**: ダウンロードされた `msedgedriver.exe` があるフォルダをシステムの PATH に追加。
   - **環境変数で指定**: PATH に追加したくない場合は、`msedgedriver.exe` の**フルパス**を環境変数 `MSEDGEDRIVER_PATH` に設定する。  
     例（PowerShell）:
     ```powershell
     $env:MSEDGEDRIVER_PATH = "C:\Users\あなたのユーザー名\.cargo\bin\msedgedriver.exe"
     ```
     プロジェクトの `wdio.tauri.conf.ts` は、この環境変数が設定されている場合に `tauri-driver` に `--native-driver` でそのパスを渡します。

**まとめ**

- PATH に `msedgedriver.exe` を入れる → 何も設定せず `npm run test:e2e:tauri` で動く。
- PATH に入れない → `MSEDGEDRIVER_PATH` にフルパスを設定してから `npm run test:e2e:tauri` を実行。

#### UIモードでの実行（Playwright）

```bash
npm run test:e2e:ui
```

PlaywrightのUIモードでテストを実行します。テストの実行状況を視覚的に確認できます。

### CI環境での実行

GitHub Actionsで自動的に実行されます。以下のトリガーで実行されます：

- Pull Request作成時（main, master, developブランチへのPR）
- プッシュ時（main, master, developブランチへのプッシュ）
- 手動実行（workflow_dispatch）

**実行されるE2Eテスト**:

1. **Playwright（フロントのみ）** - `ubuntu-latest` で Vite + Chromium
2. **Tauri（WebdriverIO + tauri-driver）** - `ubuntu-latest` と `windows-latest` で Tauri アプリ全体

## テストファイルの構造

```
tests/
├── e2e/                  # Playwright（フロントのみ）
│   ├── helpers.ts
│   ├── fixtures.ts
│   ├── dashboard.spec.ts
│   ├── navigation.spec.ts
│   └── settings.spec.ts
└── e2e-tauri/            # WebdriverIO（Tauri 全体）
    ├── navigation.spec.ts
    └── settings.spec.ts
```

## テストの追加方法

### 1. 新しいテストファイルの作成

`tests/e2e/`ディレクトリに新しい`.spec.ts`ファイルを作成します。

```typescript
import { test, expect } from '@playwright/test';
import { navigateToScreen, expectScreenTitle } from './helpers';

test.describe('新しい画面のテスト', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await navigateToScreen(page, '画面名');
  });

  test('画面が表示される', async ({ page }) => {
    await expectScreenTitle(page, '画面タイトル');
  });
});
```

### 2. ヘルパー関数の使用

`tests/e2e/helpers.ts`に定義されているヘルパー関数を使用することで、テストコードを簡潔に保てます。

#### 利用可能なヘルパー関数

- `navigateToScreen(page, screenName)`: サイドバーから指定の画面に遷移
- `expectScreenTitle(page, title)`: 画面タイトルが表示されていることを確認
- `expectSidebarVisible(page)`: サイドバーが表示されていることを確認
- `expectSuccessMessage(page, message?)`: 成功メッセージが表示されていることを確認
- `expectErrorMessage(page, message?)`: エラーメッセージが表示されていることを確認
- `expectButtonEnabled(page, buttonName)`: ボタンが有効化されていることを確認
- `expectButtonDisabled(page, buttonName)`: ボタンが無効化されていることを確認
- `fillInput(page, label, value)`: 入力フィールドに値を入力
- `expectCardVisible(page, title)`: カードが表示されていることを確認

### 3. テストのベストプラクティス

#### ページオブジェクトパターンの使用

複雑な画面の場合は、ページオブジェクトパターンを使用することを推奨します。

```typescript
// tests/e2e/pages/settings-page.ts
export class SettingsPage {
  constructor(private page: Page) {}

  async navigate() {
    await navigateToScreen(this.page, 'Settings');
  }

  async setBatchSize(value: string) {
    const input = this.page.getByLabel('バッチサイズ').locator('input');
    await input.fill(value);
  }

  async saveBatchSize() {
    await this.page
      .getByLabel('バッチサイズ')
      .locator('..')
      .getByRole('button', { name: '保存' })
      .click();
  }
}
```

#### 非同期処理の待機

非同期処理が完了するまで適切に待機します。

```typescript
test('非同期処理のテスト', async ({ page }) => {
  await page.getByRole('button', { name: '更新' }).click();

  // 読み込み完了を待機
  await page.waitForSelector('text=読み込み完了', { timeout: 5000 });

  // または、特定の要素が表示されるまで待機
  await expect(page.getByText('データが表示されました')).toBeVisible();
});
```

#### エラーハンドリングのテスト

エラーケースもテストに含めます。

```typescript
test('無効な値の入力時にエラーが表示される', async ({ page }) => {
  await fillInput(page, 'バッチサイズ', '0');
  await page.getByRole('button', { name: '保存' }).click();
  await expectErrorMessage(page, 'バッチサイズは1以上の整数を入力してください');
});
```

## CI環境での失敗時のデバッグ

### Artifactsの確認

GitHub Actionsでテストが失敗した場合、以下のArtifactsが自動的に保存されます：

1. **playwright-report/**: HTML形式のテストレポート
2. **test-results/**: テスト実行結果（スクリーンショット、ビデオ、トレース）
3. **screenshots/**: 失敗時のスクリーンショット
4. **videos/**: 失敗時のビデオ記録
5. **traces/**: 失敗時のトレースファイル

### ローカルでトレースを再生

CI環境で失敗したテストのトレースファイルをダウンロードし、ローカルで再生できます。

```bash
npx playwright show-trace path/to/trace.zip
```

## カバレッジ

### フロントエンド（JS）カバレッジ

PlaywrightのE2Eテストでは基本的なJSカバレッジ（関数カバレッジ）を収集しています。より詳細なカバレッジが必要な場合は、`vite-plugin-istanbul`などの追加設定が必要です。

- **目標カバレッジ率: 25%**（いったん。CI では未達の場合に E2E ジョブが失敗します）

```bash
npm run test:e2e:coverage
```

### 統合カバレッジ（JS + Rust）

E2EテストのJSカバレッジと、Rustのユニット/統合テストのカバレッジを併せて計測できます。OS によって実行するコマンドが異なります。

```bash
# Windows（PowerShell スクリプトを実行）
npm run test:e2e:rust-coverage

# macOS / Linux（bash スクリプトを実行）
npm run test:e2e:rust-coverage:sh
```

**前提条件**: `cargo-llvm-cov` がインストールされていること

```bash
cargo install cargo-llvm-cov
```

**実行内容**:

1. E2Eテスト（Playwright）→ フロントエンドJSカバレッジを収集
2. Rustテスト（cargo llvm-cov）→ Rustカバレッジを収集
3. 両方のサマリーを表示

**出力**:

- フロントエンド: `coverage-e2e/coverage-data.json`
- Rust HTML: `src-tauri/target/llvm-cov/html/index.html`
- Rust LCOV: `coverage-e2e/rust-coverage.lcov`

### Tauri E2E の Rust カバレッジ

Tauri アプリを起動して E2E テストを実行しながら Rust のカバレッジを収集できます。CI の `coverage-e2e-tauri` ジョブで自動実行されます。

```bash
# ローカル実行（RUSTFLAGS を設定してビルド）
RUSTFLAGS="-Cinstrument-coverage" npm run test:e2e:tauri:coverage
```

**実行後のカバレッジレポート生成**（ローカル）:

```bash
# profraw をマージしてレポート生成
llvm-profdata merge -sparse coverage-e2e-tauri/*.profraw -o coverage-e2e-tauri/merged.profdata
cd src-tauri && cargo llvm-cov report --no-run --all-features -C "profile-use=../coverage-e2e-tauri/merged.profdata" --text
```

**出力**: `coverage-e2e-tauri/lcov.info`（CI で生成）、`coverage-e2e-tauri/merged.profdata`

## トラブルシューティング

### 開発サーバが起動しない

- ポート1420が既に使用されている可能性があります
- 別のプロセスでViteサーバが起動している場合は停止してください

### テストがタイムアウトする

- `playwright.config.ts`の`timeout`設定を確認してください
- 非同期処理の待機時間が不足している可能性があります

### 要素が見つからない

- 要素が実際に表示されるまで待機しているか確認してください
- `page.waitForSelector()`や`expect().toBeVisible()`を使用して待機します

## 参考資料

- [Playwright Documentation](https://playwright.dev/)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Playwright with GitHub Actions](https://playwright.dev/docs/ci)
