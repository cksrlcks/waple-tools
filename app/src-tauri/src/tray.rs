//! 창을 닫으면 끝내지 않고 트레이(알림 영역)로 숨기기. 설정 `close_to_tray`가 켜졌을 때만.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Window, WindowEvent};

const TRAY_ID: &str = "main";

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 트레이 아이콘을 처음 숨길 때 만든다 (켜 두지 않은 사람에게는 아이콘이 생기지 않는다)
fn ensure_tray(app: &AppHandle) -> tauri::Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }
    let open = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("LocalTools")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// 메인 창 닫기 요청: 설정이 켜져 있으면 숨기고 트레이에 남긴다
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != "main" {
        return;
    }
    let settings = localtools_builtin::load_settings(&localtools_builtin::settings_path());
    // 트레이를 못 만들면 숨기지 않고 그냥 닫는다 (다시 열 길이 없어지므로)
    if settings.close_to_tray && ensure_tray(window.app_handle()).is_ok() {
        api.prevent_close();
        let _ = window.hide();
    }
}
