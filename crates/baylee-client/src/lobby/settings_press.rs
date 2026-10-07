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
}

impl SettingsPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            mailbox,
            ..
        } = cx;
        match self {
            SettingsPress::OpenSettings if !state.settings.is_open() => {
                state.settings = SettingsPane::Open;
            }
            SettingsPress::CloseSettings if state.settings.is_open() => {
                state.settings = SettingsPane::Closed;
            }
            SettingsPress::Seat(act) => state.seat.act(act),
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
            SettingsPress::OpenSettings
            | SettingsPress::CloseSettings
            | SettingsPress::PickSky(_)
            | SettingsPress::PickSound(_)
            | SettingsPress::PickAtmosphere(_) => {}
        }
    }
}
