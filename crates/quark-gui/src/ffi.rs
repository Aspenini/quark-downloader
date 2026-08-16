//! C ABI so the Swift AppKit helper can call the same reducer as Rust frontends.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn quark_gui_script(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return to_cstring(r#"{"v":1,"action":"error","message":"null script"}"#);
    }
    let raw = unsafe { CStr::from_ptr(input) };
    let text = match raw.to_str() {
        Ok(s) => s,
        Err(_) => return to_cstring(r#"{"v":1,"action":"error","message":"script is not utf-8"}"#),
    };
    match crate::script::run(text) {
        Ok(out) => to_cstring(&out.to_json()),
        Err(e) => to_cstring(&format!(
            "{{\"v\":1,\"action\":\"error\",\"message\":{}}}",
            quark_core::json::stringify_str(&e)
        )),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn quark_gui_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
