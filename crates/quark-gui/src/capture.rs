use std::cell::RefCell;

thread_local! {
    static BUF: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn start_capture() {
    BUF.with(|b| *b.borrow_mut() = Some(String::new()));
}

pub fn emit_line(line: &str) {
    BUF.with(|b| {
        if let Some(buf) = b.borrow_mut().as_mut() {
            buf.push_str(line);
            if !line.ends_with('\n') {
                buf.push('\n');
            }
        } else {
            println!("{line}");
        }
    });
}

pub fn take_capture() -> String {
    BUF.with(|b| b.borrow_mut().take().unwrap_or_default())
}
