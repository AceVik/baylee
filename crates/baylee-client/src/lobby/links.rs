//! What the lobby does with a `baylee:` link (`docs/client.md` §"Links from
//! the website"). The link waits in [`crate::links::Arrived`] until the
//! lobby is at rest: never in a game (the lobby's systems do not run there),
//! never over a question already open, never while a request is under way.
//!
//! A link never acts by itself. `open` only brings the window forward.
//! `signin` opens Baylee Central's sign-in where nobody is signed in. A
//! gateway link asks first, with the address in full: signed in at Central,
//! the gateway joins the account's list; signed out, Central's sign-in comes
//! first and the question after it, and `‹` back from that sign-in asks for
//! this device's list instead; signed in at a gateway with accounts of its
//! own, or playing offline, it joins this device's list.

#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use crate::links::Arrived;
use baylee_client_core::deeplink::Link;

/// How many gateways an account's document holds
/// (`baylee_account::data::AccountData`): a link never asks for one more.
pub(super) const ACCOUNT_GATEWAYS: usize = 128;

/// Follows the link that waits, once the lobby can.
pub(super) fn follow(mut state: ResMut<LobbyState>, mut arrived: ResMut<Arrived>) {
    // Read through `Deref` first: a frame with nothing to do writes nothing.
    let Some(link) = arrived.0.clone() else {
        return;
    };
    if !at_rest(&state) {
        return;
    }
    let done = match link {
        Link::Open => true,
        Link::SignIn => {
            if signed_out_at_the_door(&state) {
                open_central(&mut state);
            }
            true
        }
        Link::AddGateway(url) => propose(&mut state, url),
    };
    if done {
        arrived.0 = None;
    }
}

/// Nothing is open over the lobby and nothing is under way: a question may
/// stand up now without hiding another or being hidden by one.
fn at_rest(state: &LobbyState) -> bool {
    state.confirmation.is_none()
        && state.lobby.deleting_account().is_none()
        && !state.about_open
        && !state.terms.up()
        && !super::central::terms_pending(state)
        && !state.lobby.busy()
        && state.adding.is_none()
        && matches!(
            state.lobby.screen(),
            Screen::SignIn { .. } | Screen::Table | Screen::Build
        )
}

/// Nobody is signed in, anywhere: not at Central (nor on its way), not at a
/// gateway, not playing offline.
fn signed_out_at_the_door(state: &LobbyState) -> bool {
    !state.central_ready()
        && !state.central_pending()
        && state.lobby.token().is_none()
        && state.offline.is_none()
        && matches!(state.lobby.screen(), Screen::SignIn { .. })
}

/// Central's sign-in, with `‹` back to the device's gateways.
fn open_central(state: &mut LobbyState) {
    if state.gateway_selected {
        state.leave_gateway();
    }
    super::central::from_link(state);
}

/// A gateway link: the question, or what stands before it. Answers whether
/// the link is done with.
fn propose(state: &mut LobbyState, url: String) -> bool {
    if state.central_pending() {
        // Signing in at Central: asked once the account's list is back.
        return false;
    }
    if signed_out_at_the_door(state) && !state.link_asked_central {
        state.link_asked_central = true;
        open_central(state);
        // The form says why it is up, and what `‹` does instead.
        state.lobby.tell(Phrase::LinkGatewaySignInFirst, &[&url]);
        return false;
    }
    state.link_asked_central = false;
    // Signed in at Central this is the account's list, otherwise the
    // device's: the list the player is using.
    if state.gateways.contains(&url) {
        state.lobby.tell(Phrase::LinkGatewayKnown, &[&url]);
        return true;
    }
    if state.central_ready() && state.gateways.len() >= ACCOUNT_GATEWAYS {
        state.lobby.tell_refusal(Phrase::LinkGatewayListFull, &[]);
        return true;
    }
    state.confirmation = Some(confirm::Destructive::AddGateway(url));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::lobby::gateway_info::Probe;

    fn lobby() -> App {
        let mut app = App::new();
        app.insert_resource(LobbyState::from_settings(
            crate::settings::ClientSettings::default(),
        ))
        .init_resource::<Arrived>()
        .add_systems(Update, follow);
        app
    }

    fn send(app: &mut App, link: Link) {
        app.world_mut().resource_mut::<Arrived>().0 = Some(link);
        app.update();
    }

    fn state(app: &App) -> &LobbyState {
        app.world().resource::<LobbyState>()
    }

    const ORIGIN: &str = "https://gateway.example.org";

    fn asking(app: &App) -> Option<&str> {
        match &state(app).confirmation {
            Some(confirm::Destructive::AddGateway(url)) => Some(url.as_str()),
            _ => None,
        }
    }

    #[test]
    fn signed_out_a_gateway_link_opens_central_first_and_keeps_the_link() {
        let mut app = lobby();
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        assert!(state(&app).central_pending(), "Central's sign-in is up");
        assert!(state(&app).central_bound(), "with a way back");
        assert!(
            state(&app).lobby.status().contains(ORIGIN),
            "the form says which gateway waits"
        );
        assert_eq!(asking(&app), None, "nothing is asked over the sign-in");
        assert!(
            app.world().resource::<Arrived>().0.is_some(),
            "the link is kept"
        );
        // Still waiting, frame after frame, while the player signs in.
        app.update();
        app.update();
        assert_eq!(asking(&app), None);
        assert!(app.world().resource::<Arrived>().0.is_some());
    }

    #[test]
    fn back_from_that_sign_in_the_device_list_is_asked_about() {
        let mut app = lobby();
        let device = state(&app).gateways.clone();
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        app.world_mut().resource_mut::<LobbyState>().leave_gateway();
        assert!(!state(&app).central_pending());
        assert_eq!(
            state(&app).gateways,
            device,
            "the device's own list is back"
        );
        app.update();
        assert_eq!(asking(&app), Some(ORIGIN));
        assert!(app.world().resource::<Arrived>().0.is_none());
        // Nothing is added by asking.
        assert!(!state(&app).gateways.iter().any(|g| g == ORIGIN));
    }

    #[test]
    fn signed_in_at_central_the_question_comes_at_once() {
        let mut app = lobby();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            super::super::central::restored_for_test(&mut state, vec![gateway_list::PINNED.into()]);
        }
        assert!(state(&app).central_ready());
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        assert_eq!(asking(&app), Some(ORIGIN));
        assert_eq!(state(&app).gateways, vec![gateway_list::PINNED.to_string()]);
    }

    #[test]
    fn a_gateway_already_saved_is_said_and_not_asked_about() {
        let mut app = lobby();
        app.world_mut()
            .resource_mut::<LobbyState>()
            .link_asked_central = true;
        app.world_mut()
            .resource_mut::<LobbyState>()
            .gateways
            .push(ORIGIN.into());
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        assert_eq!(asking(&app), None);
        assert!(app.world().resource::<Arrived>().0.is_none());
        assert!(state(&app).lobby.status().contains(ORIGIN));
    }

    #[test]
    fn a_full_account_list_refuses_a_link() {
        let mut app = lobby();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            let full = (0..ACCOUNT_GATEWAYS)
                .map(|n| format!("https://g{n}.example.org"))
                .collect();
            super::super::central::restored_for_test(&mut state, full);
        }
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        assert_eq!(asking(&app), None);
        assert_eq!(state(&app).gateways.len(), ACCOUNT_GATEWAYS);
    }

    #[test]
    fn a_link_waits_while_another_question_is_open() {
        let mut app = lobby();
        app.world_mut().resource_mut::<LobbyState>().confirmation = Some(
            confirm::Destructive::ForgetGateway(gateway_list::PINNED.into()),
        );
        send(&mut app, Link::SignIn);
        assert!(
            !state(&app).central_pending(),
            "nothing moved under the question"
        );
        assert_eq!(app.world().resource::<Arrived>().0, Some(Link::SignIn));
        app.world_mut().resource_mut::<LobbyState>().confirmation = None;
        app.update();
        assert!(state(&app).central_pending());
        assert_eq!(app.world().resource::<Arrived>().0, None);
    }

    #[test]
    fn a_sign_in_link_never_moves_a_player_who_is_signed_in() {
        let mut app = lobby();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            super::super::central::restored_for_test(&mut state, vec![gateway_list::PINNED.into()]);
        }
        send(&mut app, Link::SignIn);
        assert!(state(&app).central_ready());
        assert_eq!(app.world().resource::<Arrived>().0, None);
        send(&mut app, Link::Open);
        assert_eq!(app.world().resource::<Arrived>().0, None);
    }

    /// The whole of yes: the question's answer asks the address about
    /// itself, and only an answering gateway joins the list.
    #[test]
    fn yes_asks_the_address_and_only_a_gateway_that_answers_is_saved() {
        let mut app = lobby();
        app.world_mut()
            .resource_mut::<LobbyState>()
            .link_asked_central = true;
        send(&mut app, Link::AddGateway(ORIGIN.into()));
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let Some(confirm::Destructive::AddGateway(url)) = state.confirmation.take() else {
            panic!("asked");
        };
        let asked = state.ask_gateway(url).expect("asked about itself");
        assert!(!state.gateway_answered(asked.clone(), Probe::Silent));
        assert!(!state.gateways.contains(&asked));
        let asked = state.ask_gateway(asked).expect("asked again");
        assert!(state.gateway_answered(asked.clone(), Probe::Older));
        assert!(state.gateways.contains(&asked));
    }
}
