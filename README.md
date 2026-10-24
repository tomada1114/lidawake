# LidAwake

MacBookの強制スリープを許可するかどうかを、macOSのメニューバーで確認・切り替える小さなアプリ。Rust製。

macOS 13以降。

## 表示と操作

- 開いた目: `SleepDisabled 1`。蓋を閉じてもスリープしない（起動維持）。
- 閉じた目: `SleepDisabled 0`。蓋を閉じると通常どおりスリープする。
- 左クリック: 2状態を即時切り替え。
- 右クリック: 現在状態、切り替え、アプリ終了を表示。

状態は3秒ごとに `/usr/bin/pmset -g` から読み直す。アプリの内部状態だけを表示せず、OSの実設定を正とする。

### 自動ミュート

次のどちらかに当てはまる間は、音声出力が鳴っていれば自動でミュートする。

- 夜間: 毎晩21:00〜翌6:00。
- 画面がすべて消えている: 蓋が閉じていて（IOPMrootDomain の `AppleClamshellState`）、外部ディスプレイもつながっていない。起動維持のまま蓋を閉じたときに、ブラウザなどが音を鳴らすのを防ぐ。外部ディスプレイで使うクラムシェルモードではミュートしない。

どちらかが新たに当てはまったときにミュートし、両方とも外れたときに、アプリがミュートした場合に限って解除する。片方が外れても、もう片方が続いていればミュートを保つ。自分で手動ミュートしていた場合は触らない。当てはまっている間に手動で解除した場合は、別の理由が新たに当てはまるまで再びミュートしない。

蓋と外部ディスプレイの状態も3秒ごとに読み直す。読み取れなかったときは前回の判定を引き継ぐ。状態が変わってもメニューバーのアイコンは変えない。

## 必要なもの

- Rust（`rustup` で導入した stable）
- [just](https://github.com/casey/just)（`brew install just` など）

## インストール

```bash
just install
```

初回だけ管理者認証が必要。次の2操作だけをパスワードなしで許可する。

```text
/usr/local/libexec/lidawake-helper 0
/usr/local/libexec/lidawake-helper 1
```

ヘルパーは `root:wheel`、モード `0755` で配置し、一般ユーザーは内容を書き換えられない。`pmset` 全体や任意コマンドを許可しない。

アプリは `~/Applications/LidAwake.app` へ置き、LaunchAgentでログイン時に自動起動する。異常終了した場合は自動で再起動し、メニューの「終了」を選んだ場合は再起動しない。

元の [Sleep Toggle Menu](https://github.com/minorun365/sleep-toggle-menu) を入れている場合は、先にそちらの `./Scripts/uninstall.sh` で削除しておく。

## 起動と停止

```bash
just start    # 起動
just stop     # 停止（次回ログイン時にはまた起動する）
just restart  # 再起動
just status   # 常駐状態と現在のスリープ設定を表示
```両方が同時に動くと、切り替えと自動ミュートが二重になる。

## アンインストール

```bash
just uninstall
```

スリープを許可状態へ戻してから、アプリ、LaunchAgent、ヘルパー、限定したsudoers設定を削除する。

## 注意

`pmset disablesleep` はmacOSの `pmset` マニュアルに記載されていない設定。macOS更新後は、表示と実際の蓋閉じ動作を再確認する。

起動維持のまま鞄やケースへ入れない。蓋を閉じても処理が続くため、発熱とバッテリー消費につながる。移動後はメニューバーの表示を確認して、閉じた目へ戻す。

## 開発

```bash
just          # レシピ一覧
just test     # 判定ロジックの単体テスト
just lint     # フォーマットとLint
just run      # その場で起動（Ctrl+Cで終了）
just build    # dist/LidAwake.app を作る
```

構成:

| パス | 役割 |
|---|---|
| `src/main.rs` | イベントループ（`tao`）。3秒ごとの再読込とイベント転送 |
| `src/app.rs` | メニューバーのアイコンとメニュー（`tray-icon`）、切り替え処理 |
| `src/power.rs` | `pmset -g` の解析とヘルパー呼び出し |
| `src/audio.rs` | `osascript` によるミュート状態の読み書き |
| `src/quiet_hours.rs` | 夜間ミュート区間の判定 |
| `src/lid.rs` | `ioreg` による蓋の開閉と、CoreGraphics による外部ディスプレイの判定 |
| `src/auto_mute.rs` | 自動ミュートの理由を合わせて、ミュート・解除を決める判定 |
| `assets/` | メニューバー用アイコン（SVG原本と36px PNG） |
| `resources/` | Info.plist、LaunchAgent、ヘルパー、sudoers |
| `justfile` | ビルド・インストール・起動停止の手順 |

アイコンを直したら、`just icons` でPNGを作り直す（`rsvg-convert` が必要）。

## 謝辞

このアプリは、[minorun365/sleep-toggle-menu](https://github.com/minorun365/sleep-toggle-menu)（Swift製、Minoru Onda氏）をRustへ移植したものです。ヘルパーとsudoersで権限を最小限に絞る構成、LaunchAgentでの常駐など、設計の多くを元のアプリから受け継いでいます。素敵なアプリをありがとうございます。

## ライセンス

Apache License 2.0。元のアプリの著作権表示は [NOTICE](NOTICE) に記載。
