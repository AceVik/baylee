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
//! - `gallery` (dev-control builds only): every component in every state,
//!   the screen WP0b-1 is accepted on.

pub mod controls;
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub mod gallery;
pub mod lint;
pub mod metrics;
pub mod role;
pub mod size;
pub mod states;
pub mod surfaces;
pub mod tokens;

pub use metrics::{ShellMetrics, px_fixed};
pub use role::Role;
pub use size::{Frame, InputClass, Platform, TextSize, Viewport};

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

/// The kit's systems: the input class, and the text-size chords.
pub struct ShellKitPlugin;

impl Plugin for ShellKitPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<bevy::input::touch::TouchInput>()
            .add_message::<bevy::input::mouse::MouseMotion>()
            .add_message::<KeyboardInput>()
            .init_resource::<InputClass>()
            .add_systems(PreUpdate, size::follow_the_input)
            .add_systems(
                Update,
                (step_the_text_size, face_follows_the_text_size).chain(),
            );
        controls::install(app);
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

/// What a text-size chord asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeChord {
    /// `Ctrl/Cmd + =` (also `+`): one step larger.
    Larger,
    /// `Ctrl/Cmd + −`: one step smaller.
    Smaller,
    /// `Ctrl/Cmd + 0`: back to the default step.
    Reset,
}

impl SizeChord {
    /// The chord a key press is, with the platform's command modifier held.
    ///
    /// Read on the **logical** key first, so the German layout's `=` (which
    /// is Shift+0) and `+` both step up; the physical keys are the fallback
    /// for a key event that carries no character (§2.6, `KEYBOARD.md`).
    #[must_use]
    pub fn of(logical: &Key, code: KeyCode) -> Option<Self> {
        if let Key::Character(text) = logical {
            match text.as_str() {
                "=" | "+" => return Some(Self::Larger),
                "-" | "\u{2212}" => return Some(Self::Smaller),
                "0" => return Some(Self::Reset),
                _ => {}
            }
        }
        match code {
            KeyCode::Equal | KeyCode::NumpadAdd => Some(Self::Larger),
            KeyCode::Minus | KeyCode::NumpadSubtract => Some(Self::Smaller),
            KeyCode::Digit0 | KeyCode::Numpad0 => Some(Self::Reset),
            _ => None,
        }
    }

    /// The step this chord leads to from `now`.
    #[must_use]
    pub fn apply(self, now: TextSize) -> TextSize {
        match self {
            Self::Larger => now.larger(),
            Self::Smaller => now.smaller(),
            Self::Reset => TextSize::default(),
        }
    }
}

/// `Ctrl/Cmd + = − 0` step the shell's text size up, down and back (§8),
/// on every platform. The setting is this device's (`ClientSettings`).
///
/// Only outside a game: on the table `⌘⇧↑↓` sizes the preview, and the
/// table's faces do not follow the shell's step.
fn step_the_text_size(
    mut keys: MessageReader<KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let (Some(codes), Some(mut settings)) = (codes, settings) else {
        keys.clear();
        return;
    };
    if phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed) {
        keys.clear();
        return;
    }
    let command = codes.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    for key in keys.read() {
        if !command || !key.state.is_pressed() {
            continue;
        }
        let Some(chord) = SizeChord::of(&key.logical_key, key.key_code) else {
            continue;
        };
        let next = chord.apply(settings.text_size);
        if next != settings.text_size {
            settings.text_size = next;
            settings.save();
        }
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

    /// The German layout's `=` is Shift+0 and arrives as the character `=`
    /// on the `Digit0` key: it steps up, it does not reset (§2.6, S4-8).
    #[test]
    fn the_logical_character_wins_over_the_physical_key() {
        let equal_on_zero = SizeChord::of(&Key::Character("=".into()), KeyCode::Digit0);
        assert_eq!(equal_on_zero, Some(SizeChord::Larger));
        let plus = SizeChord::of(&Key::Character("+".into()), KeyCode::BracketRight);
        assert_eq!(plus, Some(SizeChord::Larger));
        assert_eq!(
            SizeChord::of(&Key::Character("0".into()), KeyCode::Digit0),
            Some(SizeChord::Reset)
        );
        assert_eq!(
            SizeChord::of(
                &Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                KeyCode::Minus
            ),
            Some(SizeChord::Smaller)
        );
        assert_eq!(
            SizeChord::of(&Key::Character("a".into()), KeyCode::KeyA),
            None
        );
    }

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
    fn the_steps_stop_at_both_ends_and_reset_to_the_default() {
        assert_eq!(SizeChord::Larger.apply(TextSize::Xl), TextSize::Xl);
        assert_eq!(SizeChord::Smaller.apply(TextSize::Xs), TextSize::Xs);
        assert_eq!(SizeChord::Larger.apply(TextSize::M), TextSize::L);
        assert_eq!(SizeChord::Reset.apply(TextSize::Xs), TextSize::L);
    }
}
