//! メニューバーの表示と操作。状態は `pmset -g` の実設定を正とし、定期的に読み直す。

use std::thread;

use tao::event_loop::EventLoopProxy;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::audio::{self, MuteState};
use crate::power::{self, SleepPermission};
use crate::quiet_hours;

/// イベントループへ送るイベント。トレイ操作とバックグラウンド処理の結果をまとめて扱う。
pub enum UserEvent {
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    ToggleFinished(Result<(), String>),
}

pub struct App {
    tray: TrayIcon,
    status_item: MenuItem,
    toggle_item: MenuItem,
    quit_item: MenuItem,
    awake_icon: Icon,
    sleepy_icon: Icon,
    permission: SleepPermission,
    is_changing: bool,
    last_quiet_hours_decision: Option<bool>,
    did_mute_for_quiet_hours: bool,
    proxy: EventLoopProxy<UserEvent>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let awake_icon = load_icon(include_bytes!("../assets/eye-open.png"));
        let sleepy_icon = load_icon(include_bytes!("../assets/eye-closed.png"));

        let status_item = MenuItem::new("", false, None);
        let toggle_item = MenuItem::new("", false, None);
        let quit_item = MenuItem::new("終了", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status_item,
            &PredefinedMenuItem::separator(),
            &toggle_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .expect("メニューを組み立てられませんでした");

        // 左クリックは切り替え、右クリックでメニューを開く。
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_icon(sleepy_icon.clone())
            .with_icon_as_template(true)
            .build()
            .expect("メニューバーにアイコンを追加できませんでした");

        let mut app = Self {
            tray,
            status_item,
            toggle_item,
            quit_item,
            awake_icon,
            sleepy_icon,
            permission: SleepPermission::Unknown,
            is_changing: false,
            last_quiet_hours_decision: None,
            did_mute_for_quiet_hours: false,
            proxy,
        };
        app.refresh();
        app
    }

    /// イベントを処理する。アプリを終了すべきときは `true` を返す。
    pub fn handle(&mut self, event: UserEvent) -> bool {
        match event {
            UserEvent::Tray(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }) => self.toggle(),
            UserEvent::Tray(_) => {}
            UserEvent::Menu(event) => {
                if event.id == *self.toggle_item.id() {
                    self.toggle();
                } else if event.id == *self.quit_item.id() {
                    return true;
                }
            }
            UserEvent::ToggleFinished(result) => {
                self.is_changing = false;
                self.refresh();
                if let Err(message) = result {
                    show_error(&message);
                }
            }
        }
        false
    }

    pub fn refresh(&mut self) {
        if self.is_changing {
            return;
        }
        self.permission = power::current_permission();
        self.update_appearance();
        self.sync_mute_with_quiet_hours();
    }

    /// 終了前の後始末。自動ミュートしたままなら解除する。
    pub fn shutdown(&mut self) {
        if self.did_mute_for_quiet_hours {
            let _ = audio::set_muted(false);
        }
    }

    fn toggle(&mut self) {
        if self.is_changing {
            return;
        }

        let should_prohibit = match self.permission {
            SleepPermission::Prohibited => false,
            SleepPermission::Allowed => true,
            SleepPermission::Unknown => {
                show_error("現在のスリープ設定を取得できません。");
                return;
            }
        };

        self.is_changing = true;
        self.update_appearance();

        // sudo と pmset の待ち時間でメニューバーを固めないよう、別スレッドで実行する。
        let proxy = self.proxy.clone();
        thread::spawn(move || {
            let result = power::set_sleep_prohibited(should_prohibit);
            let _ = proxy.send_event(UserEvent::ToggleFinished(result));
        });
    }

    fn sync_mute_with_quiet_hours(&mut self) {
        let should_mute = quiet_hours::should_mute_now();
        if Some(should_mute) == self.last_quiet_hours_decision {
            return;
        }

        if should_mute {
            if audio::current_mute_state() == MuteState::Unmuted {
                if let Err(message) = audio::set_muted(true) {
                    eprintln!("ミュートに失敗しました: {message}");
                    return;
                }
                self.did_mute_for_quiet_hours = true;
            }
        } else if self.did_mute_for_quiet_hours {
            if let Err(message) = audio::set_muted(false) {
                eprintln!("ミュート解除に失敗しました: {message}");
                return;
            }
            self.did_mute_for_quiet_hours = false;
        }

        self.last_quiet_hours_decision = Some(should_mute);
    }

    fn update_appearance(&self) {
        if self.is_changing {
            self.tray.set_title(Some("…"));
            let _ = self.tray.set_tooltip(Some("スリープ設定を変更しています"));
            self.toggle_item.set_enabled(false);
            return;
        }

        let (icon, title, tooltip, status, toggle) = match self.permission {
            SleepPermission::Prohibited => (
                Some(self.awake_icon.clone()),
                None,
                "スリープ禁止中。左クリックで通常へ戻します",
                "現在：スリープ禁止",
                "通常へ戻す",
            ),
            SleepPermission::Allowed => (
                Some(self.sleepy_icon.clone()),
                None,
                "スリープ可能。左クリックで起動維持へ切り替えます",
                "現在：スリープ可能",
                "起動維持にする",
            ),
            SleepPermission::Unknown => (
                None,
                Some("? スリープ"),
                "スリープ設定を取得できません",
                "現在：状態を取得できません",
                "切り替え不可",
            ),
        };

        let _ = self.tray.set_icon_with_as_template(icon, true);
        self.tray.set_title(title);
        let _ = self.tray.set_tooltip(Some(tooltip));
        self.status_item.set_text(status);
        self.toggle_item.set_text(toggle);
        self.toggle_item
            .set_enabled(self.permission != SleepPermission::Unknown);
    }
}

fn load_icon(png: &[u8]) -> Icon {
    let image = image::load_from_memory(png)
        .expect("アイコン画像を読み込めませんでした")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("アイコン画像の形式が不正です")
}

/// 警告ダイアログを出す。メニューバーを固めないよう別スレッドの `osascript` で表示する。
fn show_error(message: &str) {
    let message = message.to_string();
    thread::spawn(move || {
        // 文言はスクリプト本文へ埋め込まず、引数（argv）として渡す。
        let _ = crate::command::run(
            "/usr/bin/osascript",
            &[
                "-e",
                "on run argv",
                "-e",
                "display alert (item 1 of argv) message (item 2 of argv) as warning",
                "-e",
                "end run",
                "スリープ設定を変更できませんでした",
                &message,
            ],
        );
    });
}
