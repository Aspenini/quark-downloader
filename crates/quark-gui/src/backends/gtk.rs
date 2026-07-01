//! Native gtk backend (scaffold).
//!
//! Enabled with the `native-gtk` (or platform) feature. It will implement
//! [`crate::backend::Renderer`] using its toolkit, mapping the same QuarkGUI
//! widget model the Slint backend uses. Until then, backend selection falls
//! back to Slint, so this module is intentionally empty.
