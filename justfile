set shell := ["zsh", "-euo", "pipefail", "-c"]

label := "com.tomada1114.lidawake"
app_name := "LidAwake.app"
dist_app := "dist/" + app_name
app_target := env("HOME") + "/Applications/" + app_name
agent_target := env("HOME") + "/Library/LaunchAgents/" + label + ".plist"
helper_target := "/usr/local/libexec/lidawake-helper"
sudoers_target := "/etc/sudoers.d/lidawake"

# レシピ一覧を表示する
default:
    @just --list --unsorted

# 単体テストを実行する
test:
    cargo test

# フォーマットとLintを確認する
lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# その場で起動する（Ctrl+Cで終了）
run:
    cargo run

# dist/LidAwake.app を作る
build:
    cargo build --release
    rm -rf {{dist_app}}
    install -d {{dist_app}}/Contents/MacOS {{dist_app}}/Contents/Resources
    install -m 0755 target/release/lidawake {{dist_app}}/Contents/MacOS/lidawake
    install -m 0644 resources/Info.plist {{dist_app}}/Contents/Info.plist
    codesign --force --sign - {{dist_app}} >/dev/null 2>&1
    @echo "作成しました: {{dist_app}}"

# ~/Applications へ入れ、LaunchAgentで常駐させる（初回は管理者認証あり）
install: build install-helper
    #!/bin/zsh
    set -euo pipefail
    domain="gui/$(id -u)"
    launchctl bootout "${domain}/{{label}}" 2>/dev/null || true
    pkill -x lidawake 2>/dev/null || true

    install -d "${HOME}/Applications" "${HOME}/Library/LaunchAgents"
    rm -rf "{{app_target}}"
    ditto "{{dist_app}}" "{{app_target}}"
    install -m 0644 "resources/{{label}}.plist" "{{agent_target}}"
    /usr/libexec/PlistBuddy -c \
      "Set :ProgramArguments:0 {{app_target}}/Contents/MacOS/lidawake" \
      "{{agent_target}}"

    launchctl bootstrap "${domain}" "{{agent_target}}"
    launchctl kickstart -k "${domain}/{{label}}"
    echo "インストールしました: {{app_target}}"

# 管理者権限ヘルパーとsudoers設定を登録する（登録済みなら何もしない）
install-helper:
    #!/bin/zsh
    set -euo pipefail
    current="$(pmset -g | awk '$1 == "SleepDisabled" { print $2; exit }')"
    if [[ ("${current}" == "0" || "${current}" == "1") && -x "{{helper_target}}" ]] \
      && sudo -n "{{helper_target}}" "${current}" >/dev/null 2>&1; then
      echo "登録済みの管理者権限ヘルパーを利用します。"
      exit 0
    fi

    echo "管理者権限ヘルパーを登録します。"
    sudo install -d -o root -g wheel -m 0755 /usr/local/libexec
    sudo install -o root -g wheel -m 0755 resources/lidawake-helper "{{helper_target}}"
    sudo install -o root -g wheel -m 0440 resources/lidawake.sudoers "{{sudoers_target}}"
    sudo visudo -cf "{{sudoers_target}}"

# スリープを許可状態へ戻してから、アプリ・LaunchAgent・ヘルパー・sudoers設定を削除する
uninstall:
    #!/bin/zsh
    set -euo pipefail
    if [[ -x "{{helper_target}}" ]]; then
      sudo "{{helper_target}}" 0
    fi

    launchctl bootout "gui/$(id -u)/{{label}}" 2>/dev/null || true
    pkill -x lidawake 2>/dev/null || true
    rm -rf "{{app_target}}"
    rm -f "{{agent_target}}"
    sudo rm -f "{{helper_target}}" "{{sudoers_target}}"
    echo "スリープを許可状態へ戻し、LidAwakeを削除しました。"

# インストール済みのアプリを起動する
start:
    #!/bin/zsh
    set -euo pipefail
    domain="gui/$(id -u)"
    launchctl print "${domain}/{{label}}" >/dev/null 2>&1 \
      || launchctl bootstrap "${domain}" "{{agent_target}}"
    launchctl kickstart "${domain}/{{label}}"
    echo "起動しました。"

# インストール済みのアプリを止める（次回ログイン時にはまた起動する）
stop:
    launchctl bootout "gui/$(id -u)/{{label}}" 2>/dev/null || true
    @echo "停止しました。"

# インストール済みのアプリを再起動する
restart:
    launchctl kickstart -k "gui/$(id -u)/{{label}}"

# 常駐状態と現在のスリープ設定を表示する
status:
    #!/bin/zsh
    set -uo pipefail
    if launchctl print "gui/$(id -u)/{{label}}" 2>/dev/null | grep -E '^\s*(state|pid) ='; then
      :
    else
      echo "LaunchAgentは読み込まれていません。"
    fi
    pmset -g | grep SleepDisabled || echo "SleepDisabled の設定は見つかりません。"

# SVGからメニューバー用アイコンのPNGを作り直す
icons:
    for n in eye-open eye-closed; do rsvg-convert -w 36 -h 36 assets/$n.svg -o assets/$n.png; done

# ビルド成果物を削除する
clean:
    cargo clean
    rm -rf dist
