//! Native macOS (Cocoa/AppKit) backend: native alerts, form, and progress
//! windows built with objc2. No Slint delegation.

mod action;
mod form;
mod progress;

use std::sync::Once;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSControl,
};
use objc2_foundation::{MainThreadMarker, NSString};

use crate::backend::Renderer;
use crate::event::ProgressChannel;
use crate::model::{FormOutcome, FormSpec, MessageKind, ProgressSpec};

use action::ActionTarget;

pub struct CocoaRenderer;

impl CocoaRenderer {
    pub fn new() -> Self {
        CocoaRenderer
    }
}

impl Default for CocoaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CocoaRenderer {
    fn run_form(&self, spec: FormSpec) -> FormOutcome {
        match MainThreadMarker::new() {
            Some(mtm) => form::run_form(mtm, spec),
            None => FormOutcome::Cancel,
        }
    }

    fn run_progress(&self, spec: ProgressSpec, channel: ProgressChannel) -> i32 {
        match MainThreadMarker::new() {
            Some(mtm) => progress::run_progress(mtm, spec, channel),
            None => drain(&channel),
        }
    }

    fn message(&self, kind: MessageKind, title: &str, body: &str) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("{title}: {body}");
            return;
        };
        let _app = init_app(mtm);
        let alert: Retained<NSAlert> = unsafe { NSAlert::new(mtm) };
        let style = match kind {
            MessageKind::Error => NSAlertStyle::Critical,
            MessageKind::Info => NSAlertStyle::Informational,
        };
        unsafe {
            alert.setAlertStyle(style);
            alert.setMessageText(&NSString::from_str(title));
            alert.setInformativeText(&NSString::from_str(body));
            alert.runModal();
        }
    }

    fn name(&self) -> &'static str {
        "cocoa"
    }
}

/// Return the shared application, configuring it once so modal sessions run.
pub(crate) fn init_app(mtm: MainThreadMarker) -> Retained<NSApplication> {
    static START: Once = Once::new();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    START.call_once(|| unsafe {
        app.finishLaunching();
    });
    app
}

/// End the current modal loop (called from a button/timer action).
pub(crate) fn stop_modal(mtm: MainThreadMarker) {
    unsafe { NSApplication::sharedApplication(mtm).stopModal() };
}

/// Wire an `NSControl`'s target/action to fire `target`'s closure.
pub(crate) fn wire(control: &NSControl, target: &Retained<ActionTarget>) {
    let any: &AnyObject = target;
    unsafe {
        control.setTarget(Some(any));
        control.setAction(Some(ActionTarget::selector()));
    }
}

/// Fallback used when no GUI can be shown: drain the channel for the exit code.
fn drain(channel: &ProgressChannel) -> i32 {
    let mut code = 0;
    while let Ok(update) = channel.updates.recv() {
        if let crate::event::ProgressUpdate::Done(c) = update {
            code = c;
            break;
        }
    }
    code
}
