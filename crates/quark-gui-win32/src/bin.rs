fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--script") => std::process::exit(quark_gui_win32::run_script()),
        Some("-h") | Some("--help") => {
            println!("Usage: quark-downloader-gui-win32 --script");
        }
        _ => {
            eprintln!("usage: quark-downloader-gui-win32 --script");
            std::process::exit(2);
        }
    }
}
