//! 떠 있는 렌즈 창: 안쪽 화면을 캡처해 명도대비를 검사하고 글자·아이콘마다 상자(통과 초록 · 미달 빨강)를 그린다.
//! 검사(OCR 포함)는 띄울 때 · [검사]·Enter · 옮기기·크기 조절을 끝냈을 때만. 그 사이엔 캡처를 비교만 해서
//! 아래 화면이 바뀌면 상자를 지운다.
//!
//! - 픽셀 알파 레이어드 창(`UpdateLayeredWindow`): 안쪽은 알파 0이라 클릭·스크롤이 아래 화면으로 넘어간다.
//! - `WDA_EXCLUDEFROMCAPTURE`: 렌즈 자신(테두리·상자)이 캡처에 찍히지 않는다 (2026-11-22 시험으로 BitBlt에서도 빠짐 확인).
//! - 렌즈는 프로세스에 하나, DLL 참조를 렌즈가 열린 동안만 쥔다: 색맹 안경(`a11y-cvd-lens/src/lens.rs`)과 같은 구조.

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, mpsc};

use waple_tools_core::ToolError;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_F5, VK_RETURN};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{BOOL, PCWSTR, w};

use crate::analyze::{Report, analyze};
use crate::{Crit, fail_label, fmt_ratio, ocr};

const CLASS: PCWSTR = w!("WapleContrastLens");
const CLASS_NAME: &str = "WapleContrastLens";
/// 다시 검사하고 앞으로 가져온다 (이미 떠 있는데 [렌즈 띄우기]를 또 눌렀을 때)
const WM_APP_FRONT: u32 = WM_APP + 1;
const TIMER: usize = 1;
/// 화면이 바뀌었는지 보는 간격 (검사는 하지 않는다)
const TICK_MS: u32 = 500;
/// 접근성 탭 강조색 `#15803d` (COLORREF는 0x00BBGGRR)
const ACCENT: COLORREF = 0x003D_8015;
/// `--destructive` `#c42b1c`
const RED: COLORREF = 0x001C_2BC4;
/// `--success` `#107c10`
const GREEN: COLORREF = 0x0010_7C10;
/// `--muted-foreground` `#6b6b6b` (배경 복잡)
const GRAY: COLORREF = 0x006B_6B6B;
const WHITE: COLORREF = 0x00FF_FFFF;
/// 꺼진 기준 글: 흰색 55% + 강조색 (손잡이 위 흐린 글)
const MUTED: COLORREF = 0x00A8_C696;

// 96 DPI 기준 크기(px). `px()`로 창의 DPI에 맞춘다
const BAR: i32 = 30;
const EDGE: i32 = 4;
const CORNER: i32 = 16;
const CLOSE_W: i32 = 56;
const CHECK_W: i32 = 56;
/// 손잡이 왼쪽 기준 토글 하나의 폭 (네모 + "4.5:1")
const TOGGLE_W: i32 = 72;
const BOX: i32 = 13;
const PAD: i32 = 10;
const FONT: i32 = 14;
const CHIP_FONT: i32 = 12;
const CHIP_H: i32 = 18;
const LINE: i32 = 2;
/// 아이콘 찾기 칸 크기
const TILE: i32 = 24;
const START_W: i32 = 560;
const START_H: i32 = 380;

/// 찾기와 새로 띄우기를 묶는다: 동시에 두 번 불려도 렌즈는 하나
static CREATING: Mutex<()> = Mutex::new(());

struct Start {
    module: HMODULE,
    ready: mpsc::SyncSender<bool>,
}

/// 호스트 창의 `GWLP_USERDATA`
struct Lens {
    crit: Crit,
    /// 끌어 옮기거나 크기를 바꾸는 중: 검사하지 않는다
    moving: bool,
    /// 마지막으로 검사한 캡처. 달라지면 상자를 지운다
    last: Vec<u8>,
    /// 그릴 검사 결과. 검사 중·실패·화면이 바뀐 뒤에는 없음
    report: Option<Report>,
}

fn failed() -> ToolError {
    ToolError::Failed("렌즈를 띄울 수 없습니다.".into())
}

fn px(v: i32, dpi: u32) -> i32 {
    v * dpi as i32 / 96
}

/// 렌즈가 없으면 띄우고, 있으면 다시 검사해 앞으로 가져온다. 바로 돌아온다
pub(crate) fn show() -> Result<(), ToolError> {
    let _guard = CREATING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hwnd) = find() {
        // SAFETY: 같은 프로세스의 렌즈 창. 그 사이 닫혔으면 보내기가 실패할 뿐이다
        unsafe { PostMessageW(hwnd, WM_APP_FRONT, 0, 0) };
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
    let start = Box::into_raw(Box::new(Start { module, ready }));
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
    let Start { module, ready } = start;
    // SAFETY: 이 스레드만의 설정. 창 클래스는 짝을 맞춰 지운다
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hwnd = create(module);
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
    }
}

/// 렌즈 창. 실패하면 null
unsafe fn create(module: HMODULE) -> HWND {
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
            w!("명도대비 렌즈"),
            WS_POPUP,
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
        // 이게 안 되면 렌즈가 자기 상자를 다시 분석한다: 띄우지 않는다
        if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) == 0 {
            DestroyWindow(hwnd);
            return null_mut();
        }
        let lens = Box::into_raw(Box::new(Lens {
            crit: Crit::START,
            moving: false,
            last: Vec::new(),
            report: None,
        }));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, lens as isize);
        place(hwnd);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetTimer(hwnd, TIMER, TICK_MS, None);
        check(hwnd);
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

/// 손잡이의 글자 버튼
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BarButton {
    /// 4.5:1 켜고 끄기
    Text,
    /// 3:1 켜고 끄기
    Large,
    Check,
    Close,
}

/// 손잡이에서 누른 곳: 왼쪽 [4.5:1] [3:1], 오른쪽 끝 [닫기], 그 왼쪽 [검사]
pub(crate) fn bar_button(x: i32, y: i32, w: i32, dpi: u32) -> Option<BarButton> {
    let close = w - px(CLOSE_W, dpi);
    let (pad, t) = (px(PAD, dpi), px(TOGGLE_W, dpi));
    if y >= px(BAR, dpi) {
        None
    } else if (pad..pad + t).contains(&x) {
        Some(BarButton::Text)
    } else if (pad + t..pad + 2 * t).contains(&x) {
        Some(BarButton::Large)
    } else if x >= close {
        Some(BarButton::Close)
    } else if x >= close - px(CHECK_W, dpi) {
        Some(BarButton::Check)
    } else {
        None
    }
}

/// 가장 작은 창 크기: 토글과 [검사] 사이에 끌 자리가 남게
pub(crate) fn min_size(dpi: u32) -> (i32, i32) {
    (
        px(PAD + 2 * TOGGLE_W + 60 + CHECK_W + CLOSE_W, dpi),
        px(BAR + EDGE + 60, dpi),
    )
}

/// 창 안 좌표가 어디인지: 테두리 = 크기 조절, 손잡이 = 이동, [닫기]·상자 = `HTCLIENT`.
/// 안쪽 빈 곳은 알파 0이라 이 메시지가 오지 않고 아래 창으로 넘어간다
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
    } else if y < px(BAR, dpi) && bar_button(x, y, w, dpi).is_none() {
        HTCAPTION
    } else {
        HTCLIENT
    }
}

/// 창 크기(px)와 DPI
unsafe fn size(hwnd: HWND) -> (RECT, i32, i32, u32) {
    // SAFETY: 창 위치·DPI 읽기
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut r);
        (r, r.right - r.left, r.bottom - r.top, GetDpiForWindow(hwnd))
    }
}

fn lparam_point(lp: LPARAM) -> POINT {
    POINT {
        x: (lp & 0xffff) as i16 as i32,
        y: ((lp >> 16) & 0xffff) as i16 as i32,
    }
}

/// 위에서부터 32비트 DIB. 픽셀 포인터와 함께
unsafe fn dib(dc: HDC, w: i32, h: i32) -> (HBITMAP, *mut u8) {
    // SAFETY: 크기를 넘긴 DIB 만들기. 실패하면 null
    unsafe {
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits: *mut c_void = null_mut();
        let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        (bmp, bits as *mut u8)
    }
}

/// 화면의 (x, y, w, h)를 BGRA로. 렌즈 자신은 캡처 제외라 찍히지 않는다
unsafe fn capture(x: i32, y: i32, w: i32, h: i32) -> Option<Vec<u8>> {
    // SAFETY: 만든 GDI 개체는 모두 지운다
    unsafe {
        let screen = GetDC(null_mut());
        let mem = CreateCompatibleDC(screen);
        let (bmp, bits) = dib(screen, w, h);
        let mut out = None;
        if !bmp.is_null() {
            let old = SelectObject(mem, bmp);
            if BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY) != 0 {
                GdiFlush();
                out = Some(std::slice::from_raw_parts(bits, (w * h * 4) as usize).to_vec());
            }
            SelectObject(mem, old);
            DeleteObject(bmp);
        }
        DeleteDC(mem);
        ReleaseDC(null_mut(), screen);
        out
    }
}

/// 안쪽(검사할 곳)의 화면 좌표
unsafe fn inner(hwnd: HWND) -> (i32, i32, i32, i32, u32) {
    // SAFETY: 창 정보 읽기
    let (r, w, h, dpi) = unsafe { size(hwnd) };
    let (e, bar) = (px(EDGE, dpi), px(BAR, dpi));
    (r.left + e, r.top + bar, w - 2 * e, h - bar - e, dpi)
}

/// 안쪽을 캡처해 검사하고 그린다 (글자 영역 검출 포함, 0.1초 남짓)
unsafe fn check(hwnd: HWND) {
    // SAFETY: 렌즈 스레드에서만 부른다
    unsafe {
        let Some(lens) = lens_of(hwnd) else { return };
        if lens.moving {
            return;
        }
        let (x, y, w, h, dpi) = inner(hwnd);
        if w <= 0 || h <= 0 {
            return;
        }
        // 지난 상자를 지우고 검사 (실패하면 상자 없음)
        lens.report = None;
        render(hwnd);
        let Some(shot) = capture(x, y, w, h) else {
            return;
        };
        let (w, h) = (w as usize, h as usize);
        lens.report = ocr::text_boxes(&shot, w, h)
            .ok()
            .map(|text| analyze(&shot, w, h, px(TILE, dpi) as usize, lens.crit, &text));
        lens.last = shot;
        render(hwnd);
    }
}

/// 검사한 뒤 아래 화면이 바뀌었으면(스크롤 등) 상자를 지운다. 검사는 다시 하지 않는다
unsafe fn watch(hwnd: HWND) {
    // SAFETY: 렌즈 스레드에서만 부른다
    unsafe {
        let Some(lens) = lens_of(hwnd) else { return };
        if lens.moving || lens.report.is_none() {
            return;
        }
        let (x, y, w, h, _) = inner(hwnd);
        if capture(x, y, w, h).is_some_and(|shot| shot != lens.last) {
            lens.report = None;
            render(hwnd);
        }
    }
}

unsafe fn fill(dc: HDC, left: i32, top: i32, right: i32, bottom: i32, color: COLORREF) {
    // SAFETY: 만든 붓은 지운다
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(
            dc,
            &RECT {
                left,
                top,
                right,
                bottom,
            },
            brush,
        );
        DeleteObject(brush);
    }
}

unsafe fn font(size: i32) -> HFONT {
    // SAFETY: 글꼴 만들기. 쓴 쪽이 지운다
    unsafe {
        CreateFontW(
            -size,
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
        )
    }
}

unsafe fn text(dc: HDC, s: &str, mut r: RECT, flags: DRAW_TEXT_FORMAT) {
    // 빈 Vec 포인터는 가짜 주소라 DrawTextW(DT_END_ELLIPSIS)가 읽다가 죽는다 (끌기 시작할 때 빈 요약)
    if s.is_empty() {
        return;
    }
    let s: Vec<u16> = s.encode_utf16().collect();
    // SAFETY: 길이를 넘겨 글 그리기
    unsafe {
        DrawTextW(
            dc,
            s.as_ptr(),
            s.len() as i32,
            &mut r,
            flags | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        )
    };
}

/// 테두리 · 손잡이 · 미달 상자를 그려 창에 올린다
unsafe fn render(hwnd: HWND) {
    // SAFETY: 렌즈 스레드에서만 부른다. 만든 GDI 개체는 모두 지운다
    unsafe {
        let Some(lens) = lens_of(hwnd) else { return };
        let (wr, w, h, dpi) = size(hwnd);
        if w <= 0 || h <= 0 {
            return;
        }
        let screen = GetDC(null_mut());
        let mem = CreateCompatibleDC(screen);
        let (bmp, bits) = dib(screen, w, h);
        if bmp.is_null() {
            DeleteDC(mem);
            ReleaseDC(null_mut(), screen);
            return;
        }
        let old = SelectObject(mem, bmp);
        let (e, bar, pad) = (px(EDGE, dpi), px(BAR, dpi), px(PAD, dpi));
        // 손잡이와 테두리 (DIB는 0으로 시작 = 투명)
        fill(mem, 0, 0, w, bar, ACCENT);
        fill(mem, 0, bar, e, h, ACCENT);
        fill(mem, w - e, bar, w, h, ACCENT);
        fill(mem, 0, h - e, w, h, ACCENT);
        SetBkMode(mem, TRANSPARENT as i32);
        SetTextColor(mem, WHITE);
        let big = font(px(FONT, dpi));
        let old_font = SelectObject(mem, big);
        let close_l = w - px(CLOSE_W, dpi);
        let check_l = close_l - px(CHECK_W, dpi);
        let r = |left, right| RECT {
            left,
            top: 0,
            right,
            bottom: bar,
        };
        // 기준 토글: 켜짐 = 흰 네모 안 체크 + 흰 글, 꺼짐 = 빈 네모 + 흐린 글
        let (t, b, one) = (px(TOGGLE_W, dpi), px(BOX, dpi), px(1, dpi).max(1));
        let pen = CreatePen(PS_SOLID, px(2, dpi).max(2), ACCENT);
        let old_pen = SelectObject(mem, pen);
        let c = lens.crit;
        for (i, (label, on)) in [("4.5:1", c.text), ("3:1", c.large)]
            .into_iter()
            .enumerate()
        {
            let (l, top) = (pad + i as i32 * t, (bar - b) / 2);
            let color = if on { WHITE } else { MUTED };
            fill(mem, l, top, l + b, top + b, color);
            if on {
                // 체크: 왼쪽 가운데 → 아래 → 오른쪽 위
                let p = |fx: i32, fy: i32| (l + b * fx / 100, top + b * fy / 100);
                let ((x0, y0), (x1, y1), (x2, y2)) = (p(22, 52), p(42, 72), p(78, 30));
                MoveToEx(mem, x0, y0, null_mut());
                LineTo(mem, x1, y1);
                LineTo(mem, x2, y2);
            } else {
                fill(mem, l + one, top + one, l + b - one, top + b - one, ACCENT);
            }
            SetTextColor(mem, color);
            text(mem, label, r(l + b + px(5, dpi), l + t), 0);
        }
        SelectObject(mem, old_pen);
        DeleteObject(pen);
        SetTextColor(mem, WHITE);
        text(mem, "검사", r(check_l, close_l), DT_CENTER);
        text(mem, "닫기", r(close_l, w - pad), DT_RIGHT);
        // 상자: 통과(초록, 가는 선) 먼저, 미달(빨강)을 위에. 안쪽 좌표 → 창 좌표
        let small = font(px(CHIP_FONT, dpi));
        SelectObject(mem, small);
        let chip_h = px(CHIP_H, dpi);
        let report = if lens.moving {
            None
        } else {
            lens.report.as_ref()
        };
        // 배경 복잡(회색) → 통과(초록) → 미달(빨강) 순서로 위에 쌓는다
        let thin = px(1, dpi).max(1);
        let unclear = report.map_or(&[][..], |r| &r.unclear[..]).iter();
        let passes = report.map_or(&[][..], |r| &r.passes[..]).iter();
        let fails = report.map_or(&[][..], |r| &r.fails[..]).iter();
        let boxes = unclear
            .map(|r| (*r, "배경 복잡".to_string(), GRAY, thin))
            .chain(passes.map(|f| (f.rect, fmt_ratio(f.ratio), GREEN, thin)))
            .chain(fails.map(|f| (f.rect, fail_label(f.ratio), RED, px(LINE, dpi))));
        for (rect, label, color, line) in boxes {
            let (l, t) = (rect.x + e, rect.y + bar);
            let (rr, b) = (l + rect.w, t + rect.h);
            // 상자 바깥쪽에 선 (글자를 가리지 않게)
            let (l, t, rr, b) = (l - line, t - line, rr + line, b + line);
            fill(mem, l, t, rr, t + line, color);
            fill(mem, l, b - line, rr, b, color);
            fill(mem, l, t, l + line, b, color);
            fill(mem, rr - line, t, rr, b, color);
            let wide: Vec<u16> = label.encode_utf16().collect();
            let mut ext = SIZE { cx: 0, cy: 0 };
            GetTextExtentPoint32W(mem, wide.as_ptr(), wide.len() as i32, &mut ext);
            let cw = ext.cx + 2 * px(4, dpi);
            // 상자 위에 자리가 없으면 안쪽 위
            let ct = if t - chip_h >= bar { t - chip_h } else { t };
            let cl = l.min(w - e - cw).max(e);
            fill(mem, cl, ct, cl + cw, ct + chip_h, color);
            text(
                mem,
                &label,
                RECT {
                    left: cl,
                    top: ct,
                    right: cl + cw,
                    bottom: ct + chip_h,
                },
                DT_CENTER,
            );
        }
        SelectObject(mem, old_font);
        DeleteObject(big);
        DeleteObject(small);
        GdiFlush();
        // GDI는 알파를 쓰지 않는다: 그린 곳(검정이 아닌 픽셀)만 불투명으로.
        // shortcut: 순수 검정은 투명이 된다, 검정을 그릴 일이 생기면 알파 마스크를 따로 만든다
        for p in std::slice::from_raw_parts_mut(bits as *mut [u8; 4], (w * h) as usize) {
            p[3] = if p[0] | p[1] | p[2] != 0 { 255 } else { 0 };
        }
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let dst = POINT {
            x: wr.left,
            y: wr.top,
        };
        let src = POINT { x: 0, y: 0 };
        let sz = SIZE { cx: w, cy: h };
        UpdateLayeredWindow(hwnd, screen, &dst, &sz, mem, &src, 0, &blend, ULW_ALPHA);
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(null_mut(), screen);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: 렌즈 스레드의 창 프로시저
    unsafe {
        match msg {
            WM_NCHITTEST => {
                let p = lparam_point(lp);
                let (r, w, h, dpi) = size(hwnd);
                hit(p.x - r.left, p.y - r.top, w, h, dpi) as LRESULT
            }
            WM_LBUTTONDOWN => {
                let p = lparam_point(lp);
                let (_, w, _, dpi) = size(hwnd);
                let toggle = |c: Crit, b: &BarButton| match b {
                    BarButton::Text => c.toggle_text(),
                    _ => c.toggle_large(),
                };
                match bar_button(p.x, p.y, w, dpi) {
                    Some(BarButton::Close) => {
                        DestroyWindow(hwnd);
                    }
                    Some(BarButton::Check) => check(hwnd),
                    Some(b) => {
                        if let Some(lens) = lens_of(hwnd) {
                            lens.crit = toggle(lens.crit, &b);
                        }
                        check(hwnd);
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
                match wp as u16 {
                    VK_ESCAPE => {
                        DestroyWindow(hwnd);
                    }
                    VK_RETURN | VK_F5 => check(hwnd),
                    _ => {}
                }
                0
            }
            WM_APP_FRONT => {
                check(hwnd);
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
                0
            }
            WM_ENTERSIZEMOVE => {
                if let Some(lens) = lens_of(hwnd) {
                    lens.moving = true;
                }
                render(hwnd);
                0
            }
            WM_EXITSIZEMOVE => {
                if let Some(lens) = lens_of(hwnd) {
                    lens.moving = false;
                }
                check(hwnd);
                0
            }
            WM_SIZE => {
                render(hwnd);
                0
            }
            WM_TIMER => {
                watch(hwnd);
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
