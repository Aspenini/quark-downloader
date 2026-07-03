//! Native Win32 progress window. A 50 ms timer drains the update channel and
//! drives the labels and progress bar while the message loop runs.

use std::cell::Cell;
use std::sync::Once;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Controls::{PBM_SETPOS, PBM_SETRANGE32};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetWindowLongPtrW, KillTimer,
    PostQuitMessage, SendMessageW, SetTimer, SetWindowLongPtrW, SetWindowTextW, TranslateMessage,
    BS_PUSHBUTTON, GWLP_USERDATA, MSG, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_TIMER, WS_CAPTION,
    WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

use super::util::{self, ThemePaint};
use crate::event::{ProgressChannel, ProgressUpdate};
use crate::model::ProgressSpec;

const MARGIN: i32 = 16;
const CLIENT_W: i32 = 460;
const CLIENT_H: i32 = 170;
const LINE_H: i32 = 18;
const BAR_H: i32 = 18;

const ID_CANCEL: usize = 2;
const TIMER_ID: usize = 1;

const STYLE: WINDOW_STYLE = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

struct ProgressState {
    channel: ProgressChannel,
    exit_code: Cell<Option<i32>>,
    queue: HWND,
    bar: HWND,
    status: HWND,
    eta: HWND,
    paint: ThemePaint,
}

pub fn run_progress(spec: ProgressSpec, channel: ProgressChannel) -> i32 {
    static CLASS: Once = Once::new();
    let class_name = util::register_class("QuarkGuiProgress", Some(progress_wndproc), &CLASS);
    let font = util::message_font();

    let hwnd = util::top_level(&class_name, &spec.window.title, STYLE, CLIENT_W, CLIENT_H);
    let paint = ThemePaint::new(spec.window.theme, hwnd);

    let inner_w = CLIENT_W - 2 * MARGIN;
    let mut y = MARGIN;
    let queue = util::child(
        "STATIC",
        "",
        WS_CHILD | WS_VISIBLE,
        MARGIN,
        y,
        inner_w,
        LINE_H,
        hwnd,
        0,
        font,
    );
    y += LINE_H + 8;
    let bar = util::child(
        "msctls_progress32",
        "",
        WS_CHILD | WS_VISIBLE,
        MARGIN,
        y,
        inner_w,
        BAR_H,
        hwnd,
        0,
        font,
    );
    y += BAR_H + 8;
    let status = util::child(
        "STATIC",
        &spec.initial_status,
        WS_CHILD | WS_VISIBLE,
        MARGIN,
        y,
        inner_w,
        LINE_H,
        hwnd,
        0,
        font,
    );
    y += LINE_H + 4;
    let eta = util::child(
        "STATIC",
        "",
        WS_CHILD | WS_VISIBLE,
        MARGIN,
        y,
        inner_w,
        LINE_H,
        hwnd,
        0,
        font,
    );
    y += LINE_H + 10;
    util::child(
        "BUTTON",
        "Cancel",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
        CLIENT_W - MARGIN - 90,
        y,
        90,
        28,
        hwnd,
        ID_CANCEL,
        font,
    );

    unsafe { SendMessageW(bar, PBM_SETRANGE32, 0, 1000) };

    let state = Box::new(ProgressState {
        channel: channel.clone(),
        exit_code: Cell::new(None),
        queue,
        bar,
        status,
        eta,
        paint,
    });
    let state_ptr = Box::into_raw(state);
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        SetTimer(hwnd, TIMER_ID, 50, None);
    }

    util::show(hwnd);
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    let code = unsafe { (*state_ptr).exit_code.get() };
    unsafe {
        KillTimer(hwnd, TIMER_ID);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        DestroyWindow(hwnd);
        drop(Box::from_raw(state_ptr));
    }

    if code.is_none() {
        // Cancelled before completion.
        channel.request_cancel();
    }
    code.unwrap_or(1)
}

unsafe extern "system" fn progress_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ProgressState;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &*state_ptr;

    if let Some(result) = state.paint.handle(hwnd, msg, wparam, lparam) {
        return result;
    }

    match msg {
        WM_TIMER if wparam == TIMER_ID => {
            while let Ok(update) = state.channel.updates.try_recv() {
                match update {
                    ProgressUpdate::Percent(p) => {
                        let pos = (p.clamp(0.0, 100.0) * 10.0) as usize;
                        SendMessageW(state.bar, PBM_SETPOS, pos, 0);
                    }
                    ProgressUpdate::Status(s) => set_text(state.status, &s),
                    ProgressUpdate::Eta(e) => {
                        let text = e.map(|x| format!("Time left: {x}")).unwrap_or_default();
                        set_text(state.eta, &text);
                    }
                    ProgressUpdate::Queue(q) => set_text(state.queue, &q),
                    ProgressUpdate::Log(_) => {}
                    ProgressUpdate::Done(c) => {
                        state.exit_code.set(Some(c));
                        PostQuitMessage(0);
                    }
                }
            }
            0
        }
        WM_COMMAND if wparam & 0xFFFF == ID_CANCEL => {
            PostQuitMessage(0);
            0
        }
        WM_CLOSE => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn set_text(hwnd: HWND, text: &str) {
    let text = util::wide(text);
    unsafe { SetWindowTextW(hwnd, text.as_ptr()) };
}
