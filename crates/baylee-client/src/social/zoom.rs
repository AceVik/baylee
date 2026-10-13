//! The lobby's chat surfaces grow with the shell past 1920 × 1080 (the
//! shell's `screen_scale` and text step, `docs/client.md` §"Lobby and
//! rooms"); at the table Bevy's `UiScale` grows the HUD and the chat with
//! it, so there the zoom is 1.
//!
//! Every size the chat's builders write goes through [`zpx`] and every font
//! size through [`z`], which read the zoom the builder set for itself
//! ([`Scope`]): one number, set once per build, instead of a scale threaded
//! through every helper.

use std::cell::Cell;

use bevy::prelude::*;

thread_local! {
    static ZOOM: Cell<f32> = const { Cell::new(1.0) };
}

/// How much larger the lobby's chat draws on this window.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Zoom(pub f32);

impl Default for Zoom {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The zoom the builder on this thread draws at, until it is dropped.
pub(crate) struct Scope(f32);

impl Scope {
    /// Draws at `zoom` until the scope ends.
    pub(crate) fn new(zoom: f32) -> Self {
        Self(ZOOM.with(|z| z.replace(zoom)))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        ZOOM.with(|z| z.set(self.0));
    }
}

/// A size at the current zoom.
pub(crate) fn z(v: f32) -> f32 {
    v * ZOOM.with(Cell::get)
}

/// A length in pixels at the current zoom.
pub(crate) fn zpx(v: f32) -> Val {
    Val::Px(z(v))
}
