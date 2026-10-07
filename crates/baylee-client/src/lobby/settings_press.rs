//! The settings screen's presses: what each control `crate::settingsui`
//! and `crate::seatpanel` draw does when it is clicked.

use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// A control of the settings screen: keys, automation, the table's
/// looks and sounds, the language-model seat's panel and the account's
/// deletion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SettingsPress {
    /// A control of the language-model seat's panel on the settings screen.
    Seat(baylee_client_core::llmseat::panel::Act),
    /// A control of a profile's key box on that panel.
    SeatKey(crate::seatpanel::KeyPress),
    /// Open the settings screen.
    OpenSettings,
    /// Leave it.
    CloseSettings,
    /// Wait for a key and bind it to this action.
    Rebind(baylee_client_core::prefs::Action),
    /// Put one action back to its default key.
    ResetBinding(baylee_client_core::prefs::Action),
    /// Put every key back.
    ResetAllBindings,
    /// Flip one automation switch.
    ToggleAuto(baylee_client_core::prefs::AutoRule),
    /// Return every card ability to manual responses.
    ResetAbilityOrders,
    /// Forget one ability's policy.
    ForgetAbility(baylee_core::ids::AbilityRef),
    /// Stop the table moving, or let it move again.
    ToggleMotion,
    /// Put a sky behind the table, or let the clock choose one.
    PickSky(baylee_client_core::sky::SkyMode),
    /// Turn the table up, down, or off.
    PickSound(baylee_client_core::cue::Loudness),
    /// Put weather in the air over the table, or take it away.
    PickAtmosphere(baylee_client_core::atmosphere::Atmosphere),
    /// Turn one step of the phase rail red or green.
    ToggleRail(
        baylee_client_core::automation::RailSide,
        baylee_client_core::automation::RailRow,
    ),
    /// Put the whole rail to a preset.
    SetRail(baylee_client_core::automation::RailPreset),
    /// Open the confirmation that deletes the account (#292).
    AskToDeleteAccount,
    /// Send the deletion.
    ConfirmAccountDeletion,
    /// Close the confirmation, deleting nothing.
    CancelAccountDeletion,
    /// Show a section (the nav).
    Section(baylee_client_core::settings_map::Section),
    /// Show a search result's section.
    Jump(baylee_client_core::settings_map::Row),
    /// Put a section's rows back.
    ResetSection(baylee_client_core::settings_map::Section),
    /// Graphics: write a preset's knobs (the device's rows only).
    GraphicsPreset(baylee_client_core::graphics::Preset),
    /// Graphics: the window's mode (a trial of 15 s).
    DisplayMode(baylee_client_core::graphics::DisplayMode),
    /// Keep (`true`) or revert the display mode on trial.
    KeepDisplay(bool),
    /// Graphics: edge smoothing.
    AntiAliasing(baylee_client_core::graphics::AntiAliasing),
    /// Graphics: vsync.
    VSync(baylee_client_core::graphics::VSync),
    /// Graphics: the frame limit.
    FrameLimit(baylee_client_core::graphics::FrameLimit),
    /// Graphics: the background limit.
    BackgroundLimit(baylee_client_core::graphics::BackgroundLimit),
    /// Graphics: the frame-rate counter.
    ShowFrameRate,
    /// Graphics: the backdrop.
    Backdrop(baylee_client_core::graphics::Backdrop),
    /// Graphics: ambient detail.
    Ambient(baylee_client_core::graphics::Effects),
    /// Audio: silent behind other windows.
    MuteUnfocused,
    /// Audio: the priority cue's own switch (D22).
    PriorityCue,
    /// Graphics: the ring's lean (D20).
    TableLean(baylee_client_core::tableview::RingLean),
    /// Graphics: where a visit stands (D21).
    VisitCamera(baylee_client_core::tableview::VisitCamera),
    /// Graphics: the default arrangement (DESIGN-v8 §2.4).
    Arrangement(baylee_client_core::tableview::Arrangement),
    /// Graphics: *Tisch folgt dem Zug* (DESIGN-v8 §1.1).
    FollowTurn,
    /// Graphics: the arrangement remembered for a seat count, stepped by
    /// one through *Default* and the eight (DESIGN-v8 §2.6).
    ArrangementForSeats(u8, i8),
    /// Display: the text step.
    TextSize(crate::shellkit::TextSize),
    /// Display: the preview a tenth larger or smaller.
    PreviewSize(i8),
    /// Display: the text face.
    TextFace,
    /// Account: the handle onto the clipboard.
    CopyHandle,
    /// Network: back to the gateway list.
    SwitchGateway,
    /// Network: the diagnostics onto the clipboard.
    CopyDiagnostics,
    /// Privacy: what a report may carry.
    Consent(baylee_client_core::bugreport::Category),
    /// Privacy: crash reports.
    Crash(baylee_client_core::bugreport::CrashConsent),
    /// Controls: wait for a key for this shortcut.
    RebindShell(baylee_client_core::shellkeys::ShellAction),
    /// Controls: a shortcut back to its default.
    ResetShell(baylee_client_core::shellkeys::ShellAction),
    /// Controls: take the refused key from its holder (the second request).
    TakeShell,
    /// Language models: this profile's sheet.
    OpenProfile(usize),
    /// Language models: the sheet put away.
    CloseProfile,
    /// Language models: the sheet's Advanced fields shown or hidden.
    ProfileAdvanced,
}

/// The device's graphics, starting from what is in force when the device
/// never chose, changed by `change` (`Graphics::adjust` names the preset).
fn graphics(
    state: &LobbyState,
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
    change: impl FnOnce(&mut baylee_client_core::graphics::Graphics),
) -> bool {
    let Some(settings) = settings.as_mut() else {
        return false;
    };
    let mut g = settings.graphics.unwrap_or(state.settings_view.in_use);
    g.adjust(change);
    if settings.graphics == Some(g) {
        return false;
    }
    settings.graphics = Some(g);
    settings.save();
    true
}

/// A device setting, written and saved when it differs.
fn device(
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
    change: impl FnOnce(&mut crate::settings::ClientSettings) -> bool,
) -> bool {
    let Some(settings) = settings.as_mut() else {
        return false;
    };
    let changed = change(settings);
    if changed {
        settings.save();
    }
    changed
}

impl SettingsPress {
    /// A press on a device row (Graphics, Audio, Display, Privacy): the
    /// settings file written when it differs, and whether it changed —
    /// `None` for every other press.
    fn on_device(
        self,
        state: &LobbyState,
        settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
    ) -> Option<bool> {
        Some(match self {
            SettingsPress::GraphicsPreset(preset) => {
                graphics(state, settings, |g| *g = g.with_preset(preset))
            }
            SettingsPress::DisplayMode(mode) => {
                graphics(state, settings, |g| g.display_mode = mode)
            }
            SettingsPress::AntiAliasing(aa) => graphics(state, settings, |g| g.anti_aliasing = aa),
            SettingsPress::VSync(vsync) => graphics(state, settings, |g| g.vsync = vsync),
            SettingsPress::FrameLimit(limit) => {
                graphics(state, settings, |g| g.frame_limit = limit)
            }
            SettingsPress::BackgroundLimit(limit) => {
                graphics(state, settings, |g| g.background_limit = limit)
            }
            SettingsPress::ShowFrameRate => {
                graphics(state, settings, |g| g.show_frame_rate = !g.show_frame_rate)
            }
            SettingsPress::Backdrop(backdrop) => {
                graphics(state, settings, |g| g.backdrop = backdrop)
            }
            SettingsPress::Ambient(effects) => graphics(state, settings, |g| g.effects = effects),
            SettingsPress::MuteUnfocused => device(settings, |s| {
                s.audio.mute_in_background = !s.audio.mute_in_background;
                true
            }),
            SettingsPress::PriorityCue => device(settings, |s| {
                s.audio.priority_cue = !s.audio.priority_cue;
                true
            }),
            SettingsPress::TableLean(lean) => device(settings, |s| {
                let differs = s.table.lean != lean;
                s.table.lean = lean;
                differs
            }),
            SettingsPress::VisitCamera(visit) => device(settings, |s| {
                let differs = s.table.visit != visit;
                s.table.visit = visit;
                differs
            }),
            SettingsPress::Arrangement(arrangement) => device(settings, |s| {
                let differs = s.table.arrangement != arrangement;
                s.table.arrangement = arrangement;
                differs
            }),
            SettingsPress::FollowTurn => device(settings, |s| {
                s.table.follow = !s.table.follow;
                true
            }),
            SettingsPress::ArrangementForSeats(seats, step) => device(settings, |s| {
                let seats = usize::from(seats);
                let now = s.table.arrangement_by_seats.get(seats);
                let next = crate::arrangement::step_remembered(now, step);
                s.table.arrangement_by_seats.set(seats, next);
                next != now
            }),
            SettingsPress::TextSize(size) => device(settings, |s| {
                let differs = s.text_size != size;
                s.text_size = size;
                differs
            }),
            SettingsPress::PreviewSize(step) => device(settings, |s| {
                let next = (s.preview_scale + f32::from(step) * 0.1).clamp(0.6, 2.0);
                let differs = (next - s.preview_scale).abs() > f32::EPSILON;
                s.preview_scale = next;
                differs
            }),
            SettingsPress::TextFace => device(settings, |s| {
                s.prefer_text_view = !s.prefer_text_view;
                true
            }),
            SettingsPress::Consent(category) => device(settings, |s| {
                let now = s.reports.allows(category);
                s.reports.set(category, !now);
                true
            }),
            SettingsPress::Crash(crash) => device(settings, |s| {
                let differs = s.reports.crashes != crash;
                s.reports.crashes = crash;
                differs
            }),
            _ => return None,
        })
    }

    /// What a click on this control does.
    /// A press on the screen itself (its nav, Reset this section, the
    /// display trial, the copies, the shell keys): whether it was one.
    fn on_screen(
        self,
        state: &mut ResMut<LobbyState>,
        prefs: &mut ResMut<crate::prefs::Prefs>,
        scrolled: &mut ResMut<Scrolled>,
        mailbox: &Mailbox,
        settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
    ) -> bool {
        match self {
            SettingsPress::Section(section) => {
                crate::settingsui::show(state, section);
                scrolled.set(List::Settings, 0.0);
            }
            SettingsPress::Jump(row) => {
                crate::settingsui::show(state, baylee_client_core::settings_map::of(row).section);
                scrolled.set(List::Settings, 0.0);
            }
            SettingsPress::ResetSection(section) => {
                crate::settingsui::reset_section(section, prefs, settings.as_deref_mut());
                state.set_changed();
            }
            SettingsPress::KeepDisplay(keep) => {
                state.settings_view.display_answer = Some(keep);
            }
            SettingsPress::OpenProfile(at) => {
                state
                    .seat
                    .act(baylee_client_core::llmseat::panel::Act::Select(at));
                state.settings_view.profile_sheet = true;
                scrolled.set(List::ProfileSheet, 0.0);
            }
            SettingsPress::CloseProfile => {
                state.seat.blur();
                state.settings_view.profile_sheet = false;
            }
            SettingsPress::ProfileAdvanced => {
                state.settings_view.profile_advanced = !state.settings_view.profile_advanced;
            }
            SettingsPress::CopyHandle => {
                if let Some(me) = state.lobby.me() {
                    state.settings_view.copy = Some(me.handle.clone());
                }
            }
            SettingsPress::CopyDiagnostics => {
                state.settings_view.copy = Some(state.diagnostics());
            }
            SettingsPress::SwitchGateway => {
                state.settings = SettingsPane::Closed;
                if state.lobby.token().is_some() || state.lobby.offline() {
                    super::clicks::sign_out(state, prefs, scrolled, mailbox, settings);
                }
                if state.lobby.gateway_chosen() {
                    state.leave_gateway();
                }
            }
            SettingsPress::RebindShell(action) => {
                state.settings_view.refused = None;
                state.settings = if state.settings.capturing_shell() == Some(action) {
                    SettingsPane::Open
                } else {
                    SettingsPane::RebindingShell(action)
                };
            }
            SettingsPress::ResetShell(action) => {
                state.settings_view.refused = None;
                prefs.edit().shell_keys.reset(action);
            }
            SettingsPress::TakeShell => {
                if let Some((action, chord, _)) = state.settings_view.refused.take() {
                    let _ = prefs.edit().shell_keys.take(action, chord);
                    state.settings = SettingsPane::Open;
                }
            }
            SettingsPress::Seat(act) => {
                use baylee_client_core::llmseat::panel::Act;
                state.seat.act(act);
                // A new profile is filled in on its sheet; a removed one's
                // sheet has nothing left to show.
                match act {
                    Act::Add | Act::AddPreset(_) | Act::Duplicate(_) => {
                        state.settings_view.profile_sheet = true;
                        scrolled.set(List::ProfileSheet, 0.0);
                    }
                    Act::Remove(_) => state.settings_view.profile_sheet = false,
                    _ => {}
                }
            }
            _ => return false,
        }
        true
    }

    #[allow(clippy::too_many_lines)] // one arm per settings row, the device's listed as answered
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            mailbox,
            settings,
            scrolled,
        } = cx;
        // A device row draws from the settings file, which is no rebuild by
        // itself: a press that changed one asks for the redraw.
        if let Some(changed) = self.on_device(state, settings) {
            if changed {
                state.set_changed();
            }
            return;
        }
        if self.on_screen(state, prefs, scrolled, mailbox, settings) {
            return;
        }
        match self {
            SettingsPress::OpenSettings if !state.settings.is_open() => {
                state.settings = SettingsPane::Open;
            }
            SettingsPress::CloseSettings if state.settings.is_open() => {
                state.settings = SettingsPane::Closed;
            }
            SettingsPress::SeatKey(key) => state.seat.key_press(key),
            SettingsPress::AskToDeleteAccount => state.lobby.ask_to_delete_account(),
            SettingsPress::CancelAccountDeletion => state.lobby.cancel_account_deletion(),
            SettingsPress::ConfirmAccountDeletion => {
                let request = state.lobby.delete_account();
                dispatch(state, mailbox, request);
            }
            SettingsPress::Rebind(action) => {
                // Tapping the armed row again disarms it, so the chip is its
                // own cancel and there is no way to get stuck waiting.
                state.settings = if state.settings.capturing() == Some(action) {
                    SettingsPane::Open
                } else {
                    SettingsPane::Rebinding(action)
                };
            }
            SettingsPress::ResetBinding(action) => prefs.edit().keymap.reset(action),
            SettingsPress::ResetAllBindings => {
                prefs.edit().keymap = baylee_client_core::prefs::Keymap::standard();
            }
            SettingsPress::ResetAbilityOrders => prefs.edit().ability_orders.clear(),
            SettingsPress::ForgetAbility(ability) => prefs
                .edit()
                .ability_orders
                .retain(|order| order.ability != ability),
            SettingsPress::ToggleAuto(rule) => {
                let mut edit = prefs.edit();
                rule.toggle(&mut edit.auto);
            }
            SettingsPress::ToggleMotion => {
                let mut edit = prefs.edit();
                edit.reduce_motion = !edit.reduce_motion;
            }
            // A choice already made is no edit: `edit` schedules a write-back
            // to the gateway as well as marking the preferences changed.
            SettingsPress::PickSky(mode) if prefs.all().sky != mode => prefs.edit().sky = mode,
            SettingsPress::PickSound(level) if prefs.all().sound != level => {
                prefs.edit().sound = level;
            }
            SettingsPress::PickAtmosphere(air) if prefs.all().atmosphere != air => {
                prefs.edit().atmosphere = air;
            }
            SettingsPress::ToggleRail(side, row) => prefs.edit().orders.toggle(side, row),
            SettingsPress::SetRail(preset) => prefs.edit().orders.set_to(preset),
            // Device rows and the screen's own, answered above.
            SettingsPress::Section(_)
            | SettingsPress::Jump(_)
            | SettingsPress::ResetSection(_)
            | SettingsPress::KeepDisplay(_)
            | SettingsPress::OpenProfile(_)
            | SettingsPress::Seat(_)
            | SettingsPress::CloseProfile
            | SettingsPress::ProfileAdvanced
            | SettingsPress::CopyHandle
            | SettingsPress::CopyDiagnostics
            | SettingsPress::SwitchGateway
            | SettingsPress::RebindShell(_)
            | SettingsPress::ResetShell(_)
            | SettingsPress::TakeShell
            | SettingsPress::GraphicsPreset(_)
            | SettingsPress::DisplayMode(_)
            | SettingsPress::AntiAliasing(_)
            | SettingsPress::VSync(_)
            | SettingsPress::FrameLimit(_)
            | SettingsPress::BackgroundLimit(_)
            | SettingsPress::ShowFrameRate
            | SettingsPress::Backdrop(_)
            | SettingsPress::Ambient(_)
            | SettingsPress::MuteUnfocused
            | SettingsPress::PriorityCue
            | SettingsPress::TableLean(_)
            | SettingsPress::VisitCamera(_)
            | SettingsPress::Arrangement(_)
            | SettingsPress::FollowTurn
            | SettingsPress::ArrangementForSeats(..)
            | SettingsPress::TextSize(_)
            | SettingsPress::PreviewSize(_)
            | SettingsPress::TextFace
            | SettingsPress::Consent(_)
            | SettingsPress::Crash(_)
            | SettingsPress::OpenSettings
            | SettingsPress::CloseSettings
            | SettingsPress::PickSky(_)
            | SettingsPress::PickSound(_)
            | SettingsPress::PickAtmosphere(_) => {}
        }
    }
}
