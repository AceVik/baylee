//! The out-of-table shell's tokens, size classes and components.
//!
//! The shell design (`.claude/ux-b6/DESIGN-v5.md` §2) gives every screen
//! outside the table one header, one body, one token set and one component
//! set. This module is that set; the screens move onto it package by package
//! (WP1–WP5), each converting its own module.
//!
//! The design names it `shellui/`. That path was taken: `crate::shellui` is
//! the hover preview's protection shells (`shell_ui.wgsl`), so the shell's
//! kit lives here instead.
//!
//! - [`size`]: the five size classes (§2.7), the input class, the five text
//!   steps (§8).
//! - [`metrics`]: [`ShellMetrics`], every size a shell screen takes, scaled
//!   by the text step with floors; `m.px(n)` and [`px_fixed`].
//! - [`tokens`]: colours, spacing, radii and the `GlobalZIndex` bands (§2.2).
//! - [`role`]: what a node *is*, for the dev-control dump and the checks
//!   that read it (overflow, label budgets, contrast, the 44-px hit area).
//! - [`controls`], [`surfaces`], [`states`]: the components of §2.4.
//! - [`focus`]: the `TabOrder` tables, the Tab walker, the focus ring and
//!   the kit's own field editor (`KEYBOARD.md` §1).
//! - [`keys`]: the shell's second keymap wired to the keyboard — the
//!   resolver over the context stack, the text-size chords (§2, §7.1).
//! - [`overlay`]: the `?` overlay (§4.1).
//! - `gallery` (dev-control builds only): every component in every state,
//!   the screen WP0b-1 is accepted on.

pub mod controls;
pub mod focus;
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
pub mod gallery;
#[cfg(test)]
mod keyboard_tests;
pub mod keys;
pub mod lint;
pub mod metrics;
pub mod overlay;
pub mod role;
pub mod size;
pub mod states;
pub mod surfaces;
pub mod tokens;

pub use metrics::{ShellMetrics, px_fixed};
pub use role::Role;
pub use size::{Frame, InputClass, Platform, TextSize, Viewport};

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

/// Whether the kit holds the keyboard: the `?` overlay is up, the dev
/// gallery, or a kit field has focus. The lobby's own key handling stands
/// aside while it does, so a key typed into the overlay's search is not
/// also typed behind it; at a table, a sheet with a kit field (the table
/// design's amendment to KEYBOARD §6) is the case the table's keymap asks
/// this about, as it asks the report form's `holds_keyboard`. Like the
/// report form's, a key that closes the sheet is still the sheet's in the
/// frame it closed it: the field editor reads the frame's keys before the
/// sheet goes.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct KitHolds(pub bool);

/// The kit's systems: the input class, focus, the shell keymap, the `?`
/// overlay, and the face step the text size sets.
pub struct ShellKitPlugin;

impl Plugin for ShellKitPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<bevy::input::touch::TouchInput>()
            .add_message::<bevy::input::mouse::MouseMotion>()
            .add_message::<KeyboardInput>()
            .init_resource::<InputClass>()
            .init_resource::<KitHolds>()
            .add_systems(PreUpdate, size::follow_the_input)
            .add_systems(Update, (face_follows_the_text_size, hold_the_keyboard));
        controls::install(app);
        focus::install(app);
        keys::install(app);
        overlay::install(app);
        #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
        gallery::install(app);
    }
}

/// Adds [`ShellKitPlugin`] unless another plugin already did.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<ShellKitPlugin>() {
        app.add_plugins(ShellKitPlugin);
    }
}

/// Keeps [`KitHolds`] true while the overlay or the gallery is up.
fn hold_the_keyboard(
    overlay: Res<overlay::Overlay>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    fields: Query<(), With<focus::ShellField>>,
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))] gallery: Option<
        Res<gallery::Gallery>,
    >,
    mut holds: ResMut<KitHolds>,
) {
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    let gallery = gallery.is_some_and(|g| g.open);
    #[cfg(not(all(feature = "dev-control", not(target_arch = "wasm32"))))]
    let gallery = false;
    let typing = focus
        .as_deref()
        .and_then(bevy::input_focus::InputFocus::get)
        .is_some_and(|f| fields.contains(f));
    let now = overlay::holds(&overlay) || gallery || typing;
    if holds.0 != now {
        holds.0 = now;
    }
}

/// The interface's card faces take the shell's text step (WP6's
/// `FaceMode::step`): the setting is the one source, written through when it
/// changes and once at start.
///
/// A dev-control build launched with `BAYLEE_TEXT_STEP` keeps the step that
/// variable names, so WP6's per-step photographs stay what they say.
fn face_follows_the_text_size(
    settings: Option<Res<crate::settings::ClientSettings>>,
    mode: Option<ResMut<crate::face::FaceMode>>,
) {
    let (Some(settings), Some(mut mode)) = (settings, mode) else {
        return;
    };
    if !settings.is_changed() || pinned_by_the_environment() {
        return;
    }
    let step = face_step(settings.text_size);
    if mode.step != step {
        mode.step = step;
    }
}

/// The face's step for a shell step: the same five, numbered alike.
#[must_use]
pub fn face_step(size: TextSize) -> baylee_client_core::textface::Step {
    baylee_client_core::textface::Step::new(size.step())
}

#[cfg(feature = "dev-control")]
fn pinned_by_the_environment() -> bool {
    std::env::var_os("BAYLEE_TEXT_STEP").is_some()
}

#[cfg(not(feature = "dev-control"))]
const fn pinned_by_the_environment() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shell's five steps are the face's five, in order (WP6 reads
    /// `FaceMode::step`; the shell's setting writes it).
    #[test]
    fn the_shell_step_and_the_face_step_are_the_same_five() {
        for (i, size) in TextSize::ALL.into_iter().enumerate() {
            let face = face_step(size);
            assert_eq!(usize::from(face.number()), i + 1);
            assert!((face.factor() - size.factor()).abs() < 1e-6);
        }
        assert_eq!(
            face_step(TextSize::default()),
            baylee_client_core::textface::Step::DEFAULT
        );
    }

    /// Changing the setting moves the faces' step; nothing else does.
    #[test]
    fn the_setting_writes_the_face_step() {
        let mut app = App::new();
        app.insert_resource(crate::settings::ClientSettings::default())
            .insert_resource(crate::face::FaceMode::default())
            .add_systems(Update, face_follows_the_text_size);
        app.update();
        assert_eq!(
            app.world().resource::<crate::face::FaceMode>().step,
            baylee_client_core::textface::Step::DEFAULT
        );
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .text_size = TextSize::Xl;
        app.update();
        assert_eq!(
            app.world().resource::<crate::face::FaceMode>().step,
            baylee_client_core::textface::Step::XL
        );
    }

    #[test]
    fn the_steps_stop_at_both_ends() {
        assert_eq!(TextSize::Xl.larger(), TextSize::Xl);
        assert_eq!(TextSize::Xs.smaller(), TextSize::Xs);
        assert_eq!(TextSize::M.larger(), TextSize::L);
    }
}
