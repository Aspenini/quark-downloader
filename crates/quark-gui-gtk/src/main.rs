//! GTK 4 helper: `--session`, `--progress`, `--message`. No download logic.

fn main() {
    #[cfg(target_os = "linux")]
    linux::main();
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("quark-downloader-gui-gtk is only supported on Linux");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use gtk4::prelude::*;
    use gtk4::{
        Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, DropDown, Entry,
        FileDialog, Label, ListBox, ListBoxRow, Orientation, ProgressBar, ScrolledWindow,
        StringList, StringObject,
    };
    use std::cell::RefCell;
    use std::io::{self, BufRead, Write};
    use std::rc::Rc;

    const APP_ID: &str = "com.aspenini.quark-downloader";
    const AUDIO_FORMATS: &[&str] = &["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"];
    const VIDEO_FORMATS: &[&str] = &["original", "mp4", "mkv", "webm"];
    const SPACES: &[&str] = &["keep", "underscore", "dash", "remove"];
    const MODES: &[&str] = &["progress", "external_cli"];
    const FRONTENDS: &[&str] = &["auto", "gtk", "cosmic", "kirigami"];
    const THEMES: &[&str] = &["light", "dark"];

    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            Some("--message") => {
                if args.len() < 4 {
                    eprintln!("usage: --message <ok|error> <title> <body>");
                    std::process::exit(2);
                }
                run_message(&args[1], &args[2], &args[3..].join(" "));
            }
            Some("--progress") => {
                let theme = args.get(2).map(String::as_str).unwrap_or("light");
                run_progress(theme);
            }
            Some("--session") => run_session(&args[1..]),
            _ => {
                eprintln!("usage: quark-downloader-gui-gtk --session|--progress|--message ...");
                std::process::exit(2);
            }
        }
    }

    fn app_title() -> String {
        match std::env::var("QUARK_VERSION") {
            Ok(v) if !v.is_empty() => format!("Quark Downloader {v}"),
            _ => "Quark Downloader".into(),
        }
    }

    fn apply_theme(theme: &str) {
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(theme.eq_ignore_ascii_case("dark"));
        }
    }

    fn json_escape(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn emit_session(
        action: &str,
        settings: Option<&SessionSettings>,
        urls: &[String],
        media: &str,
        format: &str,
        output: &str,
    ) {
        let mut out = format!("{{\"v\":1,\"action\":{}}", json_escape(action));
        if let Some(s) = settings {
            out.push_str(&format!(
                ",\"settings\":{{\"download_dir\":{},\"yt_dlp\":{},\"ffmpeg\":{},\"gui_download_mode\":{},\"download_logs\":{},\"gui_theme\":{},\"strip_video_ids\":{},\"sanitize_filenames\":{},\"filename_spaces\":{},\"playlist_folders\":{},\"gui_frontend\":{}}}",
                json_escape(&s.download_dir),
                json_escape(&s.yt_dlp),
                json_escape(&s.ffmpeg),
                json_escape(&s.gui_mode),
                s.logs,
                json_escape(&s.theme),
                s.strip_ids,
                s.sanitize,
                json_escape(&s.spaces),
                s.playlist_folders,
                json_escape(&s.frontend),
            ));
        }
        if action == "download" {
            let urls_json = urls
                .iter()
                .map(|u| json_escape(u))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(
                ",\"urls\":[{urls_json}],\"media_type\":{},\"format\":{},\"output_dir\":{}",
                json_escape(media),
                json_escape(format),
                json_escape(output)
            ));
        }
        out.push('}');
        println!("{out}");
        let _ = io::stdout().flush();
    }

    #[derive(Clone)]
    struct SessionSettings {
        download_dir: String,
        yt_dlp: String,
        ffmpeg: String,
        gui_mode: String,
        logs: bool,
        theme: String,
        strip_ids: bool,
        sanitize: bool,
        spaces: String,
        playlist_folders: bool,
        frontend: String,
    }

    fn run_message(kind: &str, title: &str, body: &str) {
        let app = Application::builder().application_id(APP_ID).build();
        let kind = kind.to_string();
        let title = title.to_string();
        let body = body.to_string();
        app.connect_activate(move |app| {
            let dialog = gtk4::AlertDialog::builder()
                .modal(true)
                .message(&title)
                .detail(&body)
                .build();
            if kind == "error" {
                // AlertDialog has no explicit error icon API in all versions; message is enough.
            }
            dialog.show(None::<&gtk4::Window>);
            app.quit();
        });
        app.run_with_args::<&str>(&[]);
    }

    fn run_progress(theme: &str) {
        let app = Application::builder().application_id(APP_ID).build();
        let theme = theme.to_string();
        app.connect_activate(move |app| {
            apply_theme(&theme);
            let window = ApplicationWindow::builder()
                .application(app)
                .title(&app_title())
                .default_width(480)
                .default_height(180)
                .build();
            let vbox = GtkBox::new(Orientation::Vertical, 8);
            vbox.set_margin_top(16);
            vbox.set_margin_bottom(16);
            vbox.set_margin_start(16);
            vbox.set_margin_end(16);
            let queue = Label::new(Some(""));
            queue.set_xalign(0.0);
            let status = Label::new(Some("Starting download..."));
            status.set_xalign(0.0);
            status.set_wrap(true);
            let bar = ProgressBar::new();
            bar.set_fraction(0.0);
            let eta = Label::new(Some("Time left: estimating..."));
            eta.set_xalign(0.0);
            let playlist_eta = Label::new(Some(""));
            playlist_eta.set_xalign(0.0);
            vbox.append(&queue);
            vbox.append(&status);
            vbox.append(&bar);
            vbox.append(&eta);
            vbox.append(&playlist_eta);
            window.set_child(Some(&vbox));
            window.present();

            let (tx, rx) = std::sync::mpsc::channel::<String>();
            std::thread::spawn(move || {
                let stdin = io::stdin();
                for line in stdin.lock().lines().flatten() {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });

            let app_weak = app.downgrade();
            glib_timeout(rx, queue, status, bar, eta, playlist_eta, app_weak);
        });
        app.run_with_args::<&str>(&[]);
    }

    fn glib_timeout(
        rx: std::sync::mpsc::Receiver<String>,
        queue: Label,
        status: Label,
        bar: ProgressBar,
        eta: Label,
        playlist_eta: Label,
        app: gtk4::glib::object::WeakRef<Application>,
    ) {
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(line) = rx.try_recv() {
                let (kind, rest) = line.split_once('\t').unwrap_or((line.as_str(), ""));
                match kind {
                    "PROGRESS" => {
                        if let Ok(p) = rest.parse::<f64>() {
                            bar.set_fraction((p / 100.0).clamp(0.0, 1.0));
                        }
                    }
                    "STATUS" => status.set_text(rest),
                    "ETA" => {
                        if rest.is_empty() {
                            eta.set_text("Time left: estimating...");
                        } else {
                            eta.set_text(&format!("Time left: {rest} left"));
                        }
                    }
                    "QUEUE" => queue.set_text(rest),
                    "DONE" => {
                        if let Some(app) = app.upgrade() {
                            app.quit();
                        }
                        return gtk4::glib::ControlFlow::Break;
                    }
                    _ => {}
                }
                let _ = &playlist_eta;
            }
            gtk4::glib::ControlFlow::Continue
        });
    }

    fn dropdown(values: &[&str], selected: &str) -> DropDown {
        let model = StringList::new(values);
        let dd = DropDown::new(Some(model), gtk4::Expression::NONE);
        if let Some(idx) = values.iter().position(|v| *v == selected) {
            dd.set_selected(idx as u32);
        }
        dd
    }

    fn dropdown_text(dd: &DropDown) -> String {
        dd.selected_item()
            .and_downcast::<StringObject>()
            .map(|o| o.string().to_string())
            .unwrap_or_default()
    }

    fn run_session(args: &[String]) {
        let default_dir = args
            .first()
            .cloned()
            .unwrap_or_else(|| "~/Downloads".into());
        let settings = SessionSettings {
            download_dir: args.get(1).cloned().unwrap_or_else(|| "~/Downloads".into()),
            yt_dlp: args.get(2).cloned().unwrap_or_else(|| "path".into()),
            ffmpeg: args.get(3).cloned().unwrap_or_else(|| "path".into()),
            gui_mode: args.get(4).cloned().unwrap_or_else(|| "progress".into()),
            logs: args.get(5).map(|s| s == "true").unwrap_or(true),
            theme: args.get(6).cloned().unwrap_or_else(|| "light".into()),
            strip_ids: args.get(7).map(|s| s != "false").unwrap_or(true),
            sanitize: args.get(8).map(|s| s != "false").unwrap_or(true),
            spaces: args.get(9).cloned().unwrap_or_else(|| "keep".into()),
            playlist_folders: args.get(10).map(|s| s != "false").unwrap_or(true),
            frontend: args.get(11).cloned().unwrap_or_else(|| "auto".into()),
        };

        let app = Application::builder().application_id(APP_ID).build();
        let settings = Rc::new(RefCell::new(settings));
        let default_dir = Rc::new(default_dir);
        app.connect_activate(move |app| {
            apply_theme(&settings.borrow().theme);
            build_session_ui(app, Rc::clone(&settings), Rc::clone(&default_dir));
        });
        let code = app.run_with_args::<&str>(&[]);
        if code != gtk4::glib::ExitCode::SUCCESS {
            emit_session(
                "cancel",
                Some(&settings.borrow()),
                &[],
                "video",
                "original",
                "",
            );
        }
    }

    fn build_session_ui(
        app: &Application,
        settings: Rc<RefCell<SessionSettings>>,
        default_dir: Rc<String>,
    ) {
        let window = ApplicationWindow::builder()
            .application(app)
            .title(&app_title())
            .default_width(520)
            .default_height(460)
            .build();

        let root = GtkBox::new(Orientation::Vertical, 10);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);

        let url = Entry::new();
        url.set_placeholder_text(Some("https://..."));
        let add = Button::with_label("Add");
        let remove = Button::with_label("Remove");
        let url_row = GtkBox::new(Orientation::Horizontal, 8);
        url_row.append(&url);
        url.set_hexpand(true);
        url_row.append(&add);
        url_row.append(&remove);

        let list = ListBox::new();
        list.set_selection_mode(gtk4::SelectionMode::Single);
        let scroll = ScrolledWindow::builder()
            .min_content_height(120)
            .vexpand(true)
            .child(&list)
            .build();

        let audio = CheckButton::with_label("audio");
        let video = CheckButton::with_label("video");
        video.set_group(Some(&audio));
        video.set_active(true);
        let type_row = GtkBox::new(Orientation::Horizontal, 12);
        type_row.append(&video);
        type_row.append(&audio);

        let format = dropdown(VIDEO_FORMATS, "original");
        let output = Entry::new();
        output.set_text(&*default_dir);
        output.set_hexpand(true);
        let browse = Button::with_label("Browse");
        let out_row = GtkBox::new(Orientation::Horizontal, 8);
        out_row.append(&output);
        out_row.append(&browse);

        let settings_btn = Button::with_label("Settings");
        let download = Button::with_label("Download");
        let cancel = Button::with_label("Cancel");
        let btn_row = GtkBox::new(Orientation::Horizontal, 8);
        btn_row.append(&settings_btn);
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        btn_row.append(&spacer);
        btn_row.append(&cancel);
        btn_row.append(&download);

        root.append(&Label::new(Some("URL")));
        root.append(&url_row);
        root.append(&Label::new(Some("Queue")));
        root.append(&scroll);
        root.append(&type_row);
        root.append(&Label::new(Some("Format")));
        root.append(&format);
        root.append(&Label::new(Some("Output folder")));
        root.append(&out_row);
        root.append(&btn_row);
        window.set_child(Some(&root));

        let list_urls = Rc::new(RefCell::new(Vec::<String>::new()));
        {
            let list = list.clone();
            let list_urls = Rc::clone(&list_urls);
            add.connect_clicked(move |_| {
                let text = url.text().trim().to_string();
                if text.is_empty() {
                    return;
                }
                if list_urls.borrow().iter().any(|u| u == &text) {
                    url.set_text("");
                    return;
                }
                list_urls.borrow_mut().push(text.clone());
                let row = ListBoxRow::new();
                row.set_child(Some(&Label::new(Some(&text))));
                list.append(&row);
                url.set_text("");
            });
        }
        {
            let list = list.clone();
            let list_urls = Rc::clone(&list_urls);
            remove.connect_clicked(move |_| {
                if let Some(row) = list.selected_row() {
                    let idx = row.index();
                    if idx >= 0 {
                        list_urls.borrow_mut().remove(idx as usize);
                        list.remove(&row);
                    }
                }
            });
        }

        let format_c = format.clone();
        video.connect_toggled(move |btn| {
            if btn.is_active() {
                let model = StringList::new(VIDEO_FORMATS);
                format_c.set_model(Some(&model));
                format_c.set_selected(0);
            }
        });
        let format_c = format.clone();
        audio.connect_toggled(move |btn| {
            if btn.is_active() {
                let model = StringList::new(AUDIO_FORMATS);
                format_c.set_model(Some(&model));
                format_c.set_selected(0);
            }
        });

        {
            let window = window.clone();
            let output = output.clone();
            browse.connect_clicked(move |_| {
                let dialog = FileDialog::new();
                let output = output.clone();
                dialog.select_folder(Some(&window), None::<gtk4::gio::Cancellable>, move |res| {
                    if let Ok(file) = res
                        && let Some(path) = file.path()
                    {
                        output.set_text(&path.to_string_lossy());
                    }
                });
            });
        }

        {
            let window = window.clone();
            let settings = Rc::clone(&settings);
            settings_btn.connect_clicked(move |_| {
                show_settings(&window, &settings);
            });
        }

        {
            let app = app.clone();
            let settings = Rc::clone(&settings);
            cancel.connect_clicked(move |_| {
                emit_session(
                    "cancel",
                    Some(&settings.borrow()),
                    &[],
                    "video",
                    "original",
                    "",
                );
                app.quit();
            });
        }

        {
            let app = app.clone();
            let settings = Rc::clone(&settings);
            let list_urls = Rc::clone(&list_urls);
            let output = output.clone();
            let format = format.clone();
            download.connect_clicked(move |_| {
                let urls = list_urls.borrow().clone();
                let out = output.text().to_string();
                if urls.is_empty() || out.trim().is_empty() {
                    return;
                }
                let media = if audio.is_active() { "audio" } else { "video" };
                let fmt = dropdown_text(&format);
                emit_session(
                    "download",
                    Some(&settings.borrow()),
                    &urls,
                    media,
                    &fmt,
                    out.trim(),
                );
                app.quit();
            });
        }

        window.connect_close_request({
            let app = app.clone();
            let settings = Rc::clone(&settings);
            move |_| {
                emit_session(
                    "cancel",
                    Some(&settings.borrow()),
                    &[],
                    "video",
                    "original",
                    "",
                );
                app.quit();
                gtk4::glib::Propagation::Proceed
            }
        });

        window.present();
    }

    fn show_settings(parent: &ApplicationWindow, settings: &Rc<RefCell<SessionSettings>>) {
        let win = ApplicationWindow::builder()
            .transient_for(parent)
            .modal(true)
            .title(&format!("{} Settings", app_title()))
            .default_width(420)
            .default_height(480)
            .build();
        let box_ = GtkBox::new(Orientation::Vertical, 8);
        box_.set_margin_top(16);
        box_.set_margin_bottom(16);
        box_.set_margin_start(16);
        box_.set_margin_end(16);
        let s = settings.borrow().clone();
        let dir = Entry::new();
        dir.set_text(&s.download_dir);
        let browse = Button::with_label("Browse");
        let dir_row = GtkBox::new(Orientation::Horizontal, 8);
        dir.set_hexpand(true);
        dir_row.append(&dir);
        dir_row.append(&browse);
        let strip = CheckButton::with_label("Strip video IDs");
        strip.set_active(s.strip_ids);
        let sanitize = CheckButton::with_label("Sanitize filenames");
        sanitize.set_active(s.sanitize);
        let spaces = dropdown(SPACES, &s.spaces);
        let folders = CheckButton::with_label("Playlist folders");
        folders.set_active(s.playlist_folders);
        let mode = dropdown(MODES, &s.gui_mode);
        let logs = CheckButton::with_label("Download logs");
        logs.set_active(s.logs);
        let theme = dropdown(THEMES, &s.theme);
        let frontend = dropdown(FRONTENDS, &s.frontend);
        let updates = Button::with_label("Check for updates...");
        let save = Button::with_label("Save");
        let cancel = Button::with_label("Cancel");
        let btns = GtkBox::new(Orientation::Horizontal, 8);
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        btns.append(&spacer);
        btns.append(&cancel);
        btns.append(&save);

        box_.append(&Label::new(Some("Default download folder")));
        box_.append(&dir_row);
        box_.append(&strip);
        box_.append(&sanitize);
        box_.append(&Label::new(Some("Filename spaces")));
        box_.append(&spaces);
        box_.append(&folders);
        box_.append(&Label::new(Some("GUI download mode")));
        box_.append(&mode);
        box_.append(&logs);
        box_.append(&Label::new(Some("Theme")));
        box_.append(&theme);
        box_.append(&Label::new(Some("Frontend")));
        box_.append(&frontend);
        box_.append(&updates);
        box_.append(&btns);
        win.set_child(Some(&box_));

        {
            let win = win.clone();
            let dir = dir.clone();
            browse.connect_clicked(move |_| {
                let dialog = FileDialog::new();
                let dir = dir.clone();
                dialog.select_folder(Some(&win), None::<gtk4::gio::Cancellable>, move |res| {
                    if let Ok(file) = res
                        && let Some(path) = file.path()
                    {
                        dir.set_text(&path.to_string_lossy());
                    }
                });
            });
        }
        updates.connect_clicked(|_| {
            if let Some(gui) = quark_gui_path() {
                let _ = std::process::Command::new(gui)
                    .arg("--check-updates")
                    .status();
            }
        });
        {
            let win = win.clone();
            cancel.connect_clicked(move |_| win.close());
        }
        {
            let win = win.clone();
            let settings = Rc::clone(settings);
            save.connect_clicked(move |_| {
                let mut s = settings.borrow_mut();
                s.download_dir = dir.text().to_string();
                s.strip_ids = strip.is_active();
                s.sanitize = sanitize.is_active();
                s.spaces = dropdown_text(&spaces);
                s.playlist_folders = folders.is_active();
                s.gui_mode = dropdown_text(&mode);
                s.logs = logs.is_active();
                s.theme = dropdown_text(&theme);
                s.frontend = dropdown_text(&frontend);
                apply_theme(&s.theme);
                win.close();
            });
        }
        win.present();
    }

    fn quark_gui_path() -> Option<std::path::PathBuf> {
        if let Ok(exe) = std::env::current_exe()
            && let Some(parent) = exe.parent()
        {
            let sibling = parent.join("quark-downloader-gui");
            if sibling.exists() {
                return Some(sibling);
            }
        }
        None
    }
}
