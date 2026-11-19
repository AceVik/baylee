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

/// The shell kit's sizes for the chat's surfaces, so that a head, a close
/// button or an action built by `shellkit` stands in the drawer as it does
/// in a sheet. Its factor is [`Zoom`]: in the lobby the window's own
/// metrics, at a table the default step's on a 1920 × 1080 window, since
/// there Bevy's `UiScale` already grows the chat.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Shell(pub crate::shellkit::ShellMetrics);

impl Shell {
    /// The sizes at a table, and before the first window is measured.
    #[must_use]
    pub fn at_table() -> Self {
        Self(crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(
                crate::shellkit::metrics::SCALE_FROM.0,
                crate::shellkit::metrics::SCALE_FROM.1,
            ),
            crate::shellkit::TextSize::default(),
        ))
    }

    /// Builds with these sizes, in `lang`, until the scope ends: what
    /// [`kit`] hands a builder's helpers, as [`Scope`] does the zoom.
    pub(crate) fn scope(self, lang: baylee_client_core::i18n::Lang) -> KitScope {
        let german = lang == baylee_client_core::i18n::Lang::De;
        KitScope(SHELL.with(|s| s.replace(Some((self.0, german)))))
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::at_table()
    }
}

thread_local! {
    static SHELL: Cell<Option<(crate::shellkit::ShellMetrics, bool)>> = const { Cell::new(None) };
}

/// The kit sizes the builder on this thread draws at, until it is dropped.
pub(crate) struct KitScope(Option<(crate::shellkit::ShellMetrics, bool)>);

impl Drop for KitScope {
    fn drop(&mut self) {
        SHELL.with(|s| s.set(self.0));
    }
}

/// The shell kit for the surface being built: [`Shell`]'s sizes where the
/// builder set them ([`Shell::scope`]), a table's otherwise.
pub(crate) fn kit(fonts: &crate::hud::UiFonts) -> crate::shellkit::controls::Kit<'_> {
    let (m, german) = SHELL
        .with(Cell::get)
        .unwrap_or_else(|| (Shell::at_table().0, false));
    crate::shellkit::controls::Kit { fonts, m, german }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellkit::metrics::screen_scale;
    use crate::shellkit::{ShellMetrics, TextSize, Viewport};

    /// A kit control in the drawer and the row beside it grow together: the
    /// kit's factor is the zoom the chat's own sizes are multiplied by, on
    /// every window and at every text step, and 1 at a table.
    #[test]
    fn the_kits_factor_is_the_chats_zoom() {
        assert!((Shell::at_table().0.factor - 1.0).abs() < f32::EPSILON);
        for (w, h) in [
            (960.0, 700.0),
            (1280.0, 800.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
        ] {
            for step in [
                TextSize::Xs,
                TextSize::S,
                TextSize::M,
                TextSize::L,
                TextSize::Xl,
            ] {
                let view = Viewport::desktop(w, h);
                let zoom = screen_scale(view, step) * step.factor();
                let kit = ShellMetrics::of(view, step).factor;
                assert!((kit - zoom).abs() < 1e-6, "{w} × {h} at {step:?}");
            }
        }
    }

    /// A helper outside a builder's scope draws at a table's sizes, and a
    /// scope ends where it began.
    #[test]
    fn a_kit_scope_ends_where_it_began() {
        let fonts = crate::hud::UiFonts::default();
        let table = Shell::at_table().0;
        assert_eq!(kit(&fonts).m, table);
        let big = Shell(ShellMetrics::of(
            Viewport::desktop(3840.0, 2160.0),
            TextSize::M,
        ));
        {
            let _scope = big.scope(baylee_client_core::i18n::Lang::De);
            assert_eq!(kit(&fonts).m, big.0);
            assert!(kit(&fonts).german);
        }
        assert_eq!(kit(&fonts).m, table);
        assert!(!kit(&fonts).german);
    }
}
