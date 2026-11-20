//! 옵션 패널 별도 창 (`x-panel-position: "window"`): 라벨 `panel` 하나. 내용은 프론트가 이벤트로 주고받는다.

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, Window,
    WindowEvent,
};

const LABEL: &str = "panel";
/// 메인 창과 패널 창 사이 간격 (물리 픽셀)
const GAP: i32 = 8;

/// 위치 계산에 쓰는 값은 모두 물리 픽셀: (x, y) · (폭, 높이)
type Pos = (i32, i32);
type Size = (u32, u32);

fn int(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// 패널 창 위치: 메인 창 오른쪽 옆(간격 `GAP`)에 위를 맞춰 두고, 화면(작업 영역)에 안 들어가면
/// 메인 창 안쪽 오른쪽 아래(메인 제목 막대의 창 버튼을 가리지 않게). 어느 쪽이든 화면 밖으로 나가지 않게 맞춘다.
pub fn panel_position(
    main_pos: Pos,
    main_size: Size,
    monitor_pos: Pos,
    monitor_size: Size,
    panel_size: Size,
) -> Pos {
    let (pw, ph) = (int(panel_size.0), int(panel_size.1));
    let main_right = main_pos.0.saturating_add(int(main_size.0));
    let mon_right = monitor_pos.0.saturating_add(int(monitor_size.0));
    let mon_bottom = monitor_pos.1.saturating_add(int(monitor_size.1));
    let beside = main_right.saturating_add(GAP);
    let (x, y) = if beside.saturating_add(pw) <= mon_right {
        (beside, main_pos.1)
    } else {
        let main_bottom = main_pos.1.saturating_add(int(main_size.1));
        (main_right - pw - GAP, main_bottom - ph - GAP)
    };
    // 패널이 화면보다 크면 왼쪽 위에 붙인다 (max가 나중)
    (
        x.min(mon_right - pw).max(monitor_pos.0),
        y.min(mon_bottom - ph).max(monitor_pos.1),
    )
}

/// 패널 창을 연다. 이미 있으면 제목만 바꾸고 앞으로 (내용은 프론트가 `panel:state`로 보낸다).
/// 창 만들기는 Windows에서 동기 명령이면 멈출 수 있어 async.
#[tauri::command]
pub async fn open_panel_window(app: AppHandle, title: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_title(&title);
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    const FAILED: &str = "창을 열 수 없습니다.";
    let main = app.get_webview_window("main").ok_or(FAILED)?;
    // 자리를 잡은 뒤 보이게 (만들 때는 숨김)
    let w = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html".into()))
        .title(&title)
        .inner_size(340.0, 620.0)
        .min_inner_size(300.0, 480.0)
        .decorations(false)
        .resizable(true)
        .always_on_top(false)
        .visible(false)
        .build()
        .map_err(|_| FAILED)?;
    if let (Ok(pos), Ok(size), Ok(Some(mon)), Ok(panel)) = (
        main.outer_position(),
        main.outer_size(),
        main.current_monitor(),
        w.outer_size(),
    ) {
        let area = mon.work_area();
        let (x, y) = panel_position(
            (pos.x, pos.y),
            (size.width, size.height),
            (area.position.x, area.position.y),
            (area.size.width, area.size.height),
            (panel.width, panel.height),
        );
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    w.show().map_err(|_| FAILED)?;
    Ok(())
}

/// 패널 창을 닫는다 (없으면 아무것도 안 함)
#[tauri::command]
pub fn close_panel_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}

/// 패널 창이 닫히면(X·Alt+F4·명령) 메인에 `panel:closed`. 메인 창을 닫으면(트레이로 숨겨도) 패널도 닫는다
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    match (window.label(), event) {
        (LABEL, WindowEvent::Destroyed) => {
            let _ = window.app_handle().emit_to("main", "panel:closed", ());
        }
        ("main", WindowEvent::CloseRequested { .. }) => {
            close_panel_window(window.app_handle().clone())
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::panel_position;

    const MON: (i32, i32) = (0, 0);
    const MON_SIZE: (u32, u32) = (1920, 1040);
    const PANEL: (u32, u32) = (340, 620);

    #[test]
    fn 자리가_있으면_메인_오른쪽_옆에_위를_맞춘다() {
        assert_eq!(
            panel_position((100, 50), (1200, 800), MON, MON_SIZE, PANEL),
            (1308, 50)
        );
    }

    #[test]
    fn 자리가_없으면_메인_안쪽_오른쪽_아래() {
        // 오른쪽 옆이면 1708 + 340 > 1920 → 안쪽: x = 1700 - 340 - 8, y = 50 + 800 - 620 - 8
        assert_eq!(
            panel_position((500, 50), (1200, 800), MON, MON_SIZE, PANEL),
            (1352, 222)
        );
    }

    #[test]
    fn 화면_밖으로_나가지_않게_맞춘다() {
        // 메인이 화면 위·왼쪽 밖으로 걸쳐 있어도 패널은 화면 안
        assert_eq!(
            panel_position((-300, -100), (400, 500), MON, MON_SIZE, PANEL),
            (108, 0)
        );
        // 최대화(화면 꽉 참): 안쪽 오른쪽 아래
        assert_eq!(
            panel_position(MON, MON_SIZE, MON, MON_SIZE, PANEL),
            (1572, 412)
        );
    }

    #[test]
    fn 둘째_모니터_좌표와_화면보다_큰_패널() {
        // 왼쪽 모니터(음수 좌표)에서도 그 모니터 안에서 계산
        assert_eq!(
            panel_position((-1800, 100), (1200, 800), (-1920, 0), MON_SIZE, PANEL),
            (-592, 100)
        );
        // 패널이 화면보다 크면 화면 왼쪽 위에 붙인다
        assert_eq!(
            panel_position((0, 0), (800, 600), MON, (300, 400), PANEL),
            (0, 0)
        );
    }
}
