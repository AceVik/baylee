//! Which door a native client's account form is: the gateway's own (sign-in,
//! sign-up and guests as its `GET /auth/config` offers them) for a gateway
//! that takes no Baylee Central sign-ins, and Central's, with a way back,
//! for one that does.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The chosen gateway's `GET /auth/config`, landed.
fn says(app: &mut App, central: bool) {
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
                guests: true,
                central,
            }),
        ));
    app.update();
    settle(app);
}

#[test]
fn a_gateway_without_central_keeps_its_own_sign_up_and_guests() {
    let mut app = headless();
    says(&mut app, false);
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
    says(&mut app, true);
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
