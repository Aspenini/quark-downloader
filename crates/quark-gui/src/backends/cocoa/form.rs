//! Native AppKit form window.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSButton, NSColor, NSControlStateValueOff,
    NSControlStateValueOn, NSFont, NSLayoutAttribute, NSModalResponseOK, NSOpenPanel,
    NSPopUpButton, NSScrollView, NSStackView, NSStackViewGravity, NSTextField, NSTextView,
    NSTitlePosition, NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask,
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
    Radio {
        selected: Rc<Cell<usize>>,
        dependents: Rc<RefCell<Vec<ComboDependency>>>,
    },
}

struct ComboDependency {
    popup: Retained<NSPopUpButton>,
    option_sets: Vec<Vec<String>>,
}

struct Entry {
    id: String,
    control: Control,
}

pub fn run_form(mtm: MainThreadMarker, spec: FormSpec) -> FormOutcome {
    let app = init_app(mtm);
    let grouped = spec
        .fields
        .iter()
        .any(|field| matches!(field, Field::Section { .. }));

    // Fit queue-style forms to their content instead of leaving a large empty
    // lower half.
    let window_height = if spec
        .fields
        .iter()
        .any(|field| matches!(field, Field::List { .. }))
    {
        380.0
    } else if grouped {
        spec.window.height as f64
    } else {
        (spec.window.height as f64).min(550.0)
    };
    let content = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(spec.window.width as f64, window_height),
    );
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

    let margin = 14.0;
    let content_width = spec.window.width as f64 - margin * 2.0;
    let content_height = window_height - margin * 2.0;
    let outer = vstack(mtm, 10.0);
    unsafe {
        outer.setFrame(NSRect::new(
            NSPoint::new(margin, margin),
            NSSize::new(content_width, content_height),
        ));
    }
    let fields_stack = vstack(mtm, 8.0);
    set_width(&fields_stack, content_width);

    let mut entries: Vec<Entry> = Vec::new();
    let mut targets: Vec<Retained<ActionTarget>> = Vec::new();

    if grouped {
        let mut index = 0;
        while index < spec.fields.len() {
            match &spec.fields[index] {
                Field::Section { label } => {
                    let start = index + 1;
                    let end = spec.fields[start..]
                        .iter()
                        .position(|field| matches!(field, Field::Section { .. }))
                        .map_or(spec.fields.len(), |offset| start + offset);
                    let group = build_group(
                        mtm,
                        label,
                        &spec.fields[start..end],
                        content_width,
                        &mut entries,
                        &mut targets,
                    );
                    unsafe { fields_stack.addArrangedSubview(&group) };
                    index = end;
                }
                field => {
                    let row = build_field(mtm, field, content_width, &mut entries, &mut targets);
                    unsafe { fields_stack.addArrangedSubview(&row) };
                    index += 1;
                }
            }
        }
    } else {
        for field in &spec.fields {
            let row = build_field(mtm, field, content_width, &mut entries, &mut targets);
            unsafe { fields_stack.addArrangedSubview(&row) };
        }
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
        unsafe { buttons.addView_inGravity(&b, NSStackViewGravity::Leading) };
    }
    {
        let target = decide(decision.clone(), Decision::Submit);
        let b = push_button(mtm, &spec.submit_label, &target);
        targets.push(target);
        unsafe { buttons.addView_inGravity(&b, NSStackViewGravity::Trailing) };
    }
    {
        let target = decide(decision.clone(), Decision::Cancel);
        let b = push_button(mtm, &spec.cancel_label, &target);
        targets.push(target);
        unsafe { buttons.addView_inGravity(&b, NSStackViewGravity::Trailing) };
    }
    set_width(&buttons, content_width);
    unsafe { outer.addArrangedSubview(&buttons) };

    let content_view = unsafe { NSView::initWithFrame(mtm.alloc::<NSView>(), content) };
    unsafe { content_view.addSubview(&outer) };
    window.setContentView(Some(&content_view));
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

/// Build a compact, titled settings card containing one logical section.
fn build_group(
    mtm: MainThreadMarker,
    title: &str,
    fields: &[Field],
    content_width: f64,
    entries: &mut Vec<Entry>,
    targets: &mut Vec<Retained<ActionTarget>>,
) -> Retained<NSBox> {
    let inset = 12.0;
    let spacing = 8.0;
    let inner_width = content_width - inset * 2.0;
    let stack = vstack(mtm, spacing);
    set_width(&stack, inner_width);

    for field in fields {
        let row = build_field(mtm, field, inner_width, entries, targets);
        unsafe { stack.addArrangedSubview(&row) };
    }

    let rows_height: f64 = fields.iter().map(field_height).sum();
    let gaps = fields.len().saturating_sub(1) as f64 * spacing;
    let height = rows_height + gaps + 30.0;
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(content_width, height));
    let group = unsafe { NSBox::initWithFrame(mtm.alloc::<NSBox>(), rect) };
    unsafe {
        group.setBoxType(NSBoxType::NSBoxCustom);
        group.setTitle(&NSString::from_str(title));
        group.setTitlePosition(NSTitlePosition::NSAtTop);
        group.setTitleFont(&NSFont::boldSystemFontOfSize(13.0));
        group.setContentViewMargins(NSSize::new(inset, 9.0));
        group.setCornerRadius(8.0);
        group.setBorderWidth(1.0);
        group.setBorderColor(&NSColor::separatorColor());
        group.setFillColor(&NSColor::controlBackgroundColor());
        group.setContentView(Some(&stack));
    }
    set_width(&group, content_width);
    set_height(&group, height);
    group
}

fn field_height(field: &Field) -> f64 {
    match field {
        Field::Text { .. } | Field::Path { .. } => 50.0,
        Field::List { .. } => 174.0,
        Field::Radio { .. } | Field::Check { .. } => 22.0,
        Field::Combo { .. } | Field::DependentCombo { .. } => 26.0,
        Field::Section { .. } => 0.0,
    }
}

/// Build one field as a stack view row, recording its control for readback.
fn build_field(
    mtm: MainThreadMarker,
    field: &Field,
    content_width: f64,
    entries: &mut Vec<Entry>,
    targets: &mut Vec<Retained<ActionTarget>>,
) -> Retained<NSStackView> {
    match field {
        Field::Section { label } => {
            let row = vstack(mtm, 2.0);
            let heading = label_view(mtm, label);
            let font = unsafe { NSFont::boldSystemFontOfSize(13.0) };
            unsafe {
                heading.setFont(Some(&font));
                row.addArrangedSubview(&heading);
            }
            row
        }
        Field::Text { id, label, value } => {
            let row = vstack(mtm, 4.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let tf = text_field(mtm, value, content_width);
            unsafe { row.addArrangedSubview(&tf) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::Text(tf),
            });
            row
        }
        Field::Path {
            id, label, value, ..
        } => {
            let row = vstack(mtm, 4.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, label)) };
            let controls = hstack(mtm, 8.0);
            let tf = text_field(mtm, value, content_width - 96.0);
            unsafe { controls.addArrangedSubview(&tf) };
            let Field::Path { directory, .. } = field else {
                unreachable!()
            };
            let tf_cb = tf.clone();
            let dir = *directory;
            let target = ActionTarget::new(mtm, Box::new(move || browse(mtm, &tf_cb, dir)));
            let b = push_button(mtm, "Browse…", &target);
            targets.push(target);
            unsafe { controls.addArrangedSubview(&b) };
            unsafe { row.addArrangedSubview(&controls) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::Text(tf),
            });
            row
        }
        Field::List {
            id,
            label,
            items,
            placeholder,
        } => {
            let row = vstack(mtm, 5.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, &format!("{label}:"))) };
            let entry_row = hstack(mtm, 8.0);
            let input = text_field(mtm, "", content_width - 66.0);
            unsafe { input.setPlaceholderString(Some(&NSString::from_str(placeholder))) };
            unsafe { entry_row.addArrangedSubview(&input) };
            let (scroll, text_view) = text_area(mtm, &items.join("\n"), content_width);
            let input_cb = input.clone();
            let text_cb = text_view.clone();
            let add_target = ActionTarget::new(
                mtm,
                Box::new(move || {
                    let value = unsafe { input_cb.stringValue() }.to_string();
                    if value.trim().is_empty() {
                        return;
                    }
                    let current = unsafe { text_cb.string() }.to_string();
                    let updated = if current.trim().is_empty() {
                        value
                    } else {
                        format!("{current}\n{value}")
                    };
                    unsafe {
                        text_cb.setString(&NSString::from_str(&updated));
                        input_cb.setStringValue(&NSString::from_str(""));
                    }
                }),
            );
            let add = push_button(mtm, "Add", &add_target);
            targets.push(add_target);
            unsafe { entry_row.addArrangedSubview(&add) };
            unsafe { row.addArrangedSubview(&entry_row) };

            let queue_header = hstack(mtm, 8.0);
            let queue_label = label_view(mtm, "Queue:");
            unsafe { queue_header.addView_inGravity(&queue_label, NSStackViewGravity::Leading) };
            let text_cb = text_view.clone();
            let remove_target = ActionTarget::new(
                mtm,
                Box::new(move || {
                    let current = unsafe { text_cb.string() }.to_string();
                    let mut lines: Vec<&str> = current.lines().collect();
                    lines.pop();
                    unsafe { text_cb.setString(&NSString::from_str(&lines.join("\n"))) };
                }),
            );
            let remove = push_button(mtm, "Remove", &remove_target);
            targets.push(remove_target);
            unsafe { queue_header.addView_inGravity(&remove, NSStackViewGravity::Trailing) };
            set_width(&queue_header, content_width);
            unsafe { row.addArrangedSubview(&queue_header) };
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
            unsafe { row.addArrangedSubview(&label_view(mtm, &format!("{label}:"))) };
            let pop = pop_up(mtm, options, *selected);
            unsafe { row.addArrangedSubview(&pop) };
            entries.push(Entry {
                id: id.clone(),
                control: Control::Combo(pop),
            });
            row
        }
        Field::DependentCombo {
            id,
            label,
            controller,
            option_sets,
            selected,
        } => {
            let row = hstack(mtm, 8.0);
            unsafe { row.addArrangedSubview(&label_view(mtm, &format!("{label}:"))) };
            let controller_selected = entries
                .iter()
                .find(|entry| entry.id == *controller)
                .and_then(|entry| match &entry.control {
                    Control::Radio { selected, .. } => Some(selected.get()),
                    _ => None,
                })
                .unwrap_or(0);
            let options = option_sets
                .get(controller_selected)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let pop = pop_up(mtm, options, *selected);
            unsafe { row.addArrangedSubview(&pop) };
            if let Some(Control::Radio { dependents, .. }) = entries
                .iter()
                .find(|entry| entry.id == *controller)
                .map(|entry| &entry.control)
            {
                dependents.borrow_mut().push(ComboDependency {
                    popup: pop.clone(),
                    option_sets: option_sets.clone(),
                });
            }
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
            let row = hstack(mtm, 10.0);
            if !label.is_empty() {
                unsafe { row.addArrangedSubview(&label_view(mtm, &format!("{label}:"))) };
            }
            let state = Rc::new(Cell::new(*selected));
            let dependents = Rc::new(RefCell::new(Vec::<ComboDependency>::new()));
            let buttons: Rc<Vec<Retained<NSButton>>> =
                Rc::new(options.iter().map(|o| radio_button(mtm, o)).collect());
            for (i, b) in buttons.iter().enumerate() {
                unsafe { b.setState(bool_state(i == *selected)) };
                let state_cb = state.clone();
                let buttons_cb = buttons.clone();
                let dependents_cb = dependents.clone();
                let target = ActionTarget::new(
                    mtm,
                    Box::new(move || {
                        state_cb.set(i);
                        for (j, other) in buttons_cb.iter().enumerate() {
                            unsafe { other.setState(bool_state(j == i)) };
                        }
                        for dependency in dependents_cb.borrow().iter() {
                            reset_pop_up(
                                &dependency.popup,
                                dependency
                                    .option_sets
                                    .get(i)
                                    .map(Vec::as_slice)
                                    .unwrap_or_default(),
                            );
                        }
                    }),
                );
                wire(b, &target);
                targets.push(target);
                unsafe { row.addArrangedSubview(b) };
            }
            entries.push(Entry {
                id: id.clone(),
                control: Control::Radio {
                    selected: state,
                    dependents,
                },
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
            Control::Radio { selected, .. } => FieldValue::Index(selected.get()),
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

fn text_field(mtm: MainThreadMarker, value: &str, width: f64) -> Retained<NSTextField> {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 24.0));
    let tf = unsafe { NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), rect) };
    unsafe { tf.setStringValue(&NSString::from_str(value)) };
    set_width(&tf, width);
    tf
}

fn text_area(
    mtm: MainThreadMarker,
    value: &str,
    width: f64,
) -> (Retained<NSScrollView>, Retained<NSTextView>) {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 96.0));
    let tv = unsafe { NSTextView::initWithFrame(mtm.alloc::<NSTextView>(), rect) };
    unsafe { tv.setString(&NSString::from_str(value)) };
    let scroll = unsafe { NSScrollView::initWithFrame(mtm.alloc::<NSScrollView>(), rect) };
    unsafe {
        scroll.setHasVerticalScroller(true);
        let doc: &NSView = &tv;
        scroll.setDocumentView(Some(doc));
    }
    set_width(&scroll, width);
    set_height(&scroll, 96.0);
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

fn reset_pop_up(pop: &NSPopUpButton, options: &[String]) {
    unsafe {
        pop.removeAllItems();
        for option in options {
            pop.addItemWithTitle(&NSString::from_str(option));
        }
        pop.selectItemAtIndex(0);
    }
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
