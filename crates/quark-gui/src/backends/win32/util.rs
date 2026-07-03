//! Shared Win32 plumbing: wide strings, window classes, fonts, theming.

use std::ffi::c_void;
use std::sync::Once;

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreateSolidBrush, DeleteObject, FillRect, GetDC, GetSysColorBrush,
    GetTextExtentPoint32W, ReleaseDC, SelectObject, SetBkMode, SetTextColor, COLOR_BTNFACE, HBRUSH,
    HDC, HFONT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_PROGRESS_CLASS, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX,
};
use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, GetClientRect, LoadCursorW, RegisterClassW, SendMessageW,
    SetWindowPos, SystemParametersInfoW, HMENU, IDC_ARROW, NONCLIENTMETRICSW,
    SPI_GETNONCLIENTMETRICS, SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CTLCOLORBTN,
    WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_SETFONT, WNDCLASSW,
    WNDPROC, WS_CLIPCHILDREN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

use crate::model::Theme;

/// UTF-16, NUL-terminated.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn hinstance() -> *mut c_void {
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

pub fn init_common_controls() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS | ICC_STANDARD_CLASSES,
        };
        unsafe { InitCommonControlsEx(&icc) };
    });
}

/// Register a top-level window class once; returns the class name.
pub fn register_class(name: &'static str, wndproc: WNDPROC, once: &'static Once) -> Vec<u16> {
    let class_name = wide(name);
    once.call_once(|| {
        let class = WNDCLASSW {
            style: 0,
            lpfnWndProc: wndproc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance(),
            hIcon: std::ptr::null_mut(),
            hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
            hbrBackground: (COLOR_BTNFACE + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        unsafe { RegisterClassW(&class) };
    });
    class_name
}

/// The message font from the current non-client metrics (Segoe UI in practice).
pub fn message_font() -> HFONT {
    unsafe {
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            ncm.cbSize,
            &mut ncm as *mut _ as *mut c_void,
            0,
        );
        CreateFontIndirectW(&ncm.lfMessageFont)
    }
}

pub fn set_font(hwnd: HWND, font: HFONT) {
    unsafe { SendMessageW(hwnd, WM_SETFONT, font as usize, 1) };
}

/// Converts the dialog units used by the original Win32 resource into pixels
/// for the active message font.
#[derive(Clone, Copy)]
pub struct DialogUnits {
    base_x: i32,
    base_y: i32,
}

impl DialogUnits {
    pub fn from_font(font: HFONT) -> Self {
        const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let text = wide(ALPHABET);
        unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let previous = SelectObject(dc, font);
            let mut extent = SIZE { cx: 0, cy: 0 };
            GetTextExtentPoint32W(dc, text.as_ptr(), ALPHABET.len() as i32, &mut extent);
            SelectObject(dc, previous);
            ReleaseDC(std::ptr::null_mut(), dc);
            Self {
                // This is the algorithm used by Windows for DS_SETFONT dialog
                // templates (the alphabet contains two copies of 26 letters).
                base_x: (((extent.cx / 26) + 1) / 2).max(4),
                base_y: extent.cy.max(8),
            }
        }
    }

    pub fn x(self, value: i32) -> i32 {
        value * self.base_x / 4
    }

    pub fn y(self, value: i32) -> i32 {
        value * self.base_y / 8
    }
}

/// Create a child control with the message font applied.
#[allow(clippy::too_many_arguments)]
pub fn child(
    class: &str,
    text: &str,
    style: WINDOW_STYLE,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: HWND,
    id: usize,
    font: HFONT,
) -> HWND {
    let class = wide(class);
    let text = wide(text);
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            w,
            h,
            parent,
            id as HMENU,
            hinstance(),
            std::ptr::null(),
        )
    };
    set_font(hwnd, font);
    hwnd
}

/// Create a hidden top-level window sized so its *client* area is `w`×`h`,
/// centered on the primary monitor.
pub fn top_level(class_name: &[u16], title: &str, style: WINDOW_STYLE, w: i32, h: i32) -> HWND {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    unsafe { AdjustWindowRectEx(&mut rect, style, 0, WINDOW_EX_STYLE::default()) };
    let ww = rect.right - rect.left;
    let wh = rect.bottom - rect.top;
    let (sx, sy) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let title = wide(title);
    unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            style | WS_CLIPCHILDREN,
            (sx - ww) / 2,
            (sy - wh) / 2,
            ww,
            wh,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance(),
            std::ptr::null(),
        )
    }
}

pub fn show(hwnd: HWND) {
    unsafe { ShowWindow(hwnd, SW_SHOW) };
}

/// Resize a window's client area (used after computing final layout height).
pub fn resize_client(hwnd: HWND, style: WINDOW_STYLE, w: i32, h: i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    unsafe {
        AdjustWindowRectEx(&mut rect, style, 0, WINDOW_EX_STYLE::default());
        let (sx, sy) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let ww = rect.right - rect.left;
        let wh = rect.bottom - rect.top;
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            (sx - ww) / 2,
            (sy - wh) / 2,
            ww,
            wh,
            SWP_NOZORDER,
        );
    }
}

// ---- theming ---------------------------------------------------------------

const DARK_BG: COLORREF = rgb(30, 30, 30);
const DARK_FIELD_BG: COLORREF = rgb(45, 45, 45);
const DARK_TEXT: COLORREF = rgb(240, 240, 240);

const fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    r | (g << 8) | (b << 16)
}

/// Per-window theme resources. Light mode uses system defaults.
pub struct ThemePaint {
    dark: bool,
    bg: HBRUSH,
    field_bg: HBRUSH,
}

impl ThemePaint {
    pub fn new(theme: Theme, hwnd: HWND) -> Self {
        let dark = theme.is_dark();
        if dark {
            // Dark titlebar (best effort; ignored on older Windows).
            let value: i32 = 1;
            unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    &value as *const _ as *const c_void,
                    std::mem::size_of::<i32>() as u32,
                );
            }
        }
        ThemePaint {
            dark,
            bg: if dark {
                unsafe { CreateSolidBrush(DARK_BG) }
            } else {
                unsafe { GetSysColorBrush(COLOR_BTNFACE) }
            },
            field_bg: if dark {
                unsafe { CreateSolidBrush(DARK_FIELD_BG) }
            } else {
                std::ptr::null_mut()
            },
        }
    }

    /// Handle WM_CTLCOLOR*/WM_ERASEBKGND for themed drawing. Returns the
    /// LRESULT to return from the wndproc, or `None` to fall through.
    pub fn handle(&self, hwnd: HWND, msg: u32, wparam: WPARAM, _lparam: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_ERASEBKGND if self.dark => {
                let hdc = wparam as HDC;
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                unsafe {
                    GetClientRect(hwnd, &mut rect);
                    FillRect(hdc, &rect, self.bg);
                }
                Some(1)
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                let hdc = wparam as HDC;
                unsafe {
                    if self.dark {
                        SetTextColor(hdc, DARK_TEXT);
                    }
                    SetBkMode(hdc, TRANSPARENT as i32);
                }
                Some(self.bg as LRESULT)
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX if self.dark => {
                let hdc = wparam as HDC;
                unsafe {
                    SetTextColor(hdc, DARK_TEXT);
                    SetBkMode(hdc, TRANSPARENT as i32);
                }
                Some(self.field_bg as LRESULT)
            }
            _ => None,
        }
    }
}

impl Drop for ThemePaint {
    fn drop(&mut self) {
        if self.dark {
            unsafe {
                DeleteObject(self.bg);
                DeleteObject(self.field_bg);
            }
        }
    }
}

/// Read a control's text.
pub fn window_text(hwnd: HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let read = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..read.max(0) as usize])
}
