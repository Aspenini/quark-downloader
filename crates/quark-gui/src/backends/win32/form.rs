//! Native Win32 form window: labels, edits, radio groups, combos, checkboxes,
//! a multi-line list edit, and file/folder pickers, laid out in fixed rows.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::Once;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::HFONT;
use windows_sys::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::BST_CHECKED;
use windows_sys::Win32::UI::Shell::{
    SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
    IsDialogMessageW, PostQuitMessage, SendMessageW, SetWindowLongPtrW, SetWindowTextW,
    TranslateMessage, BM_GETCHECK, BM_SETCHECK, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL,
    CB_RESETCONTENT, CB_SETCURSEL, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, ES_WANTRETURN,
    GWLP_USERDATA, LBS_NOTIFY, LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL,
    LB_GETTEXT, MSG, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WS_BORDER, WS_CAPTION, WS_CHILD, WS_GROUP,
    WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON, BS_GROUPBOX, BS_PUSHBUTTON,
};

use super::util::{self, ThemePaint};
use crate::model::{Field, FieldValue, FormOutcome, FormSpec, FormValues};

const MARGIN: i32 = 16;
const CLIENT_W: i32 = 560;
const LABEL_W: i32 = 150;
const ROW_H: i32 = 26;
const LABEL_H: i32 = 18;
const LIST_H: i32 = 110;
const RADIO_H: i32 = 22;
const SPACING: i32 = 8;
const BUTTON_W: i32 = 90;
const BUTTON_H: i32 = 28;
const BROWSE_W: i32 = 84;

const ID_SUBMIT: usize = 1;
const ID_CANCEL: usize = 2;
const ID_EXTRA_BASE: usize = 10;
const ID_BROWSE_BASE: usize = 200;
const ID_BROWSE_LIMIT: usize = 300;
const ID_LIST_INPUT_BASE: usize = 300;
const ID_LIST_ADD_BASE: usize = 400;
const ID_LIST_REMOVE_BASE: usize = 500;
const ID_LIST_BOX_BASE: usize = 600;

const STYLE: WINDOW_STYLE = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
const ORIGINAL_STYLE: WINDOW_STYLE = WS_POPUP | WS_CAPTION | WS_SYSMENU;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Submit,
    Extra(usize),
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayoutKind {
    OriginalMain,
    OriginalSettings,
    Generic,
}

/// A control to read back after the loop ends.
enum Control {
    Text(HWND),
    ListEdit(HWND),
    ListBox(HWND),
    Combo(HWND),
    Check(HWND),
    Radio(Vec<HWND>),
}

struct Entry {
    id: String,
    control: Control,
}

struct BrowseTarget {
    edit: HWND,
    directory: bool,
}

struct ListTarget {
    input: HWND,
    listbox: HWND,
}

struct ComboDependency {
    radio_buttons: Vec<HWND>,
    combo: HWND,
    option_sets: Vec<Vec<String>>,
}

/// Shared with the wndproc through GWLP_USERDATA.
struct FormState {
    decision: Cell<Option<Decision>>,
    browse: Vec<BrowseTarget>,
    lists: Vec<ListTarget>,
    dependencies: Vec<ComboDependency>,
    paint: ThemePaint,
}

pub fn run_form(spec: FormSpec) -> FormOutcome {
    static CLASS: Once = Once::new();
    let class_name = util::register_class("QuarkGuiForm", Some(form_wndproc), &CLASS);
    let font = util::message_font();
    let units = util::DialogUnits::from_font(font);
    let layout = layout_kind(&spec);
    let client_w = if layout == LayoutKind::Generic {
        CLIENT_W
    } else {
        units.x(360)
    };
    let window_style = if layout == LayoutKind::Generic {
        STYLE
    } else {
        ORIGINAL_STYLE
    };

    // Created with a provisional height; resized after layout below.
    let hwnd = util::top_level(
        &class_name,
        &spec.window.title,
        window_style,
        client_w,
        units.y(314),
    );
    let paint = ThemePaint::new(spec.window.theme, hwnd);

    let mut entries: Vec<Entry> = Vec::new();
    let mut browse: Vec<BrowseTarget> = Vec::new();
    let mut lists: Vec<ListTarget> = Vec::new();
    let mut y = MARGIN;
    let control_x = MARGIN + LABEL_W + SPACING;
    let control_w = client_w - control_x - MARGIN;

    if layout == LayoutKind::OriginalMain {
        build_original_main(
            hwnd,
            font,
            units,
            &spec,
            &mut entries,
            &mut browse,
            &mut lists,
        );
        y = units.y(314);
    } else if layout == LayoutKind::OriginalSettings {
        build_original_settings(hwnd, font, units, &spec, &mut entries, &mut browse);
        y = units.y(314);
    } else {
        for field in &spec.fields {
            match field {
                Field::Section { label } => {
                    y += 4;
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y,
                        client_w - 2 * MARGIN,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    y += LABEL_H + SPACING;
                }
                Field::Text { id, label, value } => {
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y + 4,
                        LABEL_W,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    let edit = util::child(
                        "EDIT",
                        value,
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_TABSTOP
                            | WS_GROUP
                            | WS_BORDER
                            | ES_AUTOHSCROLL as u32,
                        control_x,
                        y,
                        control_w,
                        ROW_H - 2,
                        hwnd,
                        0,
                        font,
                    );
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Text(edit),
                    });
                    y += ROW_H + SPACING;
                }
                Field::Path {
                    id,
                    label,
                    value,
                    directory,
                } => {
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y + 4,
                        LABEL_W,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    let edit_w = control_w - BROWSE_W - SPACING;
                    let edit = util::child(
                        "EDIT",
                        value,
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_TABSTOP
                            | WS_GROUP
                            | WS_BORDER
                            | ES_AUTOHSCROLL as u32,
                        control_x,
                        y,
                        edit_w,
                        ROW_H - 2,
                        hwnd,
                        0,
                        font,
                    );
                    let browse_id = ID_BROWSE_BASE + browse.len();
                    util::child(
                        "BUTTON",
                        "Browse...",
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_PUSHBUTTON as u32,
                        control_x + edit_w + SPACING,
                        y - 1,
                        BROWSE_W,
                        ROW_H,
                        hwnd,
                        browse_id,
                        font,
                    );
                    browse.push(BrowseTarget {
                        edit,
                        directory: *directory,
                    });
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Text(edit),
                    });
                    y += ROW_H + SPACING;
                }
                Field::List {
                    id, label, items, ..
                } => {
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y,
                        client_w - 2 * MARGIN,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    y += LABEL_H + 2;
                    let edit = util::child(
                        "EDIT",
                        &items.join("\r\n"),
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_TABSTOP
                            | WS_GROUP
                            | WS_BORDER
                            | WS_VSCROLL
                            | (ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32,
                        MARGIN,
                        y,
                        client_w - 2 * MARGIN,
                        LIST_H,
                        hwnd,
                        0,
                        font,
                    );
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::ListEdit(edit),
                    });
                    y += LIST_H + SPACING;
                }
                Field::Combo {
                    id,
                    label,
                    options,
                    selected,
                } => {
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y + 4,
                        LABEL_W,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    let combo = util::child(
                        "COMBOBOX",
                        "",
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_TABSTOP
                            | WS_GROUP
                            | WS_VSCROLL
                            | CBS_DROPDOWNLIST as u32,
                        control_x,
                        y,
                        control_w,
                        200, // includes dropdown height
                        hwnd,
                        0,
                        font,
                    );
                    for opt in options {
                        let opt = util::wide(opt);
                        unsafe { SendMessageW(combo, CB_ADDSTRING, 0, opt.as_ptr() as LPARAM) };
                    }
                    unsafe { SendMessageW(combo, CB_SETCURSEL, *selected, 0) };
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Combo(combo),
                    });
                    y += ROW_H + SPACING;
                }
                Field::DependentCombo {
                    id,
                    label,
                    controller,
                    option_sets,
                    selected,
                } => {
                    util::child(
                        "STATIC",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_GROUP,
                        MARGIN,
                        y + 4,
                        LABEL_W,
                        LABEL_H,
                        hwnd,
                        0,
                        font,
                    );
                    let combo = util::child(
                        "COMBOBOX",
                        "",
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_TABSTOP
                            | WS_GROUP
                            | WS_VSCROLL
                            | CBS_DROPDOWNLIST as u32,
                        control_x,
                        y,
                        control_w,
                        200,
                        hwnd,
                        0,
                        font,
                    );
                    let options = option_sets
                        .get(spec.selected_index(controller))
                        .map(Vec::as_slice)
                        .unwrap_or_default();
                    set_combo_options(combo, options, *selected);
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Combo(combo),
                    });
                    y += ROW_H + SPACING;
                }
                Field::Check { id, label, value } => {
                    let check = util::child(
                        "BUTTON",
                        label,
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTOCHECKBOX as u32,
                        MARGIN,
                        y,
                        client_w - 2 * MARGIN,
                        RADIO_H,
                        hwnd,
                        0,
                        font,
                    );
                    if *value {
                        unsafe { SendMessageW(check, BM_SETCHECK, BST_CHECKED as usize, 0) };
                    }
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Check(check),
                    });
                    y += RADIO_H + SPACING;
                }
                Field::Radio {
                    id,
                    label,
                    options,
                    selected,
                } => {
                    let horizontal = label.is_empty();
                    if !horizontal {
                        util::child(
                            "STATIC",
                            label,
                            WS_CHILD | WS_VISIBLE | WS_GROUP,
                            MARGIN,
                            y,
                            client_w - 2 * MARGIN,
                            LABEL_H,
                            hwnd,
                            0,
                            font,
                        );
                        y += LABEL_H + 2;
                    }
                    let mut buttons = Vec::with_capacity(options.len());
                    for (i, opt) in options.iter().enumerate() {
                        // WS_GROUP on the first button starts the radio group; the
                        // next WS_GROUP control (any later field) ends it.
                        let group = if i == 0 { WS_GROUP | WS_TABSTOP } else { 0 };
                        let radio = util::child(
                            "BUTTON",
                            opt,
                            WS_CHILD | WS_VISIBLE | group | BS_AUTORADIOBUTTON as u32,
                            if horizontal {
                                MARGIN + i as i32 * 100
                            } else {
                                MARGIN + 8
                            },
                            y,
                            if horizontal {
                                92
                            } else {
                                client_w - 2 * MARGIN - 8
                            },
                            RADIO_H,
                            hwnd,
                            0,
                            font,
                        );
                        if i == *selected {
                            unsafe { SendMessageW(radio, BM_SETCHECK, BST_CHECKED as usize, 0) };
                        }
                        buttons.push(radio);
                        if !horizontal {
                            y += RADIO_H;
                        }
                    }
                    if horizontal {
                        y += RADIO_H;
                    }
                    entries.push(Entry {
                        id: id.clone(),
                        control: Control::Radio(buttons),
                    });
                    y += SPACING;
                }
            }
        }

        // Button row, right-aligned: [extras...] [submit] [cancel].
        y += 4;
        let total = (spec.extra_buttons.len() as i32 + 2) * (BUTTON_W + SPACING) - SPACING;
        let mut bx = client_w - MARGIN - total;
        for (i, btn) in spec.extra_buttons.iter().enumerate() {
            util::child(
                "BUTTON",
                &btn.label,
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_PUSHBUTTON as u32,
                bx,
                y,
                BUTTON_W,
                BUTTON_H,
                hwnd,
                ID_EXTRA_BASE + i,
                font,
            );
            bx += BUTTON_W + SPACING;
        }
        util::child(
            "BUTTON",
            &spec.submit_label,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_DEFPUSHBUTTON as u32,
            bx,
            y,
            BUTTON_W,
            BUTTON_H,
            hwnd,
            ID_SUBMIT,
            font,
        );
        util::child(
            "BUTTON",
            &spec.cancel_label,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_PUSHBUTTON as u32,
            bx + BUTTON_W + SPACING,
            y,
            BUTTON_W,
            BUTTON_H,
            hwnd,
            ID_CANCEL,
            font,
        );
        y += BUTTON_H + MARGIN;
    }

    util::resize_client(hwnd, window_style, client_w, y);

    let dependencies = collect_dependencies(&spec, &entries);
    let state = Box::new(FormState {
        decision: Cell::new(None),
        browse,
        lists,
        dependencies,
        paint,
    });
    let state_ptr = Box::into_raw(state);
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize) };

    util::show(hwnd);
    run_loop(hwnd);

    let values = read_values(&entries);
    let decision = unsafe { (*state_ptr).decision.get() }.unwrap_or(Decision::Cancel);
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        DestroyWindow(hwnd);
        drop(Box::from_raw(state_ptr));
    }

    match decision {
        Decision::Submit => FormOutcome::Submit(values),
        Decision::Extra(i) => match spec.extra_buttons.get(i) {
            Some(b) => FormOutcome::Button(b.id.clone(), values),
            None => FormOutcome::Cancel,
        },
        Decision::Cancel => FormOutcome::Cancel,
    }
}

fn layout_kind(spec: &FormSpec) -> LayoutKind {
    let has = |id| spec.fields.iter().any(|field| field.id() == Some(id));
    if has("urls") && has("media_type") && has("format") && has("output_dir") {
        LayoutKind::OriginalMain
    } else if has("download_dir")
        && has("strip_video_ids")
        && has("gui_download_mode")
        && has("yt_dlp")
    {
        LayoutKind::OriginalSettings
    } else {
        LayoutKind::Generic
    }
}

#[allow(clippy::too_many_arguments)]
fn child_dlu(
    class: &str,
    text: &str,
    style: WINDOW_STYLE,
    rect: [i32; 4],
    parent: HWND,
    id: usize,
    font: HFONT,
    units: util::DialogUnits,
) -> HWND {
    util::child(
        class,
        text,
        style,
        units.x(rect[0]),
        units.y(rect[1]),
        units.x(rect[2]),
        units.y(rect[3]),
        parent,
        id,
        font,
    )
}

fn field<'a>(spec: &'a FormSpec, id: &str) -> Option<&'a Field> {
    spec.fields
        .iter()
        .find(|candidate| candidate.id() == Some(id))
}

#[allow(clippy::too_many_arguments)]
fn build_original_main(
    hwnd: HWND,
    font: HFONT,
    units: util::DialogUnits,
    spec: &FormSpec,
    entries: &mut Vec<Entry>,
    browse: &mut Vec<BrowseTarget>,
    lists: &mut Vec<ListTarget>,
) {
    // Ported from the Crystal UI's `win32/gui.rc` at the pre-Rust main
    // branch. Keep these dialog-unit rectangles aligned with that resource.
    if let Some(Field::List { id, items, .. }) = field(spec, "urls") {
        child_dlu(
            "STATIC",
            "Video or playlist URL:",
            WS_CHILD | WS_VISIBLE | WS_GROUP,
            [10, 10, 100, 8],
            hwnd,
            0,
            font,
            units,
        );
        let input = child_dlu(
            "EDIT",
            "",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | WS_BORDER | ES_AUTOHSCROLL as u32,
            [10, 22, 296, 14],
            hwnd,
            ID_LIST_INPUT_BASE + lists.len(),
            font,
            units,
        );
        let list_index = lists.len();
        child_dlu(
            "BUTTON",
            "Add",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            [312, 21, 38, 14],
            hwnd,
            ID_LIST_ADD_BASE + list_index,
            font,
            units,
        );
        child_dlu(
            "STATIC",
            "Queue:",
            WS_CHILD | WS_VISIBLE | WS_GROUP,
            [10, 44, 50, 8],
            hwnd,
            0,
            font,
            units,
        );
        child_dlu(
            "BUTTON",
            "Remove",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            [296, 42, 54, 14],
            hwnd,
            ID_LIST_REMOVE_BASE + list_index,
            font,
            units,
        );
        let listbox = child_dlu(
            "LISTBOX",
            "",
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_GROUP
                | WS_BORDER
                | WS_VSCROLL
                | LBS_NOTIFY as u32,
            [10, 58, 340, 60],
            hwnd,
            ID_LIST_BOX_BASE + list_index,
            font,
            units,
        );
        for item in items {
            let item = util::wide(item);
            unsafe { SendMessageW(listbox, LB_ADDSTRING, 0, item.as_ptr() as LPARAM) };
        }
        lists.push(ListTarget { input, listbox });
        entries.push(Entry {
            id: id.clone(),
            control: Control::ListBox(listbox),
        });
    }

    if let Some(Field::Radio {
        id,
        options,
        selected,
        ..
    }) = field(spec, "media_type")
    {
        let mut visual_order: Vec<usize> = (0..options.len()).collect();
        visual_order.sort_by_key(
            |index| match options[*index].to_ascii_lowercase().as_str() {
                "audio" => 0,
                "video" => 1,
                _ => 2 + *index,
            },
        );
        let mut buttons = vec![std::ptr::null_mut(); options.len()];
        for (position, model_index) in visual_order.into_iter().enumerate() {
            let option = &options[model_index];
            let label = match option.to_ascii_lowercase().as_str() {
                "audio" => "Audio",
                "video" => "Video",
                _ => option,
            };
            let group = if position == 0 { WS_GROUP } else { 0 };
            let button = child_dlu(
                "BUTTON",
                label,
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | group | BS_AUTORADIOBUTTON as u32,
                [10 + position as i32 * 50, 126, 45, 10],
                hwnd,
                0,
                font,
                units,
            );
            if model_index == *selected {
                unsafe { SendMessageW(button, BM_SETCHECK, BST_CHECKED as usize, 0) };
            }
            buttons[model_index] = button;
        }
        entries.push(Entry {
            id: id.clone(),
            control: Control::Radio(buttons),
        });
    }

    child_dlu(
        "STATIC",
        "Format:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [10, 146, 35, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "format",
        [10, 158, 140, 120],
        entries,
    );

    if let Some(Field::Path {
        id,
        value,
        directory,
        ..
    }) = field(spec, "output_dir")
    {
        child_dlu(
            "STATIC",
            "Output folder:",
            WS_CHILD | WS_VISIBLE | WS_GROUP,
            [10, 186, 60, 8],
            hwnd,
            0,
            font,
            units,
        );
        let edit = child_dlu(
            "EDIT",
            value,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | WS_BORDER | ES_AUTOHSCROLL as u32,
            [10, 198, 275, 14],
            hwnd,
            0,
            font,
            units,
        );
        let browse_id = ID_BROWSE_BASE + browse.len();
        child_dlu(
            "BUTTON",
            "Browse...",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            [292, 196, 58, 14],
            hwnd,
            browse_id,
            font,
            units,
        );
        browse.push(BrowseTarget {
            edit,
            directory: *directory,
        });
        entries.push(Entry {
            id: id.clone(),
            control: Control::Text(edit),
        });
    }

    if let Some(button) = spec.extra_buttons.first() {
        child_dlu(
            "BUTTON",
            &button.label,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_PUSHBUTTON as u32,
            [10, 290, 20, 14],
            hwnd,
            ID_EXTRA_BASE,
            font,
            units,
        );
    }
    child_dlu(
        "BUTTON",
        &spec.submit_label,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_DEFPUSHBUTTON as u32,
        [248, 290, 55, 14],
        hwnd,
        ID_SUBMIT,
        font,
        units,
    );
    child_dlu(
        "BUTTON",
        &spec.cancel_label,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
        [304, 290, 50, 14],
        hwnd,
        ID_CANCEL,
        font,
        units,
    );
}

fn build_original_settings(
    hwnd: HWND,
    font: HFONT,
    units: util::DialogUnits,
    spec: &FormSpec,
    entries: &mut Vec<Entry>,
    browse: &mut Vec<BrowseTarget>,
) {
    // Retain the original four group boxes and footer. General is 20 DLU
    // taller, and the later groups shift by 20 DLU, to fit the Rust UI's
    // required theme/backend selectors without changing the window size.
    group_box(hwnd, font, units, "General", [7, 4, 346, 72]);
    if let Some(Field::Path {
        id,
        value,
        directory,
        ..
    }) = field(spec, "download_dir")
    {
        child_dlu(
            "STATIC",
            "Default download folder:",
            WS_CHILD | WS_VISIBLE | WS_GROUP,
            [14, 16, 120, 8],
            hwnd,
            0,
            font,
            units,
        );
        let edit = child_dlu(
            "EDIT",
            value,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | WS_BORDER | ES_AUTOHSCROLL as u32,
            [14, 28, 266, 14],
            hwnd,
            0,
            font,
            units,
        );
        let browse_id = ID_BROWSE_BASE + browse.len();
        child_dlu(
            "BUTTON",
            "Browse...",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            [288, 26, 58, 14],
            hwnd,
            browse_id,
            font,
            units,
        );
        browse.push(BrowseTarget {
            edit,
            directory: *directory,
        });
        entries.push(Entry {
            id: id.clone(),
            control: Control::Text(edit),
        });
    }
    child_dlu(
        "STATIC",
        "Theme:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [14, 50, 40, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "gui_theme",
        [55, 47, 80, 80],
        entries,
    );
    child_dlu(
        "STATIC",
        "GUI backend:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [150, 50, 58, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "gui_backend",
        [210, 47, 136, 80],
        entries,
    );

    group_box(hwnd, font, units, "Download naming", [7, 80, 346, 86]);
    create_check_dlu(
        hwnd,
        font,
        units,
        spec,
        "strip_video_ids",
        "Remove trailing video ID from filenames",
        [14, 92, 320, 10],
        entries,
    );
    create_check_dlu(
        hwnd,
        font,
        units,
        spec,
        "sanitize_filenames",
        "Sanitize filenames (ASCII-safe)",
        [14, 106, 320, 10],
        entries,
    );
    child_dlu(
        "STATIC",
        "Spaces in filenames:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [14, 122, 80, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "filename_spaces",
        [100, 120, 90, 80],
        entries,
    );
    create_check_dlu(
        hwnd,
        font,
        units,
        spec,
        "playlist_folders",
        "Put playlists in their own folder",
        [14, 140, 320, 10],
        entries,
    );

    group_box(hwnd, font, units, "Downloads", [7, 170, 346, 48]);
    child_dlu(
        "STATIC",
        "Download window:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [14, 182, 80, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "gui_download_mode",
        [14, 194, 120, 80],
        entries,
    );
    create_check_dlu(
        hwnd,
        font,
        units,
        spec,
        "download_logs",
        "Create download logs",
        [150, 196, 120, 10],
        entries,
    );

    group_box(hwnd, font, units, "Tools", [7, 222, 346, 46]);
    child_dlu(
        "STATIC",
        "yt-dlp:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [14, 234, 40, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "yt_dlp",
        [14, 246, 120, 80],
        entries,
    );
    child_dlu(
        "STATIC",
        "ffmpeg:",
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        [150, 234, 40, 8],
        hwnd,
        0,
        font,
        units,
    );
    create_combo_dlu(
        hwnd,
        font,
        units,
        spec,
        "ffmpeg",
        [150, 246, 120, 80],
        entries,
    );

    child_dlu(
        "BUTTON",
        &spec.submit_label,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_DEFPUSHBUTTON as u32,
        [248, 290, 50, 14],
        hwnd,
        ID_SUBMIT,
        font,
        units,
    );
    child_dlu(
        "BUTTON",
        &spec.cancel_label,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
        [304, 290, 50, 14],
        hwnd,
        ID_CANCEL,
        font,
        units,
    );
}

fn group_box(hwnd: HWND, font: HFONT, units: util::DialogUnits, label: &str, rect: [i32; 4]) {
    child_dlu(
        "BUTTON",
        label,
        WS_CHILD | WS_VISIBLE | WS_GROUP | BS_GROUPBOX as u32,
        rect,
        hwnd,
        0,
        font,
        units,
    );
}

#[allow(clippy::too_many_arguments)]
fn create_combo_dlu(
    hwnd: HWND,
    font: HFONT,
    units: util::DialogUnits,
    spec: &FormSpec,
    id: &str,
    rect: [i32; 4],
    entries: &mut Vec<Entry>,
) {
    let Some(field) = field(spec, id) else {
        return;
    };
    let (options, selected) = match field {
        Field::Combo {
            options, selected, ..
        } => (options.as_slice(), *selected),
        Field::DependentCombo {
            controller,
            option_sets,
            selected,
            ..
        } => (
            option_sets
                .get(spec.selected_index(controller))
                .map(Vec::as_slice)
                .unwrap_or_default(),
            *selected,
        ),
        _ => return,
    };
    let combo = child_dlu(
        "COMBOBOX",
        "",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
        rect,
        hwnd,
        0,
        font,
        units,
    );
    set_combo_options(combo, options, selected);
    entries.push(Entry {
        id: id.to_owned(),
        control: Control::Combo(combo),
    });
}

#[allow(clippy::too_many_arguments)]
fn create_check_dlu(
    hwnd: HWND,
    font: HFONT,
    units: util::DialogUnits,
    spec: &FormSpec,
    id: &str,
    label: &str,
    rect: [i32; 4],
    entries: &mut Vec<Entry>,
) {
    let Some(Field::Check { value, .. }) = field(spec, id) else {
        return;
    };
    let check = child_dlu(
        "BUTTON",
        label,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTOCHECKBOX as u32,
        rect,
        hwnd,
        0,
        font,
        units,
    );
    if *value {
        unsafe { SendMessageW(check, BM_SETCHECK, BST_CHECKED as usize, 0) };
    }
    entries.push(Entry {
        id: id.to_owned(),
        control: Control::Check(check),
    });
}

/// Standard message loop with dialog navigation (Tab, Enter, Esc).
fn run_loop(hwnd: HWND) {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn read_values(entries: &[Entry]) -> FormValues {
    let mut values = FormValues::default();
    for entry in entries {
        let value = match &entry.control {
            Control::Text(edit) => FieldValue::Text(util::window_text(*edit)),
            Control::ListEdit(edit) => FieldValue::List(
                util::window_text(*edit)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect(),
            ),
            Control::ListBox(listbox) => FieldValue::List(listbox_items(*listbox)),
            Control::Combo(combo) => {
                let sel = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) };
                FieldValue::Index(sel.max(0) as usize)
            }
            Control::Check(check) => FieldValue::Bool(
                unsafe { SendMessageW(*check, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize,
            ),
            Control::Radio(buttons) => {
                let sel = buttons
                    .iter()
                    .position(|b| {
                        (unsafe { SendMessageW(*b, BM_GETCHECK, 0, 0) }) == BST_CHECKED as isize
                    })
                    .unwrap_or(0);
                FieldValue::Index(sel)
            }
        };
        values.0.insert(entry.id.clone(), value);
    }
    values
}

unsafe extern "system" fn form_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FormState;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &*state_ptr;

    if let Some(result) = state.paint.handle(hwnd, msg, wparam, lparam) {
        return result;
    }

    match msg {
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            update_dependent_combo(state, lparam as HWND);
            match id {
                ID_SUBMIT => {
                    for list in &state.lists {
                        add_list_item(list);
                    }
                    state.decision.set(Some(Decision::Submit));
                    PostQuitMessage(0);
                }
                ID_CANCEL => {
                    state.decision.set(Some(Decision::Cancel));
                    PostQuitMessage(0);
                }
                _ if (ID_EXTRA_BASE..ID_BROWSE_BASE).contains(&id) => {
                    state
                        .decision
                        .set(Some(Decision::Extra(id - ID_EXTRA_BASE)));
                    PostQuitMessage(0);
                }
                _ if (ID_BROWSE_BASE..ID_BROWSE_LIMIT).contains(&id) => {
                    if let Some(target) = state.browse.get(id - ID_BROWSE_BASE) {
                        if let Some(path) = browse(hwnd, target.directory) {
                            let path = util::wide(&path);
                            SetWindowTextW(target.edit, path.as_ptr());
                        }
                    }
                }
                _ if (ID_LIST_ADD_BASE..ID_LIST_REMOVE_BASE).contains(&id) => {
                    if let Some(list) = state.lists.get(id - ID_LIST_ADD_BASE) {
                        add_list_item(list);
                    }
                }
                _ if (ID_LIST_REMOVE_BASE..ID_LIST_BOX_BASE).contains(&id) => {
                    if let Some(list) = state.lists.get(id - ID_LIST_REMOVE_BASE) {
                        remove_list_item(list);
                    }
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            state.decision.set(Some(Decision::Cancel));
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn listbox_items(listbox: HWND) -> Vec<String> {
    let count = unsafe { SendMessageW(listbox, LB_GETCOUNT, 0, 0) };
    if count <= 0 {
        return Vec::new();
    }
    (0..count)
        .filter_map(|index| {
            let mut buffer = vec![0u16; 4096];
            let len = unsafe {
                SendMessageW(
                    listbox,
                    LB_GETTEXT,
                    index as usize,
                    buffer.as_mut_ptr() as LPARAM,
                )
            };
            (len >= 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]))
        })
        .collect()
}

fn add_list_item(list: &ListTarget) {
    let value = util::window_text(list.input).trim().to_owned();
    if value.is_empty() {
        return;
    }
    if !listbox_items(list.listbox)
        .iter()
        .any(|item| item == &value)
    {
        let value = util::wide(&value);
        unsafe { SendMessageW(list.listbox, LB_ADDSTRING, 0, value.as_ptr() as LPARAM) };
    }
    let empty = util::wide("");
    unsafe { SetWindowTextW(list.input, empty.as_ptr()) };
}

fn remove_list_item(list: &ListTarget) {
    let selected = unsafe { SendMessageW(list.listbox, LB_GETCURSEL, 0, 0) };
    if selected >= 0 {
        unsafe { SendMessageW(list.listbox, LB_DELETESTRING, selected as usize, 0) };
    }
}

fn collect_dependencies(spec: &FormSpec, entries: &[Entry]) -> Vec<ComboDependency> {
    spec.fields
        .iter()
        .filter_map(|field| {
            let Field::DependentCombo {
                id,
                controller,
                option_sets,
                ..
            } = field
            else {
                return None;
            };
            let combo =
                entries
                    .iter()
                    .find(|entry| entry.id == *id)
                    .and_then(|entry| match entry.control {
                        Control::Combo(combo) => Some(combo),
                        _ => None,
                    })?;
            let radio_buttons = entries
                .iter()
                .find(|entry| entry.id == *controller)
                .and_then(|entry| match &entry.control {
                    Control::Radio(buttons) => Some(buttons.clone()),
                    _ => None,
                })?;
            Some(ComboDependency {
                radio_buttons,
                combo,
                option_sets: option_sets.clone(),
            })
        })
        .collect()
}

fn update_dependent_combo(state: &FormState, clicked: HWND) {
    for dependency in &state.dependencies {
        let Some(selected) = dependency
            .radio_buttons
            .iter()
            .position(|button| *button == clicked)
        else {
            continue;
        };
        set_combo_options(
            dependency.combo,
            dependency
                .option_sets
                .get(selected)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            0,
        );
    }
}

fn set_combo_options(combo: HWND, options: &[String], selected: usize) {
    unsafe {
        SendMessageW(combo, CB_RESETCONTENT, 0, 0);
        for option in options {
            let option = util::wide(option);
            SendMessageW(combo, CB_ADDSTRING, 0, option.as_ptr() as LPARAM);
        }
        SendMessageW(combo, CB_SETCURSEL, selected, 0);
    }
}

fn browse(owner: HWND, directory: bool) -> Option<String> {
    if directory {
        browse_folder(owner)
    } else {
        browse_file(owner)
    }
}

fn browse_folder(owner: HWND) -> Option<String> {
    unsafe {
        // Required for BIF_NEWDIALOGSTYLE; safe to call repeatedly.
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut display = [0u16; 260];
        let bi = BROWSEINFOW {
            hwndOwner: owner,
            pidlRoot: std::ptr::null_mut(),
            pszDisplayName: display.as_mut_ptr(),
            lpszTitle: std::ptr::null(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            lpfn: None,
            lParam: 0,
            iImage: 0,
        };
        let pidl = SHBrowseForFolderW(&bi);
        if pidl.is_null() {
            return None;
        }
        let mut path = [0u16; 260];
        let ok = SHGetPathFromIDListW(pidl, path.as_mut_ptr());
        CoTaskMemFree(pidl as *const c_void);
        if ok == 0 {
            return None;
        }
        let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
        Some(String::from_utf16_lossy(&path[..len]))
    }
}

fn browse_file(owner: HWND) -> Option<String> {
    unsafe {
        let mut file = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner;
        ofn.lpstrFile = file.as_mut_ptr();
        ofn.nMaxFile = file.len() as u32;
        ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
        if GetOpenFileNameW(&mut ofn) == 0 {
            return None;
        }
        let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
        Some(String::from_utf16_lossy(&file[..len]))
    }
}
