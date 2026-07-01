//! Native AppKit form window.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSControlStateValueOff, NSControlStateValueOn, NSLayoutAttribute,
    NSModalResponseOK, NSOpenPanel, NSPopUpButton, NSScrollView, NSStackView, NSTextField,
    NSTextView, NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use super::action::ActionTarget;
use super::{init_app, stop_modal, wire};
use crate::model::{Field, FieldValue, FormOutcome, FormSpec, FormValues};

#[derive(Clone, Copy)]
enum Decision {
    Submit,
    Extra(usize),
    Cancel,
}

/// A control we need to read back after the form closes.
enum Control {
    Text(Retained<NSTextField>),
    List(Retained<NSTextView>),
    Combo(Retained<NSPopUpButton>),
    Check(Retained<NSButton>),
    Radio(Rc<Cell<usize>>),
}

struct Entry {
    id: String,
    control: Control,
}

pub fn run_form(mtm: MainThreadMarker, spec: FormSpec) -> FormOutcome {
    let app = init_app(mtm);

    let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(560.0, 560.0));
    // No Closable: the window is dismissed via its buttons, so we don't need a
    // window-close delegate to end the modal loop.
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Miniaturizable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            content,
            style,
            NSBackingStoreType(2),
            false,
        )
    };
    window.setTitle(&NSString::from_str(&spec.window.title));

    let outer = vstack(mtm, 12.0);
    unsafe {
        outer.setEdgeInsets(objc2_foundation::NSEdgeInsets {
            top: 16.0,
            left: 16.0,
            bottom: 16.0,
            right: 16.0,
        })
    };
    let fields_stack = vstack(mtm, 8.0);

    let mut entries: Vec<Entry> = Vec::new();
    let mut targets: Vec<Retained<ActionTarget>> = Vec::new();

    for field in &spec.fields {
        let row = build_field(mtm, field, &mut entries, &mut targets);
        unsafe { fields_stack.addArrangedSubview(&row) };
    }
    unsafe { outer.addArrangedSubview(&fields_stack) };

    // Button row.
    let decision: Rc<RefCell<Option<Decision>>> = Rc::new(RefCell::new(None));
    let buttons = hstack(mtm, 8.0);

    let decide = |d: Rc<RefCell<Option<Decision>>>, choice: Decision| {
        ActionTarget::new(
            mtm,
            Box::new(move || {
                *d.borrow_mut() = Some(choice);
                stop_modal(mtm);
            }),
        )
    };

    for (i, btn) in spec.extra_buttons.iter().enumerate() {
        let target = decide(decision.clone(), Decision::Extra(i));
        let b = push_button(mtm, &btn.label, &target);
        targets.push(target);
        unsafe { buttons.addArrangedSubview(&b) };
    }
    {
        let target = decide(decision.clone(), Decision::Cancel);
        let b = push_button(mtm, &spec.cancel_label, &target);
        targets.push(target);
        unsafe { buttons.addArrangedSubview(&b) };
    }
    {
        let target = decide(decision.clone(), Decision::Submit);
        let b = push_button(mtm, &spec.submit_label, &target);
        targets.push(target);
        unsafe { buttons.addArrangedSubview(&b) };
    }
    unsafe { outer.addArrangedSubview(&buttons) };

    let content_view: &NSView = &outer;
    window.setContentView(Some(content_view));
    window.center();
    window.makeKeyAndOrderFront(None);

    // Blocking modal loop; a button's action calls stopModal to end it.
    unsafe { app.runModalForWindow(&window) };

    let values = read_values(&entries);
    window.close();

    let decided = decision.borrow().unwrap_or(Decision::Cancel);
    match decided {
        Decision::Submit => FormOutcome::Submit(values),
        Decision::Extra(i) => match spec.extra_buttons.get(i) {
            Some(b) => FormOutcome::Button(b.id.clone(), values),
            None => FormOutcome::Cancel,
        },
        Decision::Cancel => FormOutcome::Cancel,
    }
}

/// Build one field as a stack view row, recording its control for readback.
fn build_field(
    mtm: MainThreadMarker,
    field: &Field,
    entries: &mut Vec<Entry>,
    targets: &mut Vec<Retained<ActionTarget>>,
) -> Retained<NSStackView> {
    match field {
        Field::Section { label } => {
            let row = vstack(mtm, 2.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            row
        }
        Field::Text { id, label, value }
        | Field::Path {
            id, label, value, ..
        } => {
            let row = hstack(mtm, 8.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let tf = text_field(mtm, value);
            unsafe { row.addArrangedSubview(&tf) };
            if let Field::Path { directory, .. } = field {
                let tf_cb = tf.clone();
                let dir = *directory;
                let target = ActionTarget::new(mtm, Box::new(move || browse(mtm, &tf_cb, dir)));
                let b = push_button(mtm, "Browse...", &target);
                targets.push(target);
                unsafe { row.addArrangedSubview(&b) };
            }
            entries.push(Entry {
                id: id.clone(),
                control: Control::Text(tf),
            });
            row
        }
        Field::List {
            id, label, items, ..
        } => {
            let row = vstack(mtm, 4.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let (scroll, text_view) = text_area(mtm, &items.join("\n"));
            unsafe { row.addArrangedSubview(&scroll) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::List(text_view),
            });
            row
        }
        Field::Combo {
            id,
            label,
            options,
            selected,
        } => {
            let row = hstack(mtm, 8.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let pop = pop_up(mtm, options, *selected);
            unsafe { row.addArrangedSubview(&pop) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::Combo(pop),
            });
            row
        }
        Field::Check { id, label, value } => {
            let row = hstack(mtm, 8.0);
            let b = checkbox(mtm, label, *value);
            unsafe { row.addArrangedSubview(&b) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::Check(b),
            });
            row
        }
        Field::Radio {
            id,
            label,
            options,
            selected,
        } => {
            let row = vstack(mtm, 2.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let state = Rc::new(Cell::new(*selected));
            let buttons: Rc<Vec<Retained<NSButton>>> =
                Rc::new(options.iter().map(|o| radio_button(mtm, o)).collect());
            for (i, b) in buttons.iter().enumerate() {
                unsafe { b.setState(bool_state(i == *selected)) };
                let state_cb = state.clone();
                let buttons_cb = buttons.clone();
                let target = ActionTarget::new(
                    mtm,
                    Box::new(move || {
                        state_cb.set(i);
                        for (j, other) in buttons_cb.iter().enumerate() {
                            unsafe { other.setState(bool_state(j == i)) };
                        }
                    }),
                );
                wire(b, &target);
                targets.push(target);
                unsafe { row.addArrangedSubview(b) };
            }
            entries.push(Entry {
                id: id.clone(),
                control: Control::Radio(state),
            });
            row
        }
    }
}

fn read_values(entries: &[Entry]) -> FormValues {
    let mut values = FormValues::default();
    for entry in entries {
        let value = match &entry.control {
            Control::Text(tf) => FieldValue::Text(unsafe { tf.stringValue() }.to_string()),
            Control::List(tv) => FieldValue::List(
                unsafe { tv.string() }
                    .to_string()
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect(),
            ),
            Control::Combo(pop) => {
                FieldValue::Index(unsafe { pop.indexOfSelectedItem() }.max(0) as usize)
            }
            Control::Check(b) => FieldValue::Bool(unsafe { b.state() } == NSControlStateValueOn),
            Control::Radio(state) => FieldValue::Index(state.get()),
        };
        values.0.insert(entry.id.clone(), value);
    }
    values
}

// ---- control builders ----------------------------------------------------

fn bool_state(on: bool) -> objc2_app_kit::NSControlStateValue {
    if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    }
}

fn zero_rect() -> NSRect {
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))
}

fn vstack(mtm: MainThreadMarker, spacing: f64) -> Retained<NSStackView> {
    let s = unsafe { NSStackView::initWithFrame(mtm.alloc::<NSStackView>(), zero_rect()) };
    unsafe {
        s.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        s.setAlignment(NSLayoutAttribute::Leading);
        s.setSpacing(spacing);
    }
    s
}

fn hstack(mtm: MainThreadMarker, spacing: f64) -> Retained<NSStackView> {
    let s = unsafe { NSStackView::initWithFrame(mtm.alloc::<NSStackView>(), zero_rect()) };
    unsafe {
        s.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        s.setSpacing(spacing);
    }
    s
}

fn label_view(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    unsafe { NSTextField::labelWithString(&NSString::from_str(text), mtm) }
}

fn text_field(mtm: MainThreadMarker, value: &str) -> Retained<NSTextField> {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(360.0, 24.0));
    let tf = unsafe { NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), rect) };
    unsafe { tf.setStringValue(&NSString::from_str(value)) };
    set_width(&tf, 360.0);
    tf
}

fn text_area(mtm: MainThreadMarker, value: &str) -> (Retained<NSScrollView>, Retained<NSTextView>) {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 110.0));
    let tv = unsafe { NSTextView::initWithFrame(mtm.alloc::<NSTextView>(), rect) };
    unsafe { tv.setString(&NSString::from_str(value)) };
    let scroll = unsafe { NSScrollView::initWithFrame(mtm.alloc::<NSScrollView>(), rect) };
    unsafe {
        scroll.setHasVerticalScroller(true);
        let doc: &NSView = &tv;
        scroll.setDocumentView(Some(doc));
    }
    set_width(&scroll, 500.0);
    set_height(&scroll, 110.0);
    (scroll, tv)
}

/// Pin a view's width so stack views don't collapse it to its intrinsic size.
fn set_width(view: &NSView, width: f64) {
    unsafe {
        view.widthAnchor()
            .constraintEqualToConstant(width)
            .setActive(true)
    };
}

fn set_height(view: &NSView, height: f64) {
    unsafe {
        view.heightAnchor()
            .constraintEqualToConstant(height)
            .setActive(true)
    };
}

fn pop_up(mtm: MainThreadMarker, options: &[String], selected: usize) -> Retained<NSPopUpButton> {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 26.0));
    let pop = unsafe {
        NSPopUpButton::initWithFrame_pullsDown(mtm.alloc::<NSPopUpButton>(), rect, false)
    };
    for opt in options {
        unsafe { pop.addItemWithTitle(&NSString::from_str(opt)) };
    }
    unsafe { pop.selectItemAtIndex(selected as isize) };
    pop
}

fn checkbox(mtm: MainThreadMarker, label: &str, value: bool) -> Retained<NSButton> {
    let b = unsafe {
        NSButton::checkboxWithTitle_target_action(&NSString::from_str(label), None, None, mtm)
    };
    unsafe { b.setState(bool_state(value)) };
    b
}

fn radio_button(mtm: MainThreadMarker, label: &str) -> Retained<NSButton> {
    unsafe {
        NSButton::radioButtonWithTitle_target_action(&NSString::from_str(label), None, None, mtm)
    }
}

fn push_button(
    mtm: MainThreadMarker,
    title: &str,
    target: &Retained<ActionTarget>,
) -> Retained<NSButton> {
    let any: &AnyObject = target;
    unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(any),
            Some(ActionTarget::selector()),
            mtm,
        )
    }
}

fn browse(mtm: MainThreadMarker, tf: &NSTextField, directory: bool) {
    let panel = unsafe { NSOpenPanel::openPanel(mtm) };
    unsafe {
        panel.setCanChooseFiles(!directory);
        panel.setCanChooseDirectories(directory);
    }
    if unsafe { panel.runModal() } == NSModalResponseOK {
        if let Some(url) = unsafe { panel.URL() } {
            if let Some(path) = unsafe { url.path() } {
                unsafe { tf.setStringValue(&path) };
            }
        }
    }
}
