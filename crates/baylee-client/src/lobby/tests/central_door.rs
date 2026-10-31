//! Which door a native client's account form is: the gateway's own (sign-in,
//! sign-up and guests as its `GET /auth/config` offers them) for a gateway
//! that takes no Baylee Central sign-ins, and Central's, with a way back,
//! for one that does; with that gateway's guest door beside it when it takes
//! guests too (`auth_mode` `mixed`).

#[allow(clippy::wildcard_imports)]
use super::*;

/// The chosen gateway's `GET /auth/config`, landed.
fn says(app: &mut App, central: bool, guests: bool) {
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Remote(
            0,
            Box::new(Reply::Registration {
                registration: Registration::Open,
                art_cache: false,
                guests,
                central,
            }),
        ));
    app.update();
    settle(app);
}

#[test]
fn a_gateway_without_central_keeps_its_own_sign_up_and_guests() {
    let mut app = headless();
    says(&mut app, false, true);
    assert!(!app.world().resource::<LobbyState>().central_pending());
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::ToggleRegistering)),
        "{found:?}"
    );
    assert!(
        found.contains(&Press::Front(FrontPress::GuestFace)),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Front(FrontPress::OpenCentralRegister)),
        "{found:?}"
    );
}

#[test]
fn a_gateway_with_central_turns_the_form_into_centrals_and_back_returns() {
    let mut app = headless();
    let device = vec!["https://device.invalid".to_string()];
    app.world_mut().resource_mut::<LobbyState>().gateways = device.clone();
    says(&mut app, true, false);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.central_pending());
        assert!(state.central_bound());
        assert_eq!(
            state.gateways,
            Vec::<String>::new(),
            "hidden until Central restores"
        );
    }
    let found = presses(&mut app);
    assert!(
        !found.contains(&Press::Front(FrontPress::ToggleRegistering)),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Front(FrontPress::GuestFace)),
        "{found:?}"
    );
    assert!(
        found.contains(&Press::Front(FrontPress::LeaveGateway)),
        "{found:?}"
    );
    // No sign-up of its own: the account is made on the website.
    assert!(
        found.contains(&Press::Front(FrontPress::OpenCentralRegister)),
        "{found:?}"
    );
    assert_eq!(
        crate::lobby::source::address_of(
            FrontPress::OpenCentralRegister,
            app.world().resource::<LobbyState>()
        )
        .as_deref(),
        Some("https://baylee.app/register")
    );

    app.world_mut().resource_mut::<LobbyState>().leave_gateway();
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.central_pending());
    assert!(!state.lobby.gateway_chosen());
    assert_eq!(state.gateways, device);
}

/// A gateway that takes Central's accounts and guests (`mixed`): Central's
/// form, with the gateway's guest door beside its sign-in. The door leaves
/// Central and opens the gateway's own guest face, the gateway still chosen,
/// so a `mixed` gateway is never a `central` one to a player signed out.
#[test]
fn a_mixed_gateway_offers_its_guest_door_beside_centrals_sign_in() {
    let mut app = headless();
    let device = vec!["https://device.invalid".to_string()];
    app.world_mut().resource_mut::<LobbyState>().gateways = device.clone();
    says(&mut app, true, true);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.central_bound());
        assert!(state.lobby.guest_offered());
    }
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::GuestFace)),
        "{found:?}"
    );
    assert!(
        found.contains(&Press::Front(FrontPress::OpenCentralRegister)),
        "{found:?}"
    );

    press(&mut app, Press::Front(FrontPress::GuestFace));
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.central_pending(), "Central is off again");
    assert!(state.gateway_selected);
    assert!(state.lobby.gateway_chosen(), "the gateway is still chosen");
    assert_eq!(state.lobby.face(), baylee_client_core::lobby::Face::Guest);
    assert_eq!(state.gateways, device, "the device's own gateways are back");
}
