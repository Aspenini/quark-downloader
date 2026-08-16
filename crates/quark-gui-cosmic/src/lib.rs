//! COSMIC desktop frontend (libcosmic). Linked into quark-downloader-gui.

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
    use cosmic::app::{context_drawer, Core, Settings, Task};
    use cosmic::iced::{Alignment, Length, Size, Subscription};
    use cosmic::prelude::*;
    use cosmic::widget::{self, icon};
    use cosmic::{executor, Element};
    use quark_core::config::{parse_gui_theme, GuiTheme};
    use std::io::{BufRead, Write};

    pub fn run(args: Vec<String>) -> i32 {
        let mode = match args.first().map(String::as_str) {
            Some("--progress") => Mode::Progress,
            Some("--message") => Mode::Message,
            _ => Mode::Session,
        };
        let theme = parse_gui_theme(theme_arg(&args, mode), true);
        let (width, height) = match mode {
            Mode::Progress => (480.0, 220.0),
            Mode::Message => (440.0, 200.0),
            Mode::Session => (560.0, 600.0),
        };
        let settings = Settings::default()
            .size(Size::new(width, height))
            .size_limits(
                cosmic::iced::Limits::NONE
                    .min_width(360.0)
                    .min_height(160.0),
            )
            .theme(cosmic_theme(theme))
            .transparent(true)
            .exit_on_close(true)
            .is_daemon(false);
        match cosmic::app::run::<App>(settings, Flags { mode, args }) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("COSMIC UI failed: {e}");
                1
            }
        }
    }

    fn theme_arg(args: &[String], mode: Mode) -> &str {
        match mode {
            Mode::Progress => args.get(2).map(String::as_str).unwrap_or("system"),
            Mode::Session => args.get(7).map(String::as_str).unwrap_or("system"),
            Mode::Message => "system",
        }
    }

    fn cosmic_theme(theme: GuiTheme) -> cosmic::Theme {
        match theme {
            GuiTheme::Dark => cosmic::theme::system_dark(),
            GuiTheme::Light => cosmic::theme::system_light(),
            GuiTheme::System => cosmic::theme::system_preference(),
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Mode {
        Session,
        Progress,
        Message,
    }

    struct Flags {
        mode: Mode,
        args: Vec<String>,
    }

    #[derive(Debug, Clone)]
    enum Message {
        UrlChanged(String),
        Add,
        Paste,
        Remove(usize),
        Audio(bool),
        Format(usize),
        Output(String),
        Download,
        Cancel,
        OpenSettings,
        CloseSettings,
        SaveSettings,
        DraftDir(String),
        DraftMode(usize),
        DraftTheme(usize),
        DraftSpaces(usize),
        DraftFrontend(usize),
        ToggleLogs(bool),
        ToggleStrip(bool),
        ToggleSanitize(bool),
        ToggleFolders(bool),
        ProgressLine(String),
        Dismiss,
    }

    struct App {
        core: Core,
        mode: Mode,
        theme: String,
        settings_saved: bool,
        emitted: bool,
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
        fn from_flags(core: Core, flags: Flags) -> Self {
            let args = flags.args;
            let a = |i: usize, fb: &str| args.get(i).cloned().unwrap_or_else(|| fb.into());
            let b = |i: usize, fb: bool| match args.get(i).map(|s| s.to_ascii_lowercase()) {
                Some(s) => matches!(s.as_str(), "1" | "true" | "yes" | "on"),
                None => fb,
            };
            let default_dir = a(1, "~/Downloads");
            Self {
                core,
                mode: flags.mode,
                theme: a(7, "system"),
                settings_saved: false,
                emitted: false,
                url: String::new(),
                queue: Vec::new(),
                audio: false,
                format: "original".into(),
                output: if args.get(1).is_some() && args.first().map(String::as_str) == Some("--session")
                {
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
                draft_theme: a(7, "system"),
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

        fn emit(&mut self, action: &str, extra: &str) {
            let mut out = format!("{{\"v\":1,\"action\":{action}");
            if self.settings_saved {
                out.push_str(&format!(",\"settings\":{}", self.settings_json()));
            }
            out.push_str(extra);
            out.push('}');
            quark_gui::capture::emit_line(&out);
            let _ = std::io::stdout().flush();
            self.emitted = true;
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

        fn close(&self) -> Task<Message> {
            match self.core.main_window_id() {
                Some(id) => cosmic::iced::window::close(id),
                None => Task::none(),
            }
        }

        fn apply_title(&mut self) -> Task<Message> {
            let title = self.title();
            self.set_header_title(title.clone());
            match self.core.main_window_id() {
                Some(id) => self.set_window_title(title, id),
                None => Task::none(),
            }
        }

        fn pick(
            items: &'static [&'static str],
            current: &str,
            on_select: impl Fn(usize) -> Message + Send + Sync + 'static,
        ) -> Element<'static, Message> {
            let selected = items.iter().position(|s| *s == current);
            widget::dropdown(items, selected, on_select).into()
        }
    }

    impl cosmic::Application for App {
        type Executor = executor::Default;
        type Flags = Flags;
        type Message = Message;
        const APP_ID: &'static str = "io.github.Aspenini.QuarkDownloader";

        fn core(&self) -> &Core {
            &self.core
        }

        fn core_mut(&mut self) -> &mut Core {
            &mut self.core
        }

        fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
            let mut app = Self::from_flags(core, flags);
            let task = app.apply_title();
            (app, task)
        }

        fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
            if matches!(self.mode, Mode::Session) {
                vec![widget::button::icon(icon::from_name(
                    "preferences-system-symbolic",
                ))
                .on_press(Message::OpenSettings)
                .into()]
            } else {
                Vec::new()
            }
        }

        fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
            if !matches!(self.mode, Mode::Session) || !self.core.window.show_context {
                return None;
            }
            Some(
                context_drawer::context_drawer(self.settings_view(), Message::CloseSettings)
                    .title("Settings"),
            )
        }

        fn on_app_exit(&mut self) -> Option<Self::Message> {
            if matches!(self.mode, Mode::Session) && !self.emitted {
                Some(Message::Cancel)
            } else {
                None
            }
        }

        fn on_escape(&mut self) -> Task<Self::Message> {
            if self.core.window.show_context {
                self.set_show_context(false);
            }
            Task::none()
        }

        fn subscription(&self) -> Subscription<Self::Message> {
            if matches!(self.mode, Mode::Progress) {
                Subscription::run(|| {
                    cosmic::iced::stream::channel(
                        32,
                        |mut sender: futures::channel::mpsc::Sender<Message>| async move {
                            let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
                            std::thread::spawn(move || {
                                let stdin = std::io::stdin();
                                for line in stdin.lock().lines().flatten() {
                                    if tx.unbounded_send(line).is_err() {
                                        break;
                                    }
                                }
                            });
                            use futures::{SinkExt, StreamExt};
                            while let Some(line) = rx.next().await {
                                if sender.send(Message::ProgressLine(line)).await.is_err() {
                                    break;
                                }
                            }
                        },
                    )
                })
            } else {
                Subscription::none()
            }
        }

        fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
            match message {
                Message::UrlChanged(s) => self.url = s,
                Message::Add => self.add_url(),
                Message::Paste => self.add_url(),
                Message::Remove(i) => {
                    if i < self.queue.len() {
                        self.queue.remove(i);
                    }
                }
                Message::Audio(on) => {
                    self.audio = on;
                    self.format = "original".into();
                }
                Message::Format(i) => {
                    if let Some(fmt) = self.formats().get(i) {
                        self.format = (*fmt).into();
                    }
                }
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
                    return self.close();
                }
                Message::Cancel => {
                    if !self.emitted {
                        self.emit("\"cancel\"", "");
                    }
                    return self.close();
                }
                Message::OpenSettings => self.set_show_context(true),
                Message::CloseSettings => self.set_show_context(false),
                Message::SaveSettings => {
                    if !self.draft_dir.trim().is_empty() {
                        self.settings_saved = true;
                        self.theme = self.draft_theme.clone();
                        self.set_show_context(false);
                    }
                }
                Message::DraftDir(s) => self.draft_dir = s,
                Message::DraftMode(i) => {
                    if let Some(v) = quark_gui::MODES.get(i) {
                        self.draft_mode = (*v).into();
                    }
                }
                Message::DraftTheme(i) => {
                    if let Some(v) = quark_gui::THEMES.get(i) {
                        self.draft_theme = (*v).into();
                    }
                }
                Message::DraftSpaces(i) => {
                    if let Some(v) = quark_gui::SPACES.get(i) {
                        self.draft_spaces = (*v).into();
                    }
                }
                Message::DraftFrontend(i) => {
                    if let Some(v) = quark_gui::supported_frontends().get(i) {
                        self.draft_frontend = (*v).into();
                    }
                }
                Message::ToggleLogs(v) => self.draft_logs = v,
                Message::ToggleStrip(v) => self.draft_strip = v,
                Message::ToggleSanitize(v) => self.draft_sanitize = v,
                Message::ToggleFolders(v) => self.draft_folders = v,
                Message::ProgressLine(line) => {
                    let done = line.starts_with("DONE");
                    self.apply_progress(&line);
                    if done {
                        return self.close();
                    }
                }
                Message::Dismiss => return self.close(),
            }
            Task::none()
        }

        fn view(&self) -> Element<'_, Self::Message> {
            let space = cosmic::theme::spacing().space_s;
            let inner: Element<'_, Message> = match self.mode {
                Mode::Message => widget::column::with_capacity(3)
                    .push(widget::text::title3(&self.msg_title))
                    .push(widget::text::body(&self.msg_body))
                    .push(widget::button::suggested("OK").on_press(Message::Dismiss))
                    .spacing(space)
                    .into(),
                Mode::Progress => widget::column::with_capacity(4)
                    .push_maybe((!self.queue_label.is_empty()).then(|| {
                        widget::text::heading(&self.queue_label)
                    }))
                    .push(widget::text::body(&self.status))
                    .push(
                        widget::progress_bar::linear::Linear::new()
                            .progress(self.progress)
                            .width(Length::Fill),
                    )
                    .push(widget::text::caption(if self.eta.is_empty() {
                        "Time left: estimating...".into()
                    } else {
                        format!("Time left: {}", self.eta)
                    }))
                    .spacing(space)
                    .into(),
                Mode::Session => self.session_view(),
            };
            widget::container(inner)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(cosmic::theme::spacing().space_m)
                .into()
        }

        fn footer(&self) -> Option<Element<'_, Self::Message>> {
            if !matches!(self.mode, Mode::Session) {
                return None;
            }
            Some(
                widget::row::with_capacity(2)
                    .push(widget::button::standard("Cancel").on_press(Message::Cancel))
                    .push(widget::Space::new().width(Length::Fill))
                    .push(widget::button::suggested("Download").on_press(Message::Download))
                    .spacing(cosmic::theme::spacing().space_s)
                    .align_y(Alignment::Center)
                    .into(),
            )
        }
    }

    impl App {
        fn session_view(&self) -> Element<'_, Message> {
            let space = cosmic::theme::spacing().space_s;
            let mut queue = widget::column::with_capacity(self.queue.len().max(1)).spacing(4);
            if self.queue.is_empty() {
                queue = queue.push(widget::text::caption("Queue is empty"));
            }
            for (i, u) in self.queue.iter().enumerate() {
                queue = queue.push(
                    widget::row::with_capacity(2)
                        .push(widget::text::body(u).width(Length::Fill))
                        .push(widget::button::text("Remove").on_press(Message::Remove(i)))
                        .align_y(Alignment::Center)
                        .spacing(space),
                );
            }
            widget::column::with_capacity(10)
                .push(widget::text::heading("Video or playlist URL"))
                .push(
                    widget::row::with_capacity(3)
                        .push(
                            widget::text_input("https://...", &self.url)
                                .on_input(Message::UrlChanged)
                                .on_submit(|_| Message::Add)
                                .width(Length::Fill),
                        )
                        .push(widget::button::standard("Add").on_press(Message::Add))
                        .push(widget::button::standard("Paste").on_press(Message::Paste))
                        .spacing(space)
                        .align_y(Alignment::Center),
                )
                .push(widget::text::heading("Queue"))
                .push(widget::scrollable(queue).height(140))
                .push(
                    widget::row::with_capacity(2)
                        .push(widget::radio(
                            "Video",
                            false,
                            Some(self.audio),
                            Message::Audio,
                        ))
                        .push(widget::radio(
                            "Audio",
                            true,
                            Some(self.audio),
                            Message::Audio,
                        ))
                        .spacing(cosmic::theme::spacing().space_m),
                )
                .push(widget::text::heading("Format"))
                .push(Self::pick(
                    self.formats(),
                    &self.format,
                    Message::Format,
                ))
                .push(widget::text::heading("Output folder"))
                .push(
                    widget::text_input("", &self.output)
                        .on_input(Message::Output)
                        .width(Length::Fill),
                )
                .spacing(space)
                .into()
        }

        fn settings_view(&self) -> Element<'_, Message> {
            widget::settings::view_column(vec![
                widget::settings::section()
                    .title("Downloads")
                    .add(widget::settings::item(
                        "Default download folder",
                        widget::text_input("", &self.draft_dir).on_input(Message::DraftDir),
                    ))
                    .add(
                        widget::settings::item::builder("Remove trailing video ID")
                            .toggler(self.draft_strip, Message::ToggleStrip),
                    )
                    .add(
                        widget::settings::item::builder("Sanitize filenames")
                            .toggler(self.draft_sanitize, Message::ToggleSanitize),
                    )
                    .add(widget::settings::item(
                        "Filename spaces",
                        Self::pick(quark_gui::SPACES, &self.draft_spaces, Message::DraftSpaces),
                    ))
                    .add(
                        widget::settings::item::builder("Playlist folders")
                            .toggler(self.draft_folders, Message::ToggleFolders),
                    )
                    .into(),
                widget::settings::section()
                    .title("Interface")
                    .add(widget::settings::item(
                        "Download window",
                        Self::pick(quark_gui::MODES, &self.draft_mode, Message::DraftMode),
                    ))
                    .add(
                        widget::settings::item::builder("Download logs")
                            .toggler(self.draft_logs, Message::ToggleLogs),
                    )
                    .add(widget::settings::item(
                        "Theme",
                        Self::pick(quark_gui::THEMES, &self.draft_theme, Message::DraftTheme),
                    ))
                    .add(widget::settings::item(
                        "GUI frontend",
                        Self::pick(
                            quark_gui::supported_frontends(),
                            &self.draft_frontend,
                            Message::DraftFrontend,
                        ),
                    ))
                    .into(),
                widget::row::with_capacity(2)
                    .push(widget::Space::new().width(Length::Fill))
                    .push(widget::button::standard("Cancel").on_press(Message::CloseSettings))
                    .push(widget::button::suggested("Save").on_press(Message::SaveSettings))
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),
            ])
            .into()
        }
    }

    fn j(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}
