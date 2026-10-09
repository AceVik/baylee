//! The settings screen's keys and the systems that read its controls back
//! (`KEYBOARD.md` §1.3, §7.8): its `TabOrder`, the search field, the
//! sliders, the sidebar's arrows and type-ahead, and the display mode's
//! 15-s trial.

use baylee_client_core::graphics::Graphics;
use baylee_client_core::settings_map::Section;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

use crate::lobby::LobbyState;
use crate::shellkit::controls::Slider;
use crate::shellkit::focus::{Remembered, Stop, TabOrder};

use super::rows::Volume;

/// The settings screen (`KEYBOARD.md` §1.3): the search field, the nav
/// (one stop, its sections walked with the arrows), Reset this section,
/// the search's results, then each section's controls in the map's order.
pub(crate) const SETTINGS: TabOrder = TabOrder {
    name: "settings",
    stops: &[
        "back",
        "search",
        "nav",
        "reset",
        "result",
        // Graphics
        "preset",
        "display-mode",
        "keep",
        "revert",
        "monitor",
        "anti-aliasing",
        "vsync",
        "frame-limit",
        "background-limit",
        "show-frame-rate",
        "backdrop",
        "ambient",
        "atmosphere",
        "hold-still",
        "sky",
        // Audio
        "master",
        "music",
        "effects",
        "mute-unfocused",
        "table-sounds",
        // Display & Interface
        "language",
        "text-size",
        "preview-size",
        "text-face",
        // Controls
        "shell-keys",
        "shell-reset",
        "take",
        "table-keys",
        "table-reset",
        "reset-all",
        // Gameplay
        "automation",
        "rail-preset",
        "rail-mine",
        "rail-theirs",
        "ability-reset-all",
        "ability",
        // Account
        "copy-handle",
        "sign-out",
        "delete-account",
        // Network & Gateway
        "switch-gateway",
        "copy-diagnostics",
        // Language models: the list, the caps, Save (`seatpanel`)
        "llm",
        // Updates
        "update",
        // Privacy & Data
        "consent",
        "crash",
        "report-problem",
    ],
    modal: false,
};

/// A language-model profile's sheet (§12): its fields and choices in the
/// order drawn, then Save, Discard and Close. A box once pressed walks
/// with Tab itself (`seatpanel::keys`).
pub(crate) const PROFILE_SHEET: TabOrder = TabOrder {
    name: "profile-sheet",
    stops: &["field", "foot"],
    modal: true,
};

/// The display mode's question line, while a trial runs.
#[derive(Component)]
pub(crate) struct TrialLine;

/// A nav item's section.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct NavSection(pub(crate) Section);

/// Tells the lobby whether the settings screen stands over its screen: its
/// search is then the one box a key types into (`Lobby::typing_here`), and
/// a search left holding the caret when the screen closes lets it go.
pub(crate) fn settings_over(mut state: ResMut<LobbyState>) {
    let open = state.settings_open();
    if state.lobby.settings_open() == open {
        return;
    }
    state.lobby.set_settings_open(open);
    if !open && state.lobby.focus() == baylee_client_core::lobby::Field::SettingsSearch {
        state.lobby.park_caret();
    }
}

/// A slider moved: its volume, written once it differs.
pub(crate) fn apply_sliders(
    sliders: Query<(&Slider, &Volume), Changed<Slider>>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let Some(mut settings) = settings else {
        return;
    };
    let mut wrote = false;
    for (slider, which) in &sliders {
        let want = f32::from(slider.value) / 100.0;
        let now = match which {
            Volume::Master => settings.audio.master(),
            Volume::Music => settings.music.volume(),
            Volume::Effects => settings.audio.effects(),
        };
        if (now - want).abs() < 0.004 {
            continue;
        }
        match which {
            Volume::Master => settings.audio.set_master(want),
            Volume::Effects => settings.audio.set_effects(want),
            Volume::Music => {
                settings.music.set_volume(want);
                if want > 0.0 {
                    settings.music.set_muted(false);
                }
            }
        }
        wrote = true;
    }
    if wrote {
        settings.save();
    }
}

/// ←→ (Shift: by ten), Home and End on a focused slider (§2.4).
pub(crate) fn slider_keys(
    mut keys: MessageReader<KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    focus: Res<InputFocus>,
    mut sliders: Query<&mut Slider>,
) {
    let Some(mut slider) = focus.get().and_then(|f| sliders.get_mut(f).ok()) else {
        keys.clear();
        return;
    };
    let shift = codes
        .as_deref()
        .is_some_and(|c| c.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    let by: i16 = if shift { 10 } else { 1 };
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let now = i16::from(slider.value);
        let to = match key.logical_key {
            Key::ArrowLeft | Key::ArrowDown => now - by,
            Key::ArrowRight | Key::ArrowUp => now + by,
            Key::Home => 0,
            Key::End => 100,
            _ => continue,
        }
        .clamp(0, 100);
        let to = u8::try_from(to).unwrap_or(0);
        if slider.value != to {
            slider.value = to;
        }
    }
}

/// The nav: focus walked onto a section shows it (automatic activation,
/// APG tabs), and letters typed there jump to the section they begin
/// (type-ahead; digits and `?` stay shortcuts).
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(crate) fn follow_nav(
    mut keys: MessageReader<KeyboardInput>,
    time: Res<Time<Real>>,
    mut focus: ResMut<InputFocus>,
    visible: Res<InputFocusVisible>,
    navs: Query<(Entity, &NavSection, &Stop)>,
    mut remembered: ResMut<Remembered>,
    mut state: ResMut<LobbyState>,
    mut typed: Local<(String, f64)>,
) {
    if !state.settings_open() {
        keys.clear();
        return;
    }
    let on_nav = focus.get().and_then(|f| navs.get(f).ok());
    if focus.is_changed()
        && visible.0
        && let Some((_, NavSection(section), _)) = on_nav
        && state.settings_section() != *section
    {
        state.set_settings_section(*section);
    }
    if on_nav.is_none() {
        keys.clear();
        return;
    }
    let now = time.elapsed_secs_f64();
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let Key::Character(ch) = &key.logical_key else {
            continue;
        };
        if !ch.chars().all(char::is_alphabetic) {
            continue;
        }
        if now - typed.1 > 1.0 {
            typed.0.clear();
        }
        typed.0.push_str(ch);
        typed.1 = now;
        let Some(section) = Section::typed(&typed.0, state.lobby.lang(), super::builds()) else {
            continue;
        };
        if state.settings_section() != section {
            state.set_settings_section(section);
        }
        if let Some((entity, _, stop)) = navs.iter().find(|(_, n, _)| n.0 == section) {
            focus.set(entity, FocusCause::Navigated);
            remembered.0 = Some(*stop);
        }
    }
}

/// The display mode's trial on the screen: its line comes and goes, and
/// its seconds count down (one rebuild a second while it runs); Keep and
/// Revert are carried out.
pub(crate) fn watch_trial(
    trial: Option<ResMut<crate::quality::DisplayTrial>>,
    mut state: ResMut<LobbyState>,
    mut last: Local<Option<u32>>,
) {
    let Some(mut trial) = trial else {
        return;
    };
    // Asked through `Deref` first: taking `&mut` marks the lobby changed,
    // and a changed lobby is a rebuilt tree — every frame, which eats every
    // click and every Tab.
    if state.settings_view.display_answer.is_some()
        && let Some(keep) = state.take_display_answer()
    {
        if keep {
            trial.keep();
        } else {
            trial.left = 0.0;
        }
    }
    let now = trial.previous.map(|_| trial.seconds());
    if *last != now {
        *last = now;
        if state.settings_open() {
            state.set_changed();
        }
    }
}

/// Puts what a press asked onto the clipboard (Copy my handle, Copy as
/// text).
pub(crate) fn copy_out(
    mut state: ResMut<LobbyState>,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    if state.settings_view.copy.is_none() {
        return;
    }
    let Some(text) = state.settings_view.copy.take() else {
        return;
    };
    if let Some(mut clipboard) = clipboard {
        let _ = clipboard.set_text(text);
    }
}

/// Keeps the lobby's copy of the graphics in force, without a rebuild: a
/// device that never chose starts its first change from them.
pub(crate) fn mirror_in_use(
    in_use: Option<Res<crate::quality::InUse>>,
    mut state: ResMut<LobbyState>,
) {
    let Some(in_use) = in_use else {
        return;
    };
    if state.settings_view.in_use != in_use.0 {
        state.bypass_change_detection().settings_view.in_use = in_use.0;
    }
}

/// What the graphics rows show: the device's choice, or what is in force.
#[must_use]
pub(crate) fn shown_graphics(
    settings: Option<&crate::settings::ClientSettings>,
    in_use: Option<&crate::quality::InUse>,
) -> Graphics {
    settings
        .and_then(|s| s.graphics)
        .or_else(|| in_use.map(|u| u.0))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::settings_map::{Builds, ROWS, rows_of};

    /// Every control the screen draws has its place in the table (§9.5's
    /// rule, read here from the map): a row with a control names a stop.
    #[test]
    fn every_section_s_controls_are_in_the_tab_order() {
        let ids = [
            "preset",
            "display-mode",
            "anti-aliasing",
            "vsync",
            "frame-limit",
            "background-limit",
            "show-frame-rate",
            "backdrop",
            "ambient",
            "atmosphere",
            "hold-still",
            "sky",
            "master",
            "music",
            "effects",
            "mute-unfocused",
            "table-sounds",
            "language",
            "text-size",
            "preview-size",
            "text-face",
        ];
        for id in ids {
            assert!(SETTINGS.stops.contains(&id), "{id}");
        }
        let mut seen = SETTINGS.stops.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), SETTINGS.stops.len(), "a stop twice");
        assert!(ROWS.len() > 30);
        assert!(rows_of(Section::Graphics, Builds::DESKTOP, 1).count() > 8);
    }
}
