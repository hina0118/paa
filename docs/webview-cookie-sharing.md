# WebView と Rust HTTP リクエスト間の Cookie 共有

## 概要

WebView でサイトにログインした後、Rust 側で HTTP リクエストを発行する際に Cookie を共有できるかどうかの調査結果をまとめたドキュメントです。

## 結論

**可能です。** Tauri 2.9.5 の `WebviewWindow` には Cookie を取得・設定する API が既に用意されています。

## 利用可能な API（Tauri 2.9.5）

`WebviewWindow` には以下のメソッドが用意されています：

| メソッド                | 説明                                            |
| ----------------------- | ----------------------------------------------- |
| `cookies()`             | すべての Cookie を取得（HttpOnly・Secure 含む） |
| `cookies_for_url(url)`  | 指定 URL 用の Cookie を取得                     |
| `set_cookie(cookie)`    | Cookie を設定                                   |
| `delete_cookie(cookie)` | Cookie を削除                                   |

`cookies()` と `cookies_for_url()` は **HttpOnly / Secure フラグ付きの Cookie も取得可能** です。`document.cookie` では不可能なため、ログイン後のセッション Cookie の取得に適しています。

## 実装の流れ

1. WebView でログイン用のサブウィンドウを開く
2. ユーザーがログイン完了後、フロントから Tauri コマンドを invoke
3. そのコマンド内で `webview.cookies()` または `cookies_for_url(url)` を呼び出して Cookie を取得
4. 取得した Cookie を `Cookie` ヘッダ形式に変換し、hyper の HTTP リクエストに付与

## 実装例

```rust
// 1. ログインした WebView ウィンドウを取得（例: サブウィンドウのラベル）
let webview = app_handle.get_webview_window("login-window")?;

// 2. Cookie を取得（async で実行、Windows のデッドロック回避）
let cookies = webview.cookies()?;

// 3. Cookie ヘッダ用の文字列に変換
let cookie_header: String = cookies
    .iter()
    .map(|c| format!("{}={}", c.name(), c.value()))
    .collect::<Vec<_>>()
    .join("; ");

// 4. HTTP リクエストに付与
let req = Request::builder()
    .method(Method::GET)
    .uri(&target_url)
    .header("Cookie", cookie_header)
    .header("User-Agent", "...")
    .body(...)?;
```

## 注意点・制約

### 1. Windows でのデッドロック

`cookies()` を同期的なコマンドやイベントハンドラ内で呼ぶと **デッドロック** する可能性があります。必ず **async コマンド** で呼び出し、必要に応じて別スレッドで実行してください。

### 2. Android は未対応

`cookies()` は常に空の `Vec` を返します。

### 3. Cookie ストアのスコープ

複数ウィンドウ間で Cookie ストアが共有される場合があります。ログイン用の WebView をサブウィンドウで開く場合は、そのウィンドウのラベルで `get_webview_window(label)` して取得する想定で問題ありません。

### 4. tauri:// プロトコル

ローカルファイル（`tauri://` など）経由で設定された Cookie は対象外です。ログイン用 WebView が `https://` の URL を表示している限りは問題ありません。

## 背景: WebView と Rust HTTP の Cookie 管理

- **WebView**: ブラウザエンジン（Wry）が Cookie を管理
- **Rust HTTP クライアント**（hyper / reqwest）: 独自の Cookie ストア
- これらは **自動では同期されません**。明示的に WebView から取得して HTTP リクエストに付与する必要があります。

## 参考リンク

- [Tauri WebviewWindow::cookies](https://docs.rs/tauri/2.9.5/tauri/webview/struct.WebviewWindow.html#method.cookies)
- [Tauri WebviewWindow::cookies_for_url](https://docs.rs/tauri/2.9.5/tauri/webview/struct.WebviewWindow.html#method.cookies_for_url)
- [Tauri Discussion #11655](https://github.com/tauri-apps/tauri/discussions/11655) - Wry の Cookie API を Tauri で使う方法
- [wry Issue #785](https://github.com/tauri-apps/wry/issues/785) - Secure Cookie 取得について（Tauri 2.9 では対応済み）

## 調査日

2025-02-08
