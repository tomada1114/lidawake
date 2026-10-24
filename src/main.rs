mod app;
mod audio;
mod auto_mute;
mod command;
mod lid;
mod power;
mod quiet_hours;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::TrayIconEvent;
use tray_icon::menu::MenuEvent;

use app::{App, UserEvent};

/// 実設定を読み直す間隔。
const REFRESH_INTERVAL: Duration = Duration::from_secs(3);

fn main() {
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Dockにアイコンを出さない、メニューバー常駐アプリとして動かす。
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    // トレイとメニューのイベントを、イベントループへ転送する。
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Tray(event));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let proxy = event_loop.create_proxy();
    let mut app: Option<App> = None;

    event_loop.run(move |event, _, control_flow| match event {
        // macOSでは、イベントループが動き始めてからトレイを作る必要がある。
        Event::NewEvents(StartCause::Init) => {
            app = Some(App::new(proxy.clone()));
            *control_flow = ControlFlow::WaitUntil(Instant::now() + REFRESH_INTERVAL);
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            if let Some(app) = app.as_mut() {
                app.refresh();
            }
            *control_flow = ControlFlow::WaitUntil(Instant::now() + REFRESH_INTERVAL);
        }
        Event::UserEvent(event) => {
            if let Some(app) = app.as_mut()
                && app.handle(event)
            {
                // 正常終了（コード0）なら、LaunchAgentは再起動しない。
                *control_flow = ControlFlow::ExitWithCode(0);
            }
        }
        Event::LoopDestroyed => {
            if let Some(app) = app.as_mut() {
                app.shutdown();
            }
        }
        _ => {}
    });
}
