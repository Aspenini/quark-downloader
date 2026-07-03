//! A reusable Objective-C target object that forwards `action:` messages to a
//! boxed Rust closure. Lets AppKit buttons/controls invoke Rust callbacks.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, Sel};
use objc2::{declare_class, msg_send_id, mutability, sel, ClassType, DeclaredClass};
use objc2_foundation::MainThreadMarker;

pub struct ActionIvars {
    pub callback: Box<dyn Fn()>,
}

declare_class!(
    pub struct ActionTarget;

    unsafe impl ClassType for ActionTarget {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "QuarkGuiActionTarget";
    }

    impl DeclaredClass for ActionTarget {
        type Ivars = ActionIvars;
    }

    unsafe impl ActionTarget {
        #[method(fire:)]
        fn fire(&self, _sender: Option<&AnyObject>) {
            (self.ivars().callback)();
        }
    }
);

impl ActionTarget {
    pub fn new(mtm: MainThreadMarker, callback: Box<dyn Fn()>) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(ActionIvars { callback });
        unsafe { msg_send_id![super(this), init] }
    }

    /// The selector the target responds to (`fire:`).
    pub fn selector() -> Sel {
        sel!(fire:)
    }
}
