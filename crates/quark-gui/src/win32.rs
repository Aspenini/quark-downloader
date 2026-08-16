//! In-process Win32 frontend (dialogs from win32/gui.rc).

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::ptr;
use std::sync::Mutex;
use std::thread;

use quark_core::config::Settings;
use quark_core::progress;
use quark_core::release;
use quark_core::result::DownloadResult;
use quark_core::session::{DownloadParams, MainAction, MainSessionResult, SettingsForm};
use quark_core::version;

use windows_sys::Win32::Foundation::{GetLastError, HWND};
use windows_sys::Win32::System::LibraryLoader::{
    FindResourceExW, FindResourceW, GetModuleHandleW, LoadResource, LockResource,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::UI::Controls::{
    CheckDlgButton, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
    IsDlgButtonChecked, PBM_SETPOS, PBM_SETRANGE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, VK_ESCAPE, VK_RETURN};
use windows_sys::Win32::UI::Shell::{
    BFFM_INITIALIZED, BFFM_SETSELECTIONW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
    SHBrowseForFolderW, SHGetPathFromIDListW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, CB_ADDSTRING, CB_GETCURSEL, CB_GETLBTEXT, CB_RESETCONTENT, CB_SETCURSEL,
    DialogBoxIndirectParamW, DrawMenuBar, EnableMenuItem, EndDialog, GetDlgItem, GetDlgItemTextW,
    GetSystemMenu, IDYES, KillTimer, LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL,
    LB_GETTEXT, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_YESNO, MF_BYCOMMAND, MF_DISABLED,
    MF_GRAYED, MessageBoxW, PostMessageW, RT_DIALOG, SC_CLOSE, SW_HIDE, SW_SHOW, SendMessageW,
    SetDlgItemTextW, SetTimer, SetWindowTextW, ShowWindow, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_INITDIALOG, WM_KEYDOWN, WM_SYSCOMMAND, WM_TIMER,
};
use windows_sys::core::PCWSTR;

type Handle = HWND;

const IDD_MAIN: i32 = 101;
const IDD_PROGRESS: i32 = 102;
const IDC_URL: i32 = 1001;
const IDC_AUDIO: i32 = 1002;
const IDC_VIDEO: i32 = 1003;
const IDC_FORMAT: i32 = 1004;
const IDC_OUTPUT: i32 = 1005;
const IDC_BROWSE: i32 = 1006;
const IDC_PROGRESS_STATUS: i32 = 1007;
const IDC_PROGRESS_BAR: i32 = 1008;
const IDC_SETTINGS: i32 = 1009;
const IDC_SET_DOWNLOAD_DIR: i32 = 1010;
const IDC_SET_BROWSE: i32 = 1011;
const IDC_SET_YTDLP: i32 = 1012;
const IDC_SET_FFMPEG: i32 = 1013;
const IDC_SET_GUI_MODE: i32 = 1014;
const IDC_SET_LOGS: i32 = 1015;
const IDC_SET_SAVE: i32 = 1023;
const IDC_SET_CANCEL: i32 = 1024;
const IDC_CHECK_UPDATES: i32 = 1025;
const IDC_PROGRESS_ETA: i32 = 1026;
const IDC_PROGRESS_QUEUE: i32 = 1027;
const IDC_URL_ADD: i32 = 1028;
const IDC_QUEUE_LIST: i32 = 1029;
const IDC_QUEUE_REMOVE: i32 = 1030;
const IDC_SET_STRIP_IDS: i32 = 1032;
const IDC_SET_SANITIZE: i32 = 1033;
const IDC_SET_SPACES: i32 = 1034;
const IDC_SET_PLAYLIST_FOLDERS: i32 = 1036;
const IDC_PROGRESS_PLAYLIST_ETA: i32 = 1041;

const MAIN_VIEW_IDS: &[i32] = &[
    1016,
    IDC_URL,
    IDC_URL_ADD,
    1031,
    IDC_QUEUE_LIST,
    IDC_QUEUE_REMOVE,
    IDC_AUDIO,
    IDC_VIDEO,
    1017,
    IDC_FORMAT,
    1018,
    IDC_OUTPUT,
    IDC_BROWSE,
    IDC_SETTINGS,
    1,
    2,
];
const SETTINGS_VIEW_IDS: &[i32] = &[
    1037,
    1019,
    IDC_SET_DOWNLOAD_DIR,
    IDC_SET_BROWSE,
    1038,
    IDC_SET_STRIP_IDS,
    IDC_SET_SANITIZE,
    1035,
    IDC_SET_SPACES,
    IDC_SET_PLAYLIST_FOLDERS,
    1039,
    1022,
    IDC_SET_GUI_MODE,
    IDC_SET_LOGS,
    1040,
    1020,
    IDC_SET_YTDLP,
    1021,
    IDC_SET_FFMPEG,
    IDC_CHECK_UPDATES,
    IDC_SET_SAVE,
    IDC_SET_CANCEL,
];

const AUDIO_FORMATS: &[&str] = &["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"];
const VIDEO_FORMATS: &[&str] = &["original", "mp4", "mkv", "webm"];
const TOOL_SOURCE_VALUES: &[&str] = &["auto", "path", "bundled"];
const GUI_MODE_VALUES: &[&str] = &["progress", "external_cli"];
const SPACES_VALUES: &[&str] = &["keep", "underscore", "dash", "remove"];

const WM_APP_DONE: u32 = WM_APP + 1;
const WM_APP_PROGRESS: u32 = WM_APP + 2;
const WM_APP_UPDATE_CHECK_DONE: u32 = WM_APP + 3;
const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 100;
const ETA_UPDATE_MS: u64 = 1500;

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    OsString::from_wide(&buf[..end])
        .to_string_lossy()
        .into_owned()
}

fn int_resource(id: i32) -> PCWSTR {
    id as u16 as usize as PCWSTR
}

fn module_handle() -> Handle {
    unsafe { GetModuleHandleW(ptr::null()) }
}

fn ensure_common_controls() {
    let icc = INITCOMMONCONTROLSEX {
        dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_WIN95_CLASSES,
    };
    unsafe {
        InitCommonControlsEx(&icc);
    }
}

fn load_dialog_template(id: i32) -> Handle {
    unsafe {
        let module = module_handle();
        let name = int_resource(id);
        let mut info = FindResourceExW(module, RT_DIALOG, name, 0);
        if info.is_null() {
            info = FindResourceW(module, name, RT_DIALOG);
        }
        if info.is_null() {
            return ptr::null_mut();
        }
        let data = LoadResource(module, info);
        if data.is_null() {
            return ptr::null_mut();
        }
        LockResource(data)
    }
}

pub fn message_box(text: &str, error: bool) {
    let flags = if error {
        MB_OK | MB_ICONERROR
    } else {
        MB_OK | MB_ICONINFORMATION
    };
    let t = wide(text);
    let c = wide(version::APP_NAME);
    unsafe {
        MessageBoxW(ptr::null_mut(), t.as_ptr(), c.as_ptr(), flags);
    }
}

pub fn confirm_open_url(message: &str, url: &str) -> bool {
    let t = wide(message);
    let c = wide(version::APP_NAME);
    let result = unsafe {
        MessageBoxW(
            ptr::null_mut(),
            t.as_ptr(),
            c.as_ptr(),
            MB_YESNO | MB_ICONINFORMATION,
        )
    };
    if result != IDYES {
        return false;
    }
    open_url(url);
    true
}

pub fn open_url(url: &str) {
    let op = wide("open");
    let file = wide(url);
    unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOW,
        );
    }
}

fn set_dlg_text(dlg: Handle, id: i32, text: &str) {
    let w = wide(text);
    unsafe {
        SetDlgItemTextW(dlg, id, w.as_ptr());
    }
}

fn get_dlg_text(dlg: Handle, id: i32) -> String {
    let mut buf = vec![0u16; 32768];
    let len = unsafe { GetDlgItemTextW(dlg, id, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    from_wide(&buf[..len])
}

fn populate_combo(dlg: Handle, id: i32, values: &[&str], selected: &str) {
    unsafe {
        let combo = GetDlgItem(dlg, id);
        SendMessageW(combo, CB_RESETCONTENT, 0, 0);
        let mut selected_index = 0usize;
        for (index, value) in values.iter().enumerate() {
            let w = wide(value);
            SendMessageW(combo, CB_ADDSTRING, 0, w.as_ptr() as isize);
            if *value == selected {
                selected_index = index;
            }
        }
        SendMessageW(combo, CB_SETCURSEL, selected_index, 0);
    }
}

fn combo_text(dlg: Handle, id: i32) -> String {
    unsafe {
        let combo = GetDlgItem(dlg, id);
        let sel = SendMessageW(combo, CB_GETCURSEL, 0, 0);
        if sel < 0 {
            return String::new();
        }
        let mut buf = vec![0u16; 256];
        SendMessageW(combo, CB_GETLBTEXT, sel as usize, buf.as_mut_ptr() as isize);
        from_wide(&buf)
    }
}

fn listbox_items(dlg: Handle, id: i32) -> Vec<String> {
    unsafe {
        let list = GetDlgItem(dlg, id);
        let count = SendMessageW(list, LB_GETCOUNT, 0, 0);
        if count <= 0 {
            return Vec::new();
        }
        let mut items = Vec::new();
        for index in 0..count {
            let mut buf = vec![0u16; 4096];
            let len = SendMessageW(list, LB_GETTEXT, index as usize, buf.as_mut_ptr() as isize);
            if len >= 0 {
                items.push(from_wide(&buf[..len as usize]));
            }
        }
        items
    }
}

fn set_view_visible(dlg: Handle, ids: &[i32], visible: bool) {
    let cmd = if visible { SW_SHOW } else { SW_HIDE };
    unsafe {
        for id in ids {
            let child = GetDlgItem(dlg, *id);
            if !child.is_null() {
                ShowWindow(child, cmd);
            }
        }
    }
}

fn resource_error(err: u32, stage: &str) -> String {
    format!(
        "Could not open the download dialog ({stage}, Windows error {err}).\n\nFix:\n  1. just clean\n  2. just build\n  3. Run build\\quark-downloader-gui.exe\n\nDo not UPX quark-downloader-gui.exe."
    )
}

struct SessionState {
    default_output: String,
    media_type: String,
    main_action: MainAction,
    dialog_view: View,
    session_settings: Settings,
    session_settings_saved: bool,
    update_check_running: bool,
    browse_initial: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Main,
    Settings,
}

static SESSION: Mutex<Option<SessionState>> = Mutex::new(None);

fn with_session<T>(f: impl FnOnce(&mut SessionState) -> T) -> Option<T> {
    SESSION.lock().ok().and_then(|mut g| g.as_mut().map(f))
}

unsafe extern "system" fn browse_cb(hwnd: Handle, msg: u32, _lparam: isize, _data: isize) -> i32 {
    if msg == BFFM_INITIALIZED
        && let Some(initial) = with_session(|s| s.browse_initial.clone())
        && !initial.is_empty()
    {
        let path = wide(&initial);
        unsafe {
            SendMessageW(hwnd, BFFM_SETSELECTIONW, 1, path.as_ptr() as isize);
        }
    }
    0
}

fn browse_folder_for(dlg: Handle, edit_id: i32, fallback: &str, title: &str) -> Option<String> {
    let current = get_dlg_text(dlg, edit_id);
    let current = current.trim();
    let initial = if current.is_empty() {
        fallback.to_string()
    } else {
        current.to_string()
    };
    with_session(|s| s.browse_initial = initial);
    let title_w = wide(title);
    let mut display = vec![0u16; 260];
    let bi = BROWSEINFOW {
        hwndOwner: dlg,
        pidlRoot: ptr::null_mut(),
        pszDisplayName: display.as_mut_ptr(),
        lpszTitle: title_w.as_ptr(),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        lpfn: Some(browse_cb),
        lParam: 0,
        iImage: 0,
    };
    unsafe {
        let pidl = SHBrowseForFolderW(&bi);
        if pidl.is_null() {
            return None;
        }
        let mut path = vec![0u16; 32768];
        if SHGetPathFromIDListW(pidl, path.as_mut_ptr()) == 0 {
            return None;
        }
        Some(from_wide(&path))
    }
}

fn populate_formats(dlg: Handle) {
    let media = with_session(|s| s.media_type.clone()).unwrap_or_else(|| "video".into());
    let formats = if media == "audio" {
        AUDIO_FORMATS
    } else {
        VIDEO_FORMATS
    };
    populate_combo(dlg, IDC_FORMAT, formats, formats[0]);
}

fn populate_settings_fields(dlg: Handle, settings: &Settings) {
    set_dlg_text(dlg, IDC_SET_DOWNLOAD_DIR, &settings.download_dir);
    populate_combo(
        dlg,
        IDC_SET_YTDLP,
        TOOL_SOURCE_VALUES,
        settings.yt_dlp.as_str(),
    );
    populate_combo(
        dlg,
        IDC_SET_FFMPEG,
        TOOL_SOURCE_VALUES,
        settings.ffmpeg.as_str(),
    );
    populate_combo(
        dlg,
        IDC_SET_GUI_MODE,
        GUI_MODE_VALUES,
        settings.gui_download_mode.as_str(),
    );
    unsafe {
        CheckDlgButton(dlg, IDC_SET_LOGS, u32::from(settings.download_logs));
        CheckDlgButton(dlg, IDC_SET_STRIP_IDS, u32::from(settings.strip_video_ids));
        CheckDlgButton(
            dlg,
            IDC_SET_SANITIZE,
            u32::from(settings.sanitize_filenames),
        );
        CheckDlgButton(
            dlg,
            IDC_SET_PLAYLIST_FOLDERS,
            u32::from(settings.playlist_folders),
        );
    }
    populate_combo(
        dlg,
        IDC_SET_SPACES,
        SPACES_VALUES,
        settings.filename_spaces.as_str(),
    );
}

fn read_settings_form(dlg: Handle, gui_theme: &str) -> Option<SettingsForm> {
    let download_dir = get_dlg_text(dlg, IDC_SET_DOWNLOAD_DIR);
    let download_dir = download_dir.trim();
    if download_dir.is_empty() {
        return None;
    }
    unsafe {
        Some(SettingsForm {
            download_dir: download_dir.to_string(),
            yt_dlp: combo_text(dlg, IDC_SET_YTDLP),
            ffmpeg: combo_text(dlg, IDC_SET_FFMPEG),
            gui_download_mode: combo_text(dlg, IDC_SET_GUI_MODE),
            download_logs: IsDlgButtonChecked(dlg, IDC_SET_LOGS) != 0,
            gui_theme: gui_theme.to_string(),
            strip_video_ids: IsDlgButtonChecked(dlg, IDC_SET_STRIP_IDS) != 0,
            sanitize_filenames: IsDlgButtonChecked(dlg, IDC_SET_SANITIZE) != 0,
            filename_spaces: combo_text(dlg, IDC_SET_SPACES),
            playlist_folders: IsDlgButtonChecked(dlg, IDC_SET_PLAYLIST_FOLDERS) != 0,
            gui_frontend: "auto".into(),
        })
    }
}

fn show_main_view(dlg: Handle) {
    with_session(|s| s.dialog_view = View::Main);
    let title = wide(&version::window_title());
    unsafe {
        SetWindowTextW(dlg, title.as_ptr());
    }
    set_view_visible(dlg, SETTINGS_VIEW_IDS, false);
    set_view_visible(dlg, MAIN_VIEW_IDS, true);
}

fn show_settings_view(dlg: Handle) {
    let settings = with_session(|s| {
        s.dialog_view = View::Settings;
        s.session_settings.clone()
    })
    .unwrap_or_default();
    let title = wide(&version::settings_window_title());
    unsafe {
        SetWindowTextW(dlg, title.as_ptr());
    }
    populate_settings_fields(dlg, &settings);
    set_view_visible(dlg, MAIN_VIEW_IDS, false);
    set_view_visible(dlg, SETTINGS_VIEW_IDS, true);
}

fn add_url_to_queue(dlg: Handle) {
    let url = get_dlg_text(dlg, IDC_URL);
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    let existing = listbox_items(dlg, IDC_QUEUE_LIST);
    if existing.iter().any(|u| u == url) {
        set_dlg_text(dlg, IDC_URL, "");
        return;
    }
    let w = wide(url);
    unsafe {
        let list = GetDlgItem(dlg, IDC_QUEUE_LIST);
        SendMessageW(list, LB_ADDSTRING, 0, w.as_ptr() as isize);
    }
    set_dlg_text(dlg, IDC_URL, "");
}

fn remove_selected_url(dlg: Handle) {
    unsafe {
        let list = GetDlgItem(dlg, IDC_QUEUE_LIST);
        let selected = SendMessageW(list, LB_GETCURSEL, 0, 0);
        if selected >= 0 {
            SendMessageW(list, LB_DELETESTRING, selected as usize, 0);
        }
    }
}

fn try_confirm(dlg: Handle) -> isize {
    add_url_to_queue(dlg);
    let urls = listbox_items(dlg, IDC_QUEUE_LIST);
    if urls.is_empty() {
        message_box("Please enter at least one video or playlist URL.", true);
        return 0;
    }
    let output = get_dlg_text(dlg, IDC_OUTPUT);
    let output = output.trim();
    if output.is_empty() {
        message_box("Please choose an output folder.", true);
        return 0;
    }
    let format = combo_text(dlg, IDC_FORMAT);
    let media_type = unsafe {
        if IsDlgButtonChecked(dlg, IDC_AUDIO) != 0 {
            "audio"
        } else {
            "video"
        }
    };
    with_session(|s| {
        s.media_type = media_type.into();
        s.main_action = MainAction::Download(DownloadParams {
            urls,
            media_type: media_type.into(),
            format: if format.is_empty() {
                "original".into()
            } else {
                format
            },
            output_dir: output.to_string(),
        });
    });
    unsafe {
        EndDialog(dlg, 1);
    }
    1
}

fn try_save_settings(dlg: Handle) -> isize {
    let theme = with_session(|s| s.session_settings.gui_theme.as_str().to_string())
        .unwrap_or_else(|| "light".into());
    let Some(form) = read_settings_form(dlg, &theme) else {
        message_box("Please choose a default download folder.", true);
        return 0;
    };
    with_session(|s| {
        s.session_settings = form.to_settings();
        s.session_settings_saved = true;
    });
    show_main_view(dlg);
    1
}

unsafe extern "system" fn main_dialog_proc(
    dlg: Handle,
    msg: u32,
    wparam: usize,
    _lparam: isize,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            with_session(|s| s.update_check_running = false);
            unsafe {
                CheckDlgButton(dlg, IDC_VIDEO, 1);
            }
            with_session(|s| s.media_type = "video".into());
            populate_formats(dlg);
            if let Some(out) = with_session(|s| s.default_output.clone()) {
                set_dlg_text(dlg, IDC_OUTPUT, &out);
            }
            set_dlg_text(dlg, IDC_SETTINGS, "\u{2699}");
            show_main_view(dlg);
            1
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let notify = ((wparam >> 16) & 0xFFFF) as u32;
            match id {
                IDC_AUDIO if notify == BN_CLICKED => {
                    with_session(|s| s.media_type = "audio".into());
                    populate_formats(dlg);
                }
                IDC_VIDEO if notify == BN_CLICKED => {
                    with_session(|s| s.media_type = "video".into());
                    populate_formats(dlg);
                }
                IDC_URL_ADD if notify == BN_CLICKED => {
                    add_url_to_queue(dlg);
                    return 1;
                }
                IDC_QUEUE_REMOVE if notify == BN_CLICKED => {
                    remove_selected_url(dlg);
                    return 1;
                }
                IDC_BROWSE if notify == BN_CLICKED => {
                    let fallback = with_session(|s| s.default_output.clone()).unwrap_or_default();
                    if let Some(folder) =
                        browse_folder_for(dlg, IDC_OUTPUT, &fallback, "Select output folder")
                    {
                        set_dlg_text(dlg, IDC_OUTPUT, &folder);
                    }
                }
                IDC_SET_BROWSE if notify == BN_CLICKED => {
                    let fallback = with_session(|s| {
                        quark_core::config::expand_path(&s.session_settings.download_dir)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_default();
                    if let Some(folder) = browse_folder_for(
                        dlg,
                        IDC_SET_DOWNLOAD_DIR,
                        &fallback,
                        "Select default download folder",
                    ) {
                        set_dlg_text(dlg, IDC_SET_DOWNLOAD_DIR, &folder);
                    }
                }
                IDC_CHECK_UPDATES if notify == BN_CLICKED => {
                    start_update_check(dlg);
                    return 1;
                }
                IDC_SETTINGS if notify == BN_CLICKED => {
                    show_settings_view(dlg);
                    return 1;
                }
                IDC_SET_SAVE if notify == BN_CLICKED => return try_save_settings(dlg),
                IDC_SET_CANCEL if notify == BN_CLICKED => {
                    show_main_view(dlg);
                    return 1;
                }
                1 if notify == BN_CLICKED => {
                    if with_session(|s| s.dialog_view == View::Main).unwrap_or(true) {
                        return try_confirm(dlg);
                    }
                }
                2 if notify == BN_CLICKED => {
                    if with_session(|s| s.dialog_view == View::Settings).unwrap_or(false) {
                        show_main_view(dlg);
                        return 1;
                    }
                    with_session(|s| s.main_action = MainAction::Cancel);
                    unsafe {
                        EndDialog(dlg, 0);
                    }
                    return 1;
                }
                _ => {}
            }
            0
        }
        WM_APP_UPDATE_CHECK_DONE => {
            with_session(|s| s.update_check_running = false);
            set_dlg_text(dlg, IDC_CHECK_UPDATES, "Check for updates...");
            unsafe {
                EnableWindow(GetDlgItem(dlg, IDC_CHECK_UPDATES), 1);
            }
            1
        }
        WM_KEYDOWN if wparam == VK_RETURN as usize => {
            if with_session(|s| s.dialog_view == View::Settings).unwrap_or(false) {
                try_save_settings(dlg)
            } else {
                try_confirm(dlg)
            }
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            if with_session(|s| s.dialog_view == View::Settings).unwrap_or(false) {
                show_main_view(dlg);
                1
            } else {
                with_session(|s| s.main_action = MainAction::Cancel);
                unsafe {
                    EndDialog(dlg, 0);
                }
                1
            }
        }
        _ => 0,
    }
}

fn start_update_check(dlg: Handle) {
    if with_session(|s| s.update_check_running).unwrap_or(false) {
        return;
    }
    with_session(|s| s.update_check_running = true);
    set_dlg_text(dlg, IDC_CHECK_UPDATES, "Checking...");
    unsafe {
        EnableWindow(GetDlgItem(dlg, IDC_CHECK_UPDATES), 0);
    }
    let dlg = dlg as usize;
    thread::spawn(move || {
        run_update_check_ui();
        unsafe {
            PostMessageW(dlg as Handle, WM_APP_UPDATE_CHECK_DONE, 0, 0);
        }
    });
}

fn run_update_check_ui() {
    let (status, latest, behind, error) = release::check_with_error();
    match status {
        release::Status::UpToDate => {
            message_box(
                &format!("You are up to date ({}).", version::VERSION),
                false,
            );
        }
        release::Status::Ahead => {
            message_box(
                &format!(
                    "You are running {} (newer than the latest release {}).",
                    version::VERSION,
                    latest.unwrap_or_default()
                ),
                false,
            );
        }
        release::Status::Behind => {
            if let Some(info) = behind {
                let message = format!(
                    "A newer version ({}) is available. You have {}.\n\nDownload the latest installer?",
                    info.latest_version,
                    version::VERSION
                );
                confirm_open_url(&message, &info.download_url);
            }
        }
        release::Status::Failed => {
            message_box(
                &format!(
                    "Could not check for updates:\n{}",
                    error.unwrap_or_else(|| "unknown error".into())
                ),
                true,
            );
        }
    }
}

pub fn collect_main_session(
    default_output: &str,
    settings: &Settings,
) -> Result<MainSessionResult, String> {
    {
        let mut guard = SESSION.lock().map_err(|e| e.to_string())?;
        *guard = Some(SessionState {
            default_output: default_output.to_string(),
            media_type: "video".into(),
            main_action: MainAction::Cancel,
            dialog_view: View::Main,
            session_settings: settings.clone(),
            session_settings_saved: false,
            update_check_running: false,
            browse_initial: String::new(),
        });
    }
    ensure_common_controls();
    let template = load_dialog_template(IDD_MAIN);
    if template.is_null() {
        let err = unsafe { GetLastError() };
        message_box(&resource_error(err, "template not found"), true);
        return Ok(MainSessionResult::cancel());
    }
    let result = unsafe {
        DialogBoxIndirectParamW(
            module_handle(),
            template.cast(),
            ptr::null_mut(),
            Some(main_dialog_proc),
            0,
        )
    };
    if result == -1 {
        let err = unsafe { GetLastError() };
        message_box(&resource_error(err, "dialog could not be created"), true);
        return Ok(MainSessionResult::cancel());
    }
    let (action, form) = with_session(|s| {
        (
            s.main_action.clone(),
            if s.session_settings_saved {
                Some(SettingsForm {
                    download_dir: s.session_settings.download_dir.clone(),
                    yt_dlp: s.session_settings.yt_dlp.as_str().into(),
                    ffmpeg: s.session_settings.ffmpeg.as_str().into(),
                    gui_download_mode: s.session_settings.gui_download_mode.as_str().into(),
                    download_logs: s.session_settings.download_logs,
                    gui_theme: s.session_settings.gui_theme.as_str().into(),
                    strip_video_ids: s.session_settings.strip_video_ids,
                    sanitize_filenames: s.session_settings.sanitize_filenames,
                    filename_spaces: s.session_settings.filename_spaces.as_str().into(),
                    playlist_folders: s.session_settings.playlist_folders,
                    gui_frontend: s.session_settings.gui_frontend.as_str().into(),
                })
            } else {
                None
            },
        )
    })
    .unwrap_or((MainAction::Cancel, None));
    Ok(MainSessionResult {
        action,
        settings_form: form,
    })
}

struct ProgressState {
    command: String,
    args: Vec<String>,
    finished: bool,
    exit_code: i32,
    result: Option<DownloadResult>,
    cancelled: bool,
    percent: f64,
    status: String,
    eta: Option<String>,
    display_eta: Option<String>,
    display_started: bool,
    last_eta_ms: u64,
    download_started: bool,
    url_text: String,
    item_text: String,
    last_output_ms: u64,
    playlist_item: Option<i32>,
    playlist_total: Option<i32>,
    playlist_started_ms: u64,
    hwnd: usize,
    runner: Option<quark_core::process::HiddenProcess>,
}

use progress_impl::run_progress_dialog;

mod progress_impl {
    use super::*;

    static STATE: Mutex<Option<ProgressState>> = Mutex::new(None);

    fn with<T>(f: impl FnOnce(&mut ProgressState) -> T) -> Option<T> {
        STATE.lock().ok().and_then(|mut g| g.as_mut().map(f))
    }

    pub fn run_progress_dialog(command: &str, cmd_args: &[String]) -> i32 {
        {
            let now = unsafe { GetTickCount64() };
            let mut guard = STATE.lock().unwrap();
            *guard = Some(ProgressState {
                command: command.to_string(),
                args: cmd_args.to_vec(),
                finished: false,
                exit_code: 1,
                result: None,
                cancelled: false,
                percent: 0.0,
                status: "Starting download...".into(),
                eta: None,
                display_eta: None,
                display_started: false,
                last_eta_ms: 0,
                download_started: false,
                url_text: String::new(),
                item_text: String::new(),
                last_output_ms: now,
                playlist_item: None,
                playlist_total: None,
                playlist_started_ms: 0,
                hwnd: 0,
                runner: None,
            });
        }
        ensure_common_controls();
        let template = load_dialog_template(IDD_PROGRESS);
        if template.is_null() {
            message_box("Could not load the progress dialog.", true);
            return 1;
        }
        let result = unsafe {
            DialogBoxIndirectParamW(
                module_handle(),
                template.cast(),
                ptr::null_mut(),
                Some(progress_proc),
                0,
            )
        };
        let (cancelled, code) = with(|s| (s.cancelled, s.exit_code)).unwrap_or((true, 1));
        if cancelled || result == -1 { 1 } else { code }
    }

    fn apply_line(line: &str) {
        if let Some(parsed) = DownloadResult::parse_emit_line(line) {
            with(|s| s.result = Some(parsed));
            return;
        }
        let now = unsafe { GetTickCount64() };
        with(|s| {
            s.last_output_ms = now;
            if let Some(rest) = line.strip_prefix("==> URL ") {
                let mut parts = rest.split_whitespace();
                if let (Some(cur), Some("of"), Some(total)) =
                    (parts.next(), parts.next(), parts.next())
                {
                    s.url_text = format!("URL {cur} of {}", total.trim_end_matches(':'));
                    s.item_text.clear();
                    s.download_started = false;
                    s.percent = 0.0;
                    s.eta = None;
                    s.playlist_item = None;
                    s.playlist_total = None;
                    s.playlist_started_ms = 0;
                }
            }
            if let Some(idx) = line.find("[download] Downloading item ") {
                let rest = &line[idx + "[download] Downloading item ".len()..];
                let mut parts = rest.split_whitespace();
                if let (Some(item), Some("of"), Some(total)) =
                    (parts.next(), parts.next(), parts.next())
                    && let (Ok(item), Ok(total)) = (item.parse::<i32>(), total.parse::<i32>())
                {
                    s.item_text = format!("item {item} of {total}");
                    s.download_started = false;
                    s.percent = 0.0;
                    s.eta = None;
                    s.playlist_item = Some(item);
                    s.playlist_total = Some(total);
                    if s.playlist_started_ms == 0 {
                        s.playlist_started_ms = now;
                    }
                }
            }
            if let Some(eta) = progress::parse_eta(line) {
                s.eta = Some(eta);
            }
            if let Some(percent) = progress::parse_progress_percent(line) {
                s.download_started = true;
                s.percent = progress::display_download_percent(percent);
            } else if let Some(status) = progress::parse_status_line(line) {
                s.status = status;
                if !s.download_started
                    && let Some(setup) = progress::next_setup_progress(s.percent, line)
                {
                    s.percent = setup;
                }
            }
            if s.hwnd != 0 {
                unsafe {
                    PostMessageW(s.hwnd as Handle, WM_APP_PROGRESS, 0, 0);
                }
            }
        });
    }

    fn update_controls(dlg: Handle, force_eta: bool) {
        let now = unsafe { GetTickCount64() };
        with(|s| {
            let elapsed = now.saturating_sub(s.last_output_ms);
            let status = progress::inactivity_status(elapsed).unwrap_or_else(|| s.status.clone());
            set_dlg_text(dlg, IDC_PROGRESS_STATUS, &status);
            let queue = [s.url_text.as_str(), s.item_text.as_str()]
                .into_iter()
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join(" - ");
            set_dlg_text(dlg, IDC_PROGRESS_QUEUE, &queue);
            if let Some(total) = s.playlist_total {
                let pelapsed = if s.playlist_started_ms == 0 {
                    0
                } else {
                    now.saturating_sub(s.playlist_started_ms)
                };
                let text = progress::playlist_eta_text(s.playlist_item, Some(total), pelapsed)
                    .unwrap_or_else(|| "Playlist: estimating...".into());
                set_dlg_text(dlg, IDC_PROGRESS_PLAYLIST_ETA, &text);
            } else {
                set_dlg_text(dlg, IDC_PROGRESS_PLAYLIST_ETA, "");
            }
            let refresh = force_eta
                || s.last_eta_ms == 0
                || now.saturating_sub(s.last_eta_ms) >= ETA_UPDATE_MS;
            let changed = s.display_eta != s.eta || s.display_started != s.download_started;
            if refresh && (changed || force_eta || s.last_eta_ms == 0) {
                s.display_eta = s.eta.clone();
                s.display_started = s.download_started;
                s.last_eta_ms = now;
                set_dlg_text(
                    dlg,
                    IDC_PROGRESS_ETA,
                    &progress::eta_status_text(s.display_eta.as_deref()),
                );
                let title = if let Some(eta) = &s.display_eta {
                    format!("{} - {eta} left", version::window_title())
                } else if s.display_started {
                    format!("{} - estimating...", version::window_title())
                } else {
                    version::window_title()
                };
                let tw = wide(&title);
                unsafe {
                    SetWindowTextW(dlg, tw.as_ptr());
                }
            }
            unsafe {
                let bar = GetDlgItem(dlg, IDC_PROGRESS_BAR);
                SendMessageW(bar, PBM_SETPOS, s.percent as usize, 0);
            }
        });
    }

    fn start_download(dlg: Handle) {
        let (command, args) = match with(|s| (s.command.clone(), s.args.clone())) {
            Some(v) => v,
            None => return,
        };
        unsafe {
            std::env::set_var("QUARK_GUI", "1");
        }
        let runner = match quark_core::process::HiddenProcess::spawn(&command, &args) {
            Ok(r) => r,
            Err(e) => {
                message_box(&format!("Could not start download:\n{e}"), true);
                unsafe {
                    PostMessageW(dlg, WM_APP_DONE, 1, 0);
                }
                return;
            }
        };
        let stdout = runner.stdout_handle() as usize;
        let stderr = runner.stderr_handle() as usize;
        with(|s| {
            s.hwnd = dlg as usize;
            s.runner = Some(runner);
        });
        thread::spawn(move || {
            quark_core::process::read_handle_lines(stdout as Handle, apply_line);
        });
        thread::spawn(move || {
            quark_core::process::read_handle_lines(stderr as Handle, apply_line);
        });
        unsafe {
            let bar = GetDlgItem(dlg, IDC_PROGRESS_BAR);
            SendMessageW(bar, PBM_SETRANGE, 0, 100isize << 16);
            SetTimer(dlg, TIMER_ID, TIMER_MS, None);
        }
    }

    fn finish_download(dlg: Handle, exit_code: i32) {
        unsafe {
            KillTimer(dlg, TIMER_ID);
        }
        let mut result = with(|s| s.result.clone())
            .flatten()
            .unwrap_or(DownloadResult {
                exit_code,
                ..DownloadResult::default()
            });
        if with(|s| s.result.is_none()).unwrap_or(true) {
            result.exit_code = exit_code;
        }
        if result.success() {
            with(|s| {
                s.percent = 100.0;
                s.status = "Done.".into();
                s.eta = None;
            });
            update_controls(dlg, true);
            let title = wide(&format!("{} - Done", version::window_title()));
            unsafe {
                SetWindowTextW(dlg, title.as_ptr());
            }
            message_box(
                &format!("Download complete!\n\n{}", result.dialog_body()),
                false,
            );
            if !result.output_dir.is_empty() {
                open_url(&result.output_dir);
            }
        } else {
            let mut message = result.dialog_body();
            if message.trim().is_empty() {
                message = "Download failed.".into();
            }
            message_box(&message, true);
        }
        unsafe {
            EndDialog(dlg, exit_code as isize);
        }
    }

    fn cancel_download(dlg: Handle) {
        with(|s| s.cancelled = true);
        unsafe {
            KillTimer(dlg, TIMER_ID);
            EndDialog(dlg, 0);
        }
    }

    unsafe extern "system" fn progress_proc(
        dlg: Handle,
        msg: u32,
        wparam: usize,
        _lparam: isize,
    ) -> isize {
        match msg {
            WM_INITDIALOG => {
                with(|s| s.hwnd = dlg as usize);
                unsafe {
                    let menu = GetSystemMenu(dlg, 0);
                    if !menu.is_null() {
                        EnableMenuItem(menu, SC_CLOSE, MF_BYCOMMAND | MF_DISABLED | MF_GRAYED);
                        DrawMenuBar(dlg);
                    }
                }
                let title = wide(&version::window_title());
                unsafe {
                    SetWindowTextW(dlg, title.as_ptr());
                }
                start_download(dlg);
                1
            }
            WM_CLOSE | WM_SYSCOMMAND if (wparam & 0xFFF0) == SC_CLOSE as usize => 1,
            WM_TIMER => {
                update_controls(dlg, false);
                if let Some(code) = with(|s| s.runner.as_ref().and_then(|r| r.try_wait())).flatten()
                {
                    with(|s| {
                        s.exit_code = code as i32;
                        s.finished = true;
                    });
                    unsafe {
                        PostMessageW(dlg, WM_APP_DONE, code as usize, 0);
                    }
                }
                1
            }
            WM_APP_PROGRESS => {
                update_controls(dlg, false);
                1
            }
            WM_APP_DONE => {
                finish_download(dlg, wparam as i32);
                1
            }
            WM_COMMAND => {
                let id = (wparam & 0xFFFF) as i32;
                let notify = ((wparam >> 16) & 0xFFFF) as u32;
                if id == 2 && notify == BN_CLICKED {
                    cancel_download(dlg);
                    return 1;
                }
                0
            }
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                cancel_download(dlg);
                1
            }
            _ => 0,
        }
    }
}

pub fn run_progress(command: &str, cmd_args: &[String]) -> i32 {
    run_progress_dialog(command, cmd_args)
}
