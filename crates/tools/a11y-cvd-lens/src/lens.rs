//! 떠 있는 렌즈 창: Magnification API(배율 1)로 렌즈 아래 화면에 색 행렬을 입힌다.
//!
//! - 렌즈는 프로세스에 하나. 업데이트 뒤에는 예전·새 DLL이 함께 올라가 `static`을 나누지 않으므로
//!   "렌즈가 있나"는 창 클래스 이름 + 프로세스로 찾는다.
//! - 렌즈 스레드가 도는 동안 DLL 참조를 하나 쥐고(`GetModuleHandleExW`), 끝날 때 `FreeLibraryAndExitThread`로 놓는다.
//!   그래서 도구를 제거·업데이트해도 렌즈를 닫을 때까지 코드가 남는다. 스레드는 `std::thread`가 아니라 `CreateThread`.

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, mpsc};

use waple_tools_core::ToolError;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows_sys::Win32::UI::Magnification::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{BOOL, PCWSTR, w};

use crate::matrix::{MODES, effect};

const CLASS: PCWSTR = w!("WapleCvdLens");
const CLASS_NAME: &str = "WapleCvdLens";
/// wparam = 보기 방식 번호. 렌즈를 그 방식으로 바꾸고 앞으로 가져온다
const WM_APP_MODE: u32 = WM_APP + 1;
const TIMER: usize = 1;
const TICK_MS: u32 = 16;
/// 접근성 탭 강조색 `#15803d` (COLORREF는 0x00BBGGRR)
const ACCENT: COLORREF = 0x003D_8015;
const WHITE: COLORREF = 0x00FF_FFFF;

// 96 DPI 기준 크기(px). `px()`로 창의 DPI에 맞춘다
const BAR: i32 = 30;
const EDGE: i32 = 4;
const CORNER: i32 = 16;
const LABEL_W: i32 = 96;
const CLOSE_W: i32 = 56;
const PAD: i32 = 10;
const FONT: i32 = 14;
const START_W: i32 = 520;
const START_H: i32 = 360;

/// 찾기와 새로 띄우기를 묶는다: 동시에 두 번 불려도 렌즈는 하나
static CREATING: Mutex<()> = Mutex::new(());

struct Start {
    mode: usize,
    module: HMODULE,
    ready: mpsc::SyncSender<bool>,
}

/// 호스트 창의 `GWLP_USERDATA`
struct Lens {
    mag: HWND,
    mode: usize,
}

fn failed() -> ToolError {
    ToolError::Failed("렌즈를 띄울 수 없습니다.".into())
}

fn px(v: i32, dpi: u32) -> i32 {
    v * dpi as i32 / 96
}

/// 렌즈가 없으면 띄우고, 있으면 그 보기 방식으로 바꿔 앞으로 가져온다. 바로 돌아온다
pub(crate) fn show(mode: usize) -> Result<(), ToolError> {
    let mode = mode.min(MODES.len() - 1);
    let _guard = CREATING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hwnd) = find() {
        // SAFETY: 같은 프로세스의 렌즈 창. 그 사이 닫혔으면 보내기가 실패할 뿐이다
        unsafe { PostMessageW(hwnd, WM_APP_MODE, mode, 0) };
        return Ok(());
    }
    let mut module: HMODULE = null_mut();
    // SAFETY: 이 함수의 주소로 이 모듈(DLL)을 찾고 참조를 하나 늘린다. 렌즈 스레드가 끝날 때 놓는다
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            show as *const () as PCWSTR,
            &mut module,
        )
    };
    if ok == 0 {
        return Err(failed());
    }
    let (ready, started) = mpsc::sync_channel(1);
    let start = Box::into_raw(Box::new(Start {
        mode,
        module,
        ready,
    }));
    // SAFETY: `start`는 렌즈 스레드가 `Box::from_raw`로 넘겨받는다
    let thread = unsafe {
        CreateThread(
            null(),
            0,
            Some(lens_thread),
            start as *const c_void,
            0,
            null_mut(),
        )
    };
    if thread.is_null() {
        // SAFETY: 스레드가 없으니 넘기려던 값과 늘린 참조를 돌려받는다
        unsafe {
            drop(Box::from_raw(start));
            FreeLibrary(module);
        }
        return Err(failed());
    }
    // SAFETY: 스레드 핸들은 쓰지 않는다 (스레드는 혼자 끝난다)
    unsafe { CloseHandle(thread) };
    match started.recv() {
        Ok(true) => Ok(()),
        _ => Err(failed()),
    }
}

/// 이 프로세스의 렌즈 창
fn find() -> Option<HWND> {
    let mut found: HWND = null_mut();
    // SAFETY: 콜백은 EnumWindows 동안만 `found`를 쓴다
    unsafe { EnumWindows(Some(find_one), &mut found as *mut HWND as LPARAM) };
    (!found.is_null()).then_some(found)
}

unsafe extern "system" fn find_one(hwnd: HWND, out: LPARAM) -> BOOL {
    let mut name = [0u16; 32];
    let mut pid = 0;
    // SAFETY: 버퍼 길이를 넘기고, 창 정보만 읽는다
    let (len, me) = unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
        (
            GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32),
            GetCurrentProcessId(),
        )
    };
    if pid == me && String::from_utf16_lossy(&name[..len.max(0) as usize]) == CLASS_NAME {
        // SAFETY: `find()`가 넘긴 `HWND` 자리
        unsafe { *(out as *mut HWND) = hwnd };
        return 0; // 그만 찾기
    }
    1
}

unsafe extern "system" fn lens_thread(param: *mut c_void) -> u32 {
    // SAFETY: `show()`가 `Box::into_raw`로 넘긴 값
    let start = unsafe { Box::from_raw(param as *mut Start) };
    let module = start.module;
    // SAFETY: 이 스레드에서 렌즈 창을 만들고 메시지 루프를 돈다
    unsafe { run(*start) };
    // SAFETY: `show()`에서 늘린 참조를 놓고 스레드를 끝낸다. 이 뒤로 이 모듈의 코드는 돌지 않는다
    unsafe { FreeLibraryAndExitThread(module, 0) }
}

/// 렌즈 창을 만들고 닫힐 때까지 메시지 루프. Rust 값은 모두 여기서 정리된다
unsafe fn run(start: Start) {
    let Start {
        mode,
        module,
        ready,
    } = start;
    // SAFETY: 이 스레드만의 설정과 Magnification 초기화. 짝을 맞춰 정리한다
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if MagInitialize() == 0 {
            let _ = ready.send(false);
            return;
        }
        let hwnd = create(mode, module);
        let _ = ready.send(!hwnd.is_null());
        drop(ready);
        if !hwnd.is_null() {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        // DLL이 내려가도 클래스는 남는다: 다른 판이 같은 주소에 올라오면 옛 창 프로시저를 부르게 되므로 지운다
        UnregisterClassW(CLASS, module);
        MagUninitialize();
    }
}

/// 호스트 창 + 돋보기 자식 창. 실패하면 null
unsafe fn create(mode: usize, module: HMODULE) -> HWND {
    // SAFETY: Win32 창 만들기. 실패는 null로 확인한다
    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: module,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: CLASS,
            ..std::mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            return null_mut();
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            CLASS,
            w!("색맹 안경"),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            START_W,
            START_H,
            null_mut(),
            null_mut(),
            module,
            null(),
        );
        if hwnd.is_null() {
            return null_mut();
        }
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        let mag = CreateWindowExW(
            0,
            WC_MAGNIFIER,
            w!(""),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            1,
            1,
            hwnd,
            null_mut(),
            module,
            null(),
        );
        if mag.is_null() {
            DestroyWindow(hwnd);
            return null_mut();
        }
        let mut one = MAGTRANSFORM {
            v: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        };
        MagSetWindowTransform(mag, &mut one);
        // 렌즈가 렌즈 자신을 비추지 않게
        let mut me = hwnd;
        MagSetWindowFilterList(mag, MW_FILTERMODE_EXCLUDE, 1, &mut me);
        let lens = Box::into_raw(Box::new(Lens { mag, mode }));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, lens as isize);
        set_mode(hwnd, mode);
        place(hwnd);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetTimer(hwnd, TIMER, TICK_MS, None);
        hwnd
    }
}

/// 커서가 있는 모니터 작업 영역 가운데, 그 모니터 DPI 크기로
unsafe fn place(hwnd: HWND) {
    // SAFETY: 창 위치·크기만 바꾼다
    unsafe {
        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST), &mut mi);
        let r = mi.rcWork;
        SetWindowPos(
            hwnd,
            null_mut(),
            r.left,
            r.top,
            1,
            1,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let dpi = GetDpiForWindow(hwnd);
        let (w, h) = (px(START_W, dpi), px(START_H, dpi));
        let x = r.left + (r.right - r.left - w) / 2;
        let y = r.top + (r.bottom - r.top - h) / 2;
        SetWindowPos(hwnd, null_mut(), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

unsafe fn lens_of(hwnd: HWND) -> Option<&'static mut Lens> {
    // SAFETY: `create()`가 넣고 `WM_NCDESTROY`에서 비우는 포인터. 같은 스레드에서만 쓴다
    unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Lens).as_mut() }
}

unsafe fn set_mode(hwnd: HWND, mode: usize) {
    // SAFETY: 렌즈 스레드에서만 부른다
    unsafe {
        let Some(lens) = lens_of(hwnd) else { return };
        lens.mode = mode.min(MODES.len() - 1);
        let mut e = MAGCOLOREFFECT {
            transform: effect(&MODES[lens.mode].m),
        };
        MagSetColorEffect(lens.mag, &mut e);
        InvalidateRect(hwnd, null(), 0);
    }
}

/// 창 안 좌표(x, y)가 어디인지: 테두리 = 크기 조절, 손잡이 = 이동, 손잡이의 이름·[닫기]와 렌즈 안 = `HTCLIENT`
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BarTarget {
    Menu,
    Close,
}

/// 손잡이에서 누른 곳: 왼쪽 보기 방식 이름 = 메뉴, 오른쪽 = [닫기]
pub(crate) fn bar_target(x: i32, y: i32, w: i32, dpi: u32) -> Option<BarTarget> {
    if y >= px(BAR, dpi) {
        None
    } else if x < px(LABEL_W, dpi) {
        Some(BarTarget::Menu)
    } else if x >= w - px(CLOSE_W, dpi) {
        Some(BarTarget::Close)
    } else {
        None
    }
}

/// 가장 작은 창 크기: 이름과 [닫기] 사이에 끌 자리가 남게
pub(crate) fn min_size(dpi: u32) -> (i32, i32) {
    (px(LABEL_W + CLOSE_W + 48, dpi), px(BAR + EDGE + 60, dpi))
}

pub(crate) fn hit(x: i32, y: i32, w: i32, h: i32, dpi: u32) -> u32 {
    let (e, c) = (px(EDGE, dpi), px(CORNER, dpi));
    let (l, r, t, b) = (x < e, x >= w - e, y < e, y >= h - e);
    let (nl, nr, nt, nb) = (x < c, x >= w - c, y < c, y >= h - c);
    if (t && nl) || (l && nt) {
        HTTOPLEFT
    } else if (t && nr) || (r && nt) {
        HTTOPRIGHT
    } else if (b && nl) || (l && nb) {
        HTBOTTOMLEFT
    } else if (b && nr) || (r && nb) {
        HTBOTTOMRIGHT
    } else if t {
        HTTOP
    } else if b {
        HTBOTTOM
    } else if l {
        HTLEFT
    } else if r {
        HTRIGHT
    } else if y < px(BAR, dpi) && x >= px(LABEL_W, dpi) && x < w - px(CLOSE_W, dpi) {
        HTCAPTION
    } else {
        HTCLIENT
    }
}

unsafe fn client(hwnd: HWND) -> (i32, i32, u32) {
    // SAFETY: 창 크기·DPI 읽기
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut r);
        (r.right, r.bottom, GetDpiForWindow(hwnd))
    }
}

fn lparam_point(lp: LPARAM) -> POINT {
    POINT {
        x: (lp & 0xffff) as i16 as i32,
        y: ((lp >> 16) & 0xffff) as i16 as i32,
    }
}

unsafe fn layout(hwnd: HWND) {
    // SAFETY: 렌즈 스레드에서만 부른다
    unsafe {
        let Some(lens) = lens_of(hwnd) else { return };
        let (w, h, dpi) = client(hwnd);
        let (e, bar) = (px(EDGE, dpi), px(BAR, dpi));
        MoveWindow(
            lens.mag,
            e,
            bar,
            (w - 2 * e).max(1),
            (h - bar - e).max(1),
            1,
        );
    }
}

unsafe fn menu(hwnd: HWND) {
    // SAFETY: 렌즈 스레드에서만 부른다. 메뉴는 만든 뒤 지운다
    unsafe {
        let Some(cur) = lens_of(hwnd).map(|l| l.mode) else {
            return;
        };
        let m = CreatePopupMenu();
        for (i, mode) in MODES.iter().enumerate() {
            let text = wide_z(&format!("{}\t{}", mode.name, i + 1));
            let check = if i == cur { MF_CHECKED } else { MF_UNCHECKED };
            AppendMenuW(m, MF_STRING | check, i + 1, text.as_ptr());
        }
        let (_, _, dpi) = client(hwnd);
        let mut p = POINT {
            x: 0,
            y: px(BAR, dpi),
        };
        ClientToScreen(hwnd, &mut p);
        let cmd = TrackPopupMenu(
            m,
            TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN,
            p.x,
            p.y,
            0,
            hwnd,
            null(),
        );
        DestroyMenu(m);
        if cmd > 0 {
            set_mode(hwnd, cmd as usize - 1);
        }
    }
}

unsafe fn paint(hwnd: HWND) {
    // SAFETY: BeginPaint/EndPaint 짝, 만든 GDI 개체는 지운다
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut ps);
        let (w, h, dpi) = client(hwnd);
        let brush = CreateSolidBrush(ACCENT);
        FillRect(
            dc,
            &RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            },
            brush,
        );
        DeleteObject(brush);
        let font = CreateFontW(
            -px(FONT, dpi),
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            w!("Malgun Gothic"),
        );
        let old = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, WHITE);
        let (bar, pad) = (px(BAR, dpi), px(PAD, dpi));
        let name = lens_of(hwnd).map_or(MODES[0].name, |l| MODES[l.mode].name);
        let label: Vec<u16> = format!("{name} ▾").encode_utf16().collect();
        let mut r = RECT {
            left: pad,
            top: 0,
            right: px(LABEL_W, dpi),
            bottom: bar,
        };
        DrawTextW(
            dc,
            label.as_ptr(),
            label.len() as i32,
            &mut r,
            DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        );
        let close: Vec<u16> = "닫기".encode_utf16().collect();
        let mut r = RECT {
            left: w - px(CLOSE_W, dpi),
            top: 0,
            right: w - pad,
            bottom: bar,
        };
        DrawTextW(
            dc,
            close.as_ptr(),
            close.len() as i32,
            &mut r,
            DT_SINGLELINE | DT_VCENTER | DT_RIGHT | DT_NOPREFIX,
        );
        SelectObject(dc, old);
        DeleteObject(font);
        EndPaint(hwnd, &ps);
    }
}

fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: 렌즈 스레드의 창 프로시저
    unsafe {
        match msg {
            WM_NCHITTEST => {
                let mut p = lparam_point(lp);
                ScreenToClient(hwnd, &mut p);
                let (w, h, dpi) = client(hwnd);
                hit(p.x, p.y, w, h, dpi) as LRESULT
            }
            // 누를 때 처리: 뗄 때 처리하면 메뉴를 닫으려 누른 클릭이 메뉴를 다시 열고,
            // 렌즈 안에서 누른 채 [닫기]에서 떼도 닫힌다
            WM_LBUTTONDOWN => {
                let p = lparam_point(lp);
                let (w, _, dpi) = client(hwnd);
                match bar_target(p.x, p.y, w, dpi) {
                    Some(BarTarget::Menu) => menu(hwnd),
                    Some(BarTarget::Close) => {
                        DestroyWindow(hwnd);
                    }
                    None => {}
                }
                0
            }
            WM_GETMINMAXINFO => {
                let (w, h) = min_size(GetDpiForWindow(hwnd));
                let mmi = &mut *(lp as *mut MINMAXINFO);
                mmi.ptMinTrackSize = POINT { x: w, y: h };
                0
            }
            WM_KEYDOWN => {
                match wp {
                    0x31..=0x34 => set_mode(hwnd, wp - 0x31),
                    k if k == VK_ESCAPE as usize => {
                        DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                0
            }
            WM_APP_MODE => {
                set_mode(hwnd, wp);
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
                0
            }
            WM_SIZE => {
                layout(hwnd);
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_TIMER => {
                if let Some(lens) = lens_of(hwnd) {
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(lens.mag, &mut r);
                    MagSetWindowSource(lens.mag, r);
                    InvalidateRect(lens.mag, null(), 1);
                }
                0
            }
            WM_DPICHANGED => {
                let r = &*(lp as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                0
            }
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                paint(hwnd);
                0
            }
            WM_DESTROY => {
                KillTimer(hwnd, TIMER);
                PostQuitMessage(0);
                0
            }
            WM_NCDESTROY => {
                let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Lens;
                if !p.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(p));
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
