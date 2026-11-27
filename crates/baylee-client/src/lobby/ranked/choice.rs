//! Whom a ranked search accepts (`client_core::aiplayers::Opponents`): the
//! person's own, informed choice, never preselected.
//!
//! The first search of an account that has not chosen opens a sheet with
//! the three choices and nothing picked ([`ask`], drawn by
//! `confirm::draw_opponents`); only its confirmation searches. After that the
//! desk shows the choice as a segmented control, which changes it at any
//! time: while a search waits, the search moves with it ([`choose`]). The
//! choice is kept per Central account in the device's settings
//! ([`crate::settings::ClientSettings::ranked_opponents`]) and mirrored in
//! the page's own state, which a headless test without settings reads.
use super::{LobbyState, api, native, queue};
use crate::lobby::confirm::Destructive;
use crate::settings::ClientSettings;
use baylee_client_core::aiplayers::Opponents;
use bevy::prelude::*;

/// The signed-in Central account, as the settings key it by.
fn account(state: &LobbyState) -> Option<String> {
    state
        .central_handle()
        .and_then(|handle| handle.public_id)
        .map(|id| uuid::Uuid::from_bytes(id).to_string())
}

/// Reads the signed-in account's choice into the page, once per account:
/// another account signing in here has not chosen.
pub(super) fn recall(state: &mut LobbyState, settings: Option<&ClientSettings>) {
    let who = account(state);
    let ui = &mut state.ranked;
    if who.is_none() {
        ui.opponents = None;
        ui.opponents_of = None;
        return;
    }
    if let Some(settings) = settings {
        ui.opponents = who
            .as_deref()
            .and_then(|key| settings.ranked_opponents.of(key));
    } else if ui.opponents_of != who {
        ui.opponents = None;
    }
    ui.opponents_of = who;
}

/// Keeps `choice` for the signed-in account, in the page and the settings.
fn remember(
    state: &mut LobbyState,
    settings: &mut Option<ResMut<ClientSettings>>,
    choice: Opponents,
) {
    let who = account(state);
    state.ranked.opponents = Some(choice);
    state.ranked.opponents_of.clone_from(&who);
    if let (Some(settings), Some(key)) = (settings.as_mut(), who.as_deref())
        && settings.ranked_opponents.keep(key, choice)
    {
        settings.save();
    }
}

/// The signed-in account's choice as the page holds it; none while the page
/// still holds another account's, so one person's consent never searches
/// for the next.
pub(super) fn current(state: &LobbyState) -> Option<Opponents> {
    state
        .ranked
        .opponents
        .filter(|_| state.ranked.opponents_of == account(state))
}

/// Whether a search may start now: the account has chosen, or the search
/// is the restore of a waiting one, which keeps what its ticket accepts.
pub(super) fn chosen(state: &LobbyState) -> bool {
    current(state).is_some() || state.ranked_net.needs_deck
}

/// The first search: the sheet that asks, with nothing picked.
pub(super) fn ask(state: &mut ResMut<LobbyState>) {
    state.confirmation = Some(Destructive::RankedOpponents(None));
}

/// An option picked on the sheet; nothing is kept until it is confirmed.
pub(super) fn pick(state: &mut ResMut<LobbyState>, choice: Opponents) {
    if matches!(state.confirmation, Some(Destructive::RankedOpponents(_))) {
        state.confirmation = Some(Destructive::RankedOpponents(Some(choice)));
    }
}

/// The sheet's Search: keeps the choice and searches with it.
pub(in crate::lobby) fn confirm(
    state: &mut LobbyState,
    settings: &mut Option<ResMut<ClientSettings>>,
    choice: Opponents,
) {
    recall(state, settings.as_deref());
    remember(state, settings, choice);
    #[cfg(feature = "dev-control")]
    if super::visual_fixture::reply(state, super::Action::Join) {
        state.ranked.revision = state.ranked.revision.wrapping_add(1);
        return;
    }
    super::join(state);
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
}

/// The desk's control: keeps the choice and, while a search waits, moves it.
pub(super) fn choose(
    state: &mut LobbyState,
    settings: &mut Option<ResMut<ClientSettings>>,
    choice: Opponents,
) {
    recall(state, settings.as_deref());
    let before = state.ranked.opponents;
    remember(state, settings, choice);
    if before != Some(choice)
        && matches!(
            state.ranked_net.shown_queue(),
            api::QueueState::Waiting { .. }
        )
        && let Some((deck, _)) = queue::selected(state)
    {
        #[cfg(feature = "dev-control")]
        if super::visual_fixture::moved(state, choice) {
            state.ranked.revision = state.ranked.revision.wrapping_add(1);
            return;
        }
        native::switch(state, choice.wire(), &deck.to_string());
    }
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_chosen_until_the_person_chooses_and_the_sheet_keeps_nothing() {
        let mut app = App::new();
        app.insert_resource(LobbyState::new());
        app.add_systems(Update, |mut state: ResMut<LobbyState>| {
            if state.confirmation.is_none() {
                ask(&mut state);
            } else {
                pick(&mut state, Opponents::Local);
            }
        });
        app.update();
        let state = app.world().resource::<LobbyState>();
        assert!(!chosen(state), "a search without a choice asks first");
        assert!(matches!(
            state.confirmation,
            Some(Destructive::RankedOpponents(None))
        ));
        app.update();
        let state = app.world().resource::<LobbyState>();
        assert!(matches!(
            state.confirmation,
            Some(Destructive::RankedOpponents(Some(Opponents::Local)))
        ));
        assert_eq!(state.ranked.opponents, None, "picked is not chosen");
    }

    #[test]
    fn a_choice_is_the_accounts_and_is_not_read_for_another() {
        let mut state = LobbyState::new();
        state.ranked.opponents = Some(Opponents::Any);
        state.ranked.opponents_of = Some("someone-else".into());
        // Signed in nowhere: nothing is anyone's choice.
        recall(&mut state, None);
        assert_eq!(state.ranked.opponents, None);
        let mut settings = ClientSettings::default();
        settings
            .ranked_opponents
            .keep("someone-else", Opponents::Any);
        recall(&mut state, Some(&settings));
        assert_eq!(state.ranked.opponents, None);
    }
}
