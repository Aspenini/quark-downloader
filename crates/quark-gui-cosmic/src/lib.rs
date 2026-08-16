//! COSMIC desktop frontend (iced). Linked into quark-downloader-gui.

pub fn available() -> bool {
    cfg!(target_os = "linux")
}

pub fn invoke(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("--script") => {
            quark_gui::assert_frontend_binds(|event| {
                let _ = event;
            });
            quark_gui::run_script_stdio()
        }
        Some("-h") | Some("--help") => {
            println!("Usage: --session|--progress|--message|--script");
            0
        }
        _ => {
            #[cfg(target_os = "linux")]
            {
                if std::env::var_os("DISPLAY").is_none()
                    && std::env::var_os("WAYLAND_DISPLAY").is_none()
                {
                    eprintln!(
                        "No graphical display (DISPLAY and WAYLAND_DISPLAY are unset).\nOn WSL: use WSLg, or an X server and export DISPLAY."
                    );
                    return 1;
                }
                if std::env::var_os("ICED_BACKEND").is_none() {
                    unsafe {
                        std::env::set_var("ICED_BACKEND", "tiny-skia");
                    }
                }
                linux::run(args.to_vec())
            }
            #[cfg(not(target_os = "linux"))]
            {
                eprintln!("COSMIC visual UI is Linux-only");
                1
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use iced::widget::{
        Space, button, checkbox, column, container, pick_list, progress_bar, radio, row,
        scrollable, text, text_input,
    };
    use iced::{Element, Length, Subscription, Task, Theme};
    use std::io::{BufRead, Write};

    pub fn run(args: Vec<String>) -> i32 {
        match args.first().map(String::as_str) {
            Some("--progress") => run_app(Mode::Progress, args),
            Some("--message") => run_app(Mode::Message, args),
            Some("--session") | _ => run_app(Mode::Session, args),
        }
    }

    #[derive(Clone, Copy)]
    enum Mode {
        Session,
        Progress,
        Message,
    }

    fn run_app(mode: Mode, args: Vec<String>) -> i32 {
        let state = App::from_args(mode, args);
        let theme = if state.theme == "dark" {
            Theme::Dark
        } else {
            Theme::Light
        };
        let result = iced::application(App::title, App::update, App::view)
            .subscription(App::subscription)
            .theme(move |_| theme.clone())
            .window_size(iced::Size::new(520.0, 540.0))
            .run_with(move || (state, Task::none()));
        match result {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("COSMIC UI failed: {e}");
                1
            }
        }
    }

    #[derive(Debug, Clone)]
    enum Message {
        UrlChanged(String),
        Add,
        Paste,
        Remove(usize),
        Audio(bool),
        Format(String),
        Output(String),
        Download,
        Cancel,
        OpenSettings,
        CloseSettings,
        SaveSettings,
        DraftDir(String),
        DraftMode(String),
        DraftTheme(String),
        DraftSpaces(String),
        DraftFrontend(String),
        ToggleLogs(bool),
        ToggleStrip(bool),
        ToggleSanitize(bool),
        ToggleFolders(bool),
        ProgressLine(String),
        Dismiss,
    }

    struct App {
        mode: Mode,
        #[allow(dead_code)]
        default_dir: String,
        #[allow(dead_code)]
        download_dir: String,
        #[allow(dead_code)]
        gui_mode: String,
        #[allow(dead_code)]
        logs: bool,
        theme: String,
        #[allow(dead_code)]
        strip_ids: bool,
        #[allow(dead_code)]
        sanitize: bool,
        #[allow(dead_code)]
        spaces: String,
        #[allow(dead_code)]
        playlist_folders: bool,
        #[allow(dead_code)]
        frontend: String,
        settings_saved: bool,
        show_settings: bool,
        url: String,
        queue: Vec<String>,
        audio: bool,
        format: String,
        output: String,
        status: String,
        eta: String,
        queue_label: String,
        progress: f32,
        msg_title: String,
        msg_body: String,
        draft_dir: String,
        draft_mode: String,
        draft_theme: String,
        draft_spaces: String,
        draft_frontend: String,
        draft_logs: bool,
        draft_strip: bool,
        draft_sanitize: bool,
        draft_folders: bool,
    }

    impl App {
        fn from_args(mode: Mode, args: Vec<String>) -> Self {
            let a = |i: usize, fb: &str| args.get(i).cloned().unwrap_or_else(|| fb.into());
            let b = |i: usize, fb: bool| match args.get(i).map(|s| s.to_ascii_lowercase()) {
                Some(s) => matches!(s.as_str(), "1" | "true" | "yes" | "on"),
                None => fb,
            };
            // args[0] is --session; args[1] is default_dir.
            let default_dir = a(1, "~/Downloads");
            Self {
                mode,
                default_dir: default_dir.clone(),
                download_dir: a(2, "~/Downloads"),
                gui_mode: a(5, "progress"),
                logs: b(6, true),
                theme: a(7, "light"),
                strip_ids: b(8, true),
                sanitize: b(9, true),
                spaces: a(10, "keep"),
                playlist_folders: b(11, true),
                frontend: a(12, "auto"),
                settings_saved: false,
                show_settings: false,
                url: String::new(),
                queue: Vec::new(),
                audio: false,
                format: "original".into(),
                output: if args.get(1).is_some() && args[0] == "--session" {
                    a(1, "~/Downloads")
                } else {
                    default_dir
                },
                status: "Starting download...".into(),
                eta: String::new(),
                queue_label: String::new(),
                progress: 0.0,
                msg_title: a(2, "Quark Downloader"),
                msg_body: args.iter().skip(3).cloned().collect::<Vec<_>>().join(" "),
                draft_dir: a(2, "~/Downloads"),
                draft_mode: a(5, "progress"),
                draft_theme: a(7, "light"),
                draft_spaces: a(10, "keep"),
                draft_frontend: a(12, "auto"),
                draft_logs: b(6, true),
                draft_strip: b(8, true),
                draft_sanitize: b(9, true),
                draft_folders: b(11, true),
            }
        }

        fn title(&self) -> String {
            match std::env::var("QUARK_VERSION") {
                Ok(v) if !v.is_empty() => format!("Quark Downloader {v}"),
                _ => "Quark Downloader".into(),
            }
        }

        fn formats(&self) -> &'static [&'static str] {
            if self.audio {
                quark_gui::AUDIO_FORMATS
            } else {
                quark_gui::VIDEO_FORMATS
            }
        }

        fn settings_json(&self) -> String {
            format!(
                "{{\"download_dir\":{},\"yt_dlp\":\"path\",\"ffmpeg\":\"path\",\"gui_download_mode\":{},\"download_logs\":{},\"gui_theme\":{},\"strip_video_ids\":{},\"sanitize_filenames\":{},\"filename_spaces\":{},\"playlist_folders\":{},\"gui_frontend\":{}}}",
                j(&self.draft_dir),
                j(&self.draft_mode),
                self.draft_logs,
                j(&self.draft_theme),
                self.draft_strip,
                self.draft_sanitize,
                j(&self.draft_spaces),
                self.draft_folders,
                j(&self.draft_frontend),
            )
        }

        fn emit(&self, action: &str, extra: &str) {
            let mut out = format!("{{\"v\":1,\"action\":{action}");
            if self.settings_saved {
                out.push_str(&format!(",\"settings\":{}", self.settings_json()));
            }
            out.push_str(extra);
            out.push('}');
            quark_gui::capture::emit_line(&out);
            let _ = std::io::stdout().flush();
        }

        fn add_url(&mut self) {
            let u = self.url.trim().to_string();
            if u.is_empty() {
                return;
            }
            if !self.queue.iter().any(|x| x == &u) {
                self.queue.push(u);
            }
            self.url.clear();
        }

        fn update(&mut self, message: Message) -> Task<Message> {
            match message {
                Message::UrlChanged(s) => self.url = s,
                Message::Add => self.add_url(),
                Message::Paste => {
                    // iced 0.13 clipboard read is async; treat url field as paste target
                    self.add_url();
                }
                Message::Remove(i) => {
                    if i < self.queue.len() {
                        self.queue.remove(i);
                    }
                }
                Message::Audio(on) => {
                    self.audio = on;
                    self.format = "original".into();
                }
                Message::Format(f) => self.format = f,
                Message::Output(s) => self.output = s,
                Message::Download => {
                    self.add_url();
                    if self.queue.is_empty() || self.output.trim().is_empty() {
                        return Task::none();
                    }
                    let urls: Vec<String> = self
                        .queue
                        .iter()
                        .map(|u| format!("\"{}\"", u.replace('"', "\\\"")))
                        .collect();
                    let extra = format!(
                        ",\"urls\":[{}],\"media_type\":{},\"format\":{},\"output_dir\":{}",
                        urls.join(","),
                        j(if self.audio { "audio" } else { "video" }),
                        j(&self.format),
                        j(self.output.trim())
                    );
                    self.emit("\"download\"", &extra);
                    return iced::exit();
                }
                Message::Cancel => {
                    self.emit("\"cancel\"", "");
                    return iced::exit();
                }
                Message::OpenSettings => self.show_settings = true,
                Message::CloseSettings => self.show_settings = false,
                Message::SaveSettings => {
                    if !self.draft_dir.trim().is_empty() {
                        self.settings_saved = true;
                        self.show_settings = false;
                    }
                }
                Message::DraftDir(s) => self.draft_dir = s,
                Message::DraftMode(s) => self.draft_mode = s,
                Message::DraftTheme(s) => self.draft_theme = s,
                Message::DraftSpaces(s) => self.draft_spaces = s,
                Message::DraftFrontend(s) => self.draft_frontend = s,
                Message::ToggleLogs(v) => self.draft_logs = v,
                Message::ToggleStrip(v) => self.draft_strip = v,
                Message::ToggleSanitize(v) => self.draft_sanitize = v,
                Message::ToggleFolders(v) => self.draft_folders = v,
                Message::ProgressLine(line) => self.apply_progress(&line),
                Message::Dismiss => return iced::exit(),
            }
            Task::none()
        }

        fn apply_progress(&mut self, line: &str) {
            let (kind, rest) = line.split_once('\t').unwrap_or((line, ""));
            match kind {
                "PROGRESS" => {
                    if let Ok(p) = rest.parse::<f32>() {
                        self.progress = (p / 100.0).clamp(0.0, 1.0);
                    }
                }
                "STATUS" => self.status = rest.into(),
                "ETA" => self.eta = rest.into(),
                "QUEUE" => self.queue_label = rest.into(),
                "DONE" => {}
                _ => {}
            }
        }

        fn subscription(&self) -> Subscription<Message> {
            if matches!(self.mode, Mode::Progress) {
                Subscription::run(stdin_lines)
            } else {
                Subscription::none()
            }
        }

        fn view(&self) -> Element<'_, Message> {
            let inner = match self.mode {
                Mode::Message => column![
                    text(&self.msg_title).size(22),
                    text(&self.msg_body),
                    button("OK").on_press(Message::Dismiss),
                ]
                .spacing(12)
                .into(),
                Mode::Progress => column![
                    text(&self.queue_label),
                    text(&self.status),
                    progress_bar(0.0..=1.0, self.progress),
                    text(if self.eta.is_empty() {
                        "Time left: estimating...".into()
                    } else {
                        format!("Time left: {}", self.eta)
                    }),
                ]
                .spacing(10)
                .into(),
                Mode::Session if self.show_settings => self.settings_view(),
                Mode::Session => self.session_view(),
            };
            container(inner)
                .padding(16)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }

        fn session_view(&self) -> Element<'_, Message> {
            let mut queue_col = column![].spacing(4);
            for (i, u) in self.queue.iter().enumerate() {
                queue_col = queue_col.push(row![
                    text(u).width(Length::Fill),
                    button("Remove").on_press(Message::Remove(i)),
                ]);
            }
            let formats: Vec<String> = self.formats().iter().map(|s| (*s).to_string()).collect();
            column![
                text("Video or playlist URL"),
                row![
                    text_input("https://...", &self.url)
                        .on_input(Message::UrlChanged)
                        .on_submit(Message::Add)
                        .width(Length::Fill),
                    button("Add").on_press(Message::Add),
                    button("Paste").on_press(Message::Paste),
                ]
                .spacing(8),
                text("Queue"),
                scrollable(queue_col).height(120),
                row![
                    radio("Video", false, Some(self.audio), Message::Audio),
                    radio("Audio", true, Some(self.audio), Message::Audio),
                ]
                .spacing(12),
                text("Format"),
                pick_list(formats, Some(self.format.clone()), Message::Format),
                text("Output folder"),
                text_input("", &self.output).on_input(Message::Output),
                row![
                    button("Settings").on_press(Message::OpenSettings),
                    Space::with_width(Length::Fill),
                    button("Cancel").on_press(Message::Cancel),
                    button("Download").on_press(Message::Download),
                ]
                .spacing(8),
            ]
            .spacing(8)
            .into()
        }

        fn settings_view(&self) -> Element<'_, Message> {
            let fronts: Vec<String> = quark_gui::supported_frontends()
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            let modes = vec!["progress".into(), "external_cli".into()];
            let themes = vec!["light".into(), "dark".into()];
            let spaces = vec![
                "keep".into(),
                "underscore".into(),
                "dash".into(),
                "remove".into(),
            ];
            column![
                text("Settings").size(22),
                text("Default download folder"),
                text_input("", &self.draft_dir).on_input(Message::DraftDir),
                checkbox("Remove trailing video ID", self.draft_strip)
                    .on_toggle(Message::ToggleStrip),
                checkbox("Sanitize filenames", self.draft_sanitize)
                    .on_toggle(Message::ToggleSanitize),
                text("Filename spaces"),
                pick_list(
                    spaces,
                    Some(self.draft_spaces.clone()),
                    Message::DraftSpaces
                ),
                checkbox("Playlist folders", self.draft_folders).on_toggle(Message::ToggleFolders),
                text("Download window"),
                pick_list(modes, Some(self.draft_mode.clone()), Message::DraftMode),
                checkbox("Download logs", self.draft_logs).on_toggle(Message::ToggleLogs),
                text("Theme"),
                pick_list(themes, Some(self.draft_theme.clone()), Message::DraftTheme),
                text("GUI frontend"),
                pick_list(
                    fronts,
                    Some(self.draft_frontend.clone()),
                    Message::DraftFrontend
                ),
                row![
                    Space::with_width(Length::Fill),
                    button("Cancel").on_press(Message::CloseSettings),
                    button("Save").on_press(Message::SaveSettings),
                ]
                .spacing(8),
            ]
            .spacing(8)
            .into()
        }
    }

    fn j(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn stdin_lines() -> impl iced::futures::Stream<Item = Message> {
        iced::stream::channel(32, |mut sender| async move {
            let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
            std::thread::spawn(move || {
                let stdin = std::io::stdin();
                for line in stdin.lock().lines().flatten() {
                    if tx.unbounded_send(line).is_err() {
                        break;
                    }
                }
            });
            use iced::futures::StreamExt;
            while let Some(line) = rx.next().await {
                if sender.try_send(Message::ProgressLine(line)).is_err() {
                    break;
                }
            }
        })
    }
}
