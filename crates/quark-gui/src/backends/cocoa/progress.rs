//! Native AppKit progress window. Driven by an `NSTimer` scheduled in the
//! modal run-loop mode, so `runModalForWindow` can block while the timer polls
//! the update channel.

use std::cell::Cell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSLayoutAttribute, NSModalPanelRunLoopMode, NSProgressIndicator,
    NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSRunLoop, NSSize, NSString, NSTimer};

use super::action::ActionTarget;
use super::{init_app, stop_modal};
use crate::event::{ProgressChannel, ProgressUpdate};
use crate::model::ProgressSpec;

pub fn run_progress(mtm: MainThreadMarker, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
    let app = init_app(mtm);

    let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(460.0, 190.0));
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Miniaturizable;
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

    let stack = vstack(mtm, 12.0);
    let queue_label = label(mtm, "");
    let bar = progress_bar(mtm);
    let status_label = label(mtm, &spec.initial_status);
    let eta_label = label(mtm, "");

    let cancelled = Rc::new(Cell::new(false));
    let cancel_target = {
        let cancelled = cancelled.clone();
        ActionTarget::new(
            mtm,
            Box::new(move || {
                cancelled.set(true);
                stop_modal(mtm);
            }),
        )
    };
    let cancel_button = push_button(mtm, "Cancel", &cancel_target);

    unsafe {
        stack.addArrangedSubview(&queue_label);
        stack.addArrangedSubview(&bar);
        stack.addArrangedSubview(&status_label);
        stack.addArrangedSubview(&eta_label);
        stack.addArrangedSubview(&cancel_button);
    }

    let content_view: &NSView = &stack;
    window.setContentView(Some(content_view));
    window.center();

    // A timer scheduled in the modal run-loop mode drains updates and drives
    // the UI while runModalForWindow blocks.
    let exit_code = Rc::new(Cell::new(None::<i32>));
    let tick = {
        let bar = bar.clone();
        let status_label = status_label.clone();
        let eta_label = eta_label.clone();
        let queue_label = queue_label.clone();
        let channel = channel.clone();
        let exit_code = exit_code.clone();
        ActionTarget::new(
            mtm,
            Box::new(move || {
                while let Ok(update) = channel.updates.try_recv() {
                    unsafe {
                        match update {
                            ProgressUpdate::Percent(p) => bar.setDoubleValue(p.clamp(0.0, 100.0)),
                            ProgressUpdate::Status(s) => {
                                status_label.setStringValue(&NSString::from_str(&s))
                            }
                            ProgressUpdate::Eta(e) => {
                                let text = e.map(|x| format!("Time left: {x}")).unwrap_or_default();
                                eta_label.setStringValue(&NSString::from_str(&text));
                            }
                            ProgressUpdate::Queue(q) => {
                                queue_label.setStringValue(&NSString::from_str(&q))
                            }
                            ProgressUpdate::Log(_) => {}
                            ProgressUpdate::Done(c) => {
                                exit_code.set(Some(c));
                                stop_modal(mtm);
                            }
                        }
                    }
                }
            }),
        )
    };

    let timer: Retained<NSTimer> = {
        let any: &AnyObject = &tick;
        unsafe {
            NSTimer::timerWithTimeInterval_target_selector_userInfo_repeats(
                0.05,
                any,
                ActionTarget::selector(),
                None,
                true,
            )
        }
    };
    unsafe {
        NSRunLoop::currentRunLoop().addTimer_forMode(&timer, NSModalPanelRunLoopMode);
    }

    window.makeKeyAndOrderFront(None);
    unsafe { app.runModalForWindow(&window) };
    unsafe { timer.invalidate() };
    window.close();

    let code = exit_code.get();
    if code.is_none() {
        // Cancelled before completion.
        channel.request_cancel();
    }
    code.unwrap_or(1)
}

fn vstack(mtm: MainThreadMarker, spacing: f64) -> Retained<NSStackView> {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
    let s = unsafe { NSStackView::initWithFrame(mtm.alloc::<NSStackView>(), rect) };
    unsafe {
        s.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        s.setAlignment(NSLayoutAttribute::Leading);
        s.setSpacing(spacing);
    }
    s
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    unsafe { NSTextField::labelWithString(&NSString::from_str(text), mtm) }
}

fn progress_bar(mtm: MainThreadMarker) -> Retained<NSProgressIndicator> {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 16.0));
    let bar =
        unsafe { NSProgressIndicator::initWithFrame(mtm.alloc::<NSProgressIndicator>(), rect) };
    unsafe {
        bar.setIndeterminate(false);
        bar.setMinValue(0.0);
        bar.setMaxValue(100.0);
        bar.setDoubleValue(0.0);
    }
    bar
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
