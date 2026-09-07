# Asana GPUI

[GPUI Kit](https://github.com/longbridge/gpui-kit) 0.6.0 を使った Rust 製のネイティブ Asana クライアントです。
Web 版の Work ナビゲーションと Home、Inbox、My tasks、プロジェクトページを再現しています。
OAuth 2.0 で実際のワークスペースに接続し、タスクの作成・編集・完了・コメント投稿ができます。

## 起動

Rust が必要です。macOS では Xcode 開発ツールも必要です。
macOS の開発・動作確認環境は macOS 26.6.2、Rust 1.98.1 です。
Linux の準備と検証範囲は後述の「Linux」を参照してください。

接続せずに画面を試す場合:

```sh
cargo run --locked -- --demo
```

既存の環境変数で接続する場合:

```sh
# ASANA_API_KEY が設定済みのシェルで実行
cargo run --locked -- --pat
```

OAuth で接続する場合:

```sh
# ASANA_CLIENT_ID / ASANA_CLIENT_SECRET が設定済みのシェルで実行
cargo run --locked
```

「Connect with Asana」を押すと、OS の既定ブラウザで認可画面が開きます。
認可後はアプリが自動的にワークスペースを読み込みます。
すでに保存済みの OAuth セッションがある場合、起動時に復元します。
Settings からも OAuth に切り替えられます。

macOS のアプリケーション形式でビルドする場合:

```sh
scripts/bundle-macos.sh
"target/debug/Asana GPUI.app/Contents/MacOS/asana-gpui"

# 配布用の最適化ビルドが必要な場合
scripts/bundle-macos.sh --release
```

生成先は `target/debug/Asana GPUI.app` または `target/release/Asana GPUI.app` です。
Finder からの起動では、シェルの環境変数が引き継がれない場合があります。
初回は上記のコマンドで起動するか、接続画面に Client ID と Client secret を入力してください。
ローカル実行用のアドホック署名を付けます。配布用署名・公証・自動更新は含みません。

## Linux

Wayland と X11 の両方を有効にしています。
まずデスクトップセッションの端末から `cargo run --locked -- --demo` で起動してください。
SSH や TTY だけの環境では画面を表示できません。

ビルドには C/C++ ツールチェーン、pkg-config、Fontconfig、FreeType、Wayland、X11/XCB、xkbcommon の開発用ライブラリが必要です。
実行環境には GPU ドライバー、Vulkan ローダー、日本語を含む表示用フォント、既定ブラウザを開く `xdg-open` を用意してください。
上流の [Linux 依存関係インストール手順](https://github.com/longbridge/gpui-kit/blob/main/script/install-linux.sh) も参照できます。
ディストリビューションによってパッケージ名が異なります。

OAuth セッションの保存には、ログイン中の D-Bus セッションと Secret Service 対応の資格情報ストア（GNOME Keyring など）が必要です。
ストアが使えない場合はエラーを表示し、その起動中だけ接続できます。
PAT は Linux でも保存しません。

Linux 用の実行ファイルとデスクトップファイルをまとめる場合:

```sh
sh scripts/bundle-linux.sh
target/debug/asana-gpui-linux/bin/asana-gpui --demo

# 最適化ビルド
sh scripts/bundle-linux.sh --release
```

生成先は `target/debug/asana-gpui-linux/` または `target/release/asana-gpui-linux/` です。
スクリプトは実行した Linux の CPU アーキテクチャ向けにビルドし、システムへのインストールは行いません。
共有ライブラリや GPU ドライバーは同梱しないため、生成物が別のディストリビューションで動くことは保証しません。

アプリメニューへ登録する場合は、生成物を `/usr/local` などデスクトップセッションの `PATH` が通る場所へ配置します。
以下はデバッグビルドの配置例です。最適化ビルドを使う場合は `debug` を `release` に変更してください。

```sh
sudo install -Dm755 target/debug/asana-gpui-linux/bin/asana-gpui /usr/local/bin/asana-gpui
sudo install -Dm644 target/debug/asana-gpui-linux/share/applications/jp.ktutumi.asana-gpui.desktop /usr/local/share/applications/jp.ktutumi.asana-gpui.desktop
sudo install -Dm644 target/debug/asana-gpui-linux/share/icons/hicolor/scalable/apps/jp.ktutumi.asana-gpui.svg /usr/local/share/icons/hicolor/scalable/apps/jp.ktutumi.asana-gpui.svg
```

アプリメニューからの起動ではシェルの環境変数が引き継がれない場合があります。
その場合は接続画面へ Client ID と Client secret を入力してください。
削除する場合は上記の配置先の 3 ファイルを削除します。
保存済みのセッションを削除するには、先にアプリからサインアウトしてください。

## OAuth の設定

Asana Developer Console の OAuth アプリに、次の Redirect URL を登録してください。

```text
http://127.0.0.1:18787/callback
```

既存の `asana-cli` と同じクライアント設定を利用できます。
同じポートを使用する別の認証処理は、先に完了させてください。
ポートが使用中の場合はアプリにエラーを表示します。

認証には Authorization Code、PKCE S256、ランダムな `state`、有効期限付きのコールバック待受を使用します。
待受は `127.0.0.1` に限定しています。
認可に失敗した場合、既存の接続とデータを維持します。
手動コード方式を使う場合は、Asana に登録した `urn:ietf:wg:oauth:2.0:oob` を Redirect URL に指定できます。

| 環境変数 | 用途 |
| --- | --- |
| `ASANA_CLIENT_ID` | OAuth アプリの Client ID |
| `ASANA_CLIENT_SECRET` | OAuth アプリの Client secret |
| `ASANA_REDIRECT_URI` | 上記 Redirect URL の変更 |
| `ASANA_OAUTH_SCOPES` | 認可スコープ。既定値は `default` |
| `ASANA_API_KEY` | `--pat` または接続画面から利用する PAT |
| `ASANA_WORKSPACE_ID` | 起動時に優先するワークスペースの GID |
| `ASANA_GPUI_DATA_DIR` | ローカル設定の保存先。絶対パスのみ |

`default` は Asana アプリを Full permissions に設定した場合のスコープです。
Granular scopes を使用する場合は、Developer Console で必要なスコープを有効にし、`ASANA_OAUTH_SCOPES` にスペース区切りで指定してください。
使用する API は users、workspaces、projects、sections、tasks、stories の読み取りと、projects、tasks、stories、セクションへのタスク移動の書き込みです。
スコープ不足やタスク固有のアクセス拒否は画面に表示します。
登録内容は [Asana OAuth の公式資料](https://developers.asana.com/docs/oauth) と [スコープ一覧](https://developers.asana.com/docs/oauth-scopes)を参照してください。

OAuth の access token、refresh token、クライアント設定は OS の資格情報ストアに保存します。
macOS では Keychain の `asana-gpui.oauth` を使用します。
Linux では Secret Service 対応の資格情報ストアを使用します。
期限切れ前の更新に対応し、サインアウトでは保存処理の完了を待ってから資格情報を削除します。
`ASANA_API_KEY` は保存せず、そのプロセス内だけで使用します。
ソースやアプリのバンドルに Client secret を埋め込まないでください。

## 画面と操作

- **Home**: 日付と挨拶、担当タスクの Upcoming / Overdue / Completed、プロジェクト、ワークスペースのメンバー、Private notepad。
- **Inbox**: 担当タスクの更新順一覧、詳細表示、未読フィルター、既読化、アーカイブと復元。
- **My tasks**: 自分の担当タスクを List / Board / Calendar で表示。完了・未完了・期限超過の絞り込み、期日順の並べ替え。
- **プロジェクト**: Overview / List / Board / Calendar / Dashboard、セクションごとのタスク、進捗集計、プロジェクト作成、お気に入り。
- **タスク詳細**: 名前・説明・担当者・期日を編集し、Save changes で保存。完了切り替え、プロジェクト内のセクション移動、サブタスク参照、コメント表示と投稿、Web 版へのリンク。
- **共通**: 読み込み済みのタスク名・プロジェクト名の検索、ワークスペース切り替え、ライト／ダーク表示、通信エラー表示。

左サイドバーの右端と、タスク詳細の左端をドラッグすると幅を変更できます。
調整した幅はアプリを開いている間保持し、サイドバーやタスク詳細を開き直しても引き継ぎます。

新規タスクは作成時に自分を担当者にします。
プロジェクトページでは、そのプロジェクトにも追加します。
未保存の編集やコメントがある場合は、保存・投稿・破棄するまで別タスクへの移動や終了を止めます。
説明などの変更していないフィールドは API に送信せず、他の利用者による更新の上書きを抑えます。
同じフィールドを同時に編集した場合の競合解決画面はありません。

| macOS | Linux | 操作 |
| --- | --- | --- |
| `⌘ K` | `Ctrl K` | 検索欄へ移動 |
| `⌘ N` | `Ctrl N` | My tasks の新規タスク入力へ移動 |
| `Enter` | `Enter` | 新規タスクを作成 |
| `⌘ S` | `Ctrl S` | タスク詳細を保存 |
| `⌘ R` | `Ctrl R` | 現在のデータを再取得 |
| `Esc` | `Esc` | タスク詳細を閉じる |
| `⌘ Q` | `Ctrl Q` | 終了 |

## Web 版との違い

Asana の公開 API に Web Inbox の通知一覧・既読・アーカイブの API はありません。
このアプリの Inbox は、担当タスクの `modified_at` に基づく「タスクごとの最新更新」です。
Web の通知履歴全体やメンション一覧との同期は行いません。
API の制約は [Asana の Inbox API に関する回答](https://forum.asana.com/t/how-i-can-get-inbox-notification-and-tasks-from-asana-api/168830)を参照してください。

お気に入り、既読・アーカイブ、メモ、テーマ、選択ワークスペースは利用者ごとに端末へ保存します。
macOS の標準保存先は `~/Library/Application Support/asana-gpui/<user-gid>.json` です。
Linux では `$XDG_CONFIG_HOME/asana-gpui/<user-gid>.json`、`XDG_CONFIG_HOME` が未設定なら `~/.config/asana-gpui/<user-gid>.json` です。
メモは Save note または画面を離れるときに保存します。
ローカルファイルが壊れていた場合は上書きせず、エラーを表示します。
デモの変更はメモリ内だけに保持します。

API データは手動更新です。
タスクとプロジェクトの読み取りは全ページを取得し、List の行は仮想化しています。
Board・Calendar・Inbox は読み込んだ項目を通常描画するため、大きなワークスペースでは List と検索を使用してください。
サーバー全体の検索、オフライン編集、ドラッグによる並べ替え、Timeline、プロジェクトの Messages / Files、カスタムフィールド編集は含みません。
組織でプロジェクトを作成するときに必要となるチーム選択も未対応です。

## 検証

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

OAuth の PKCE とコールバック検証、更新トークンの保持、ページ送り、タスクの絞り込み、編集差分、更新時の順序保持をテストします。

Linux の GUI 動作は、Wayland と X11 の各デスクトップセッションで以下を確認してください。

- `--demo` での起動、ライト／ダーク表示、日本語の表示と IME 入力、コピーと貼り付け。
- Ctrl ショートカット、ウインドウの移動とリサイズ、拡大率変更、未保存編集がある状態での終了操作。
- OAuth のブラウザ起動とコールバック、再起動時のセッション復元、サインアウト後の資格情報削除。
- 資格情報ストアが使えない場合のエラー表示と、その起動中だけの接続。
- ローカル設定の保存と復元、アプリメニューからの起動とアイコン表示。

Linux のビルド検証環境は Omarchy 4.0.2（Arch 系）、Rust 1.98.1 です。
GUI と実 OAuth の Linux 上での動作確認は未実施です。

実 API の書き込みテストは通常実行から除外しています。
実行すると指定したテストプロジェクトにタスクとコメントを作成し、編集・セクション移動・完了まで検証します。
作成物は確認用に残します。
誤操作防止のため、ワークスペース名が `Test`、プロジェクト名が `GPUI Client` で始まることを確認します。

```sh
ASANA_GPUI_TEST_WORKSPACE=<TestのGID> \
ASANA_GPUI_TEST_PROJECT=<テストプロジェクトのGID> \
cargo test --locked live_test_workspace_roundtrip -- --ignored
```

開発時には Chrome に開かれた Test ワークスペースを参考にし、`GPUI Client — Launch` と `GPUI Client — Design` を作成しました。
実 API でのタスク作成・編集・コメント・完了、ネイティブ画面からの期日保存とコメント投稿、ブラウザを経由する OAuth 認可と PAT なしの再起動によるセッション復元を確認しています。
ネイティブ画面ではキーボードからのタスク作成、未保存編集の保護・破棄、不正な期日の拒否、Inbox のアーカイブも確認しています。
OAuth と PAT の両方のサインアウト、ウインドウを閉じる操作による正常終了も確認しています。
