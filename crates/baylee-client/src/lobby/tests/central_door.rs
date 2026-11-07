//! Which door a native client's account form is: the gateway's own (sign-in,
//! sign-up and guests as its `GET /auth/config` offers them) for a gateway
//! that takes no Baylee Central sign-ins, and Central's, with a way back,
//! for one that does; with that gateway's guest door beside it when it takes
//! guests too (`auth_mode` `mixed`).

#[allow(clippy::wildcard_imports)]
use super::*;

/// The chosen gateway's `GET /auth/config`, landed.
fn says(app: &mut App, central: bool, guests: bool) {
    let epoch = app.world().resource::<LobbyState>().gateway_epoch;
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Remote(
            epoch,
            Box::new(Reply::Registration {
                registration: Registration::Open,
                art_cache: false,
                guests,
                central,
                guests_only: false,
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

#[test]
fn launch_starts_at_central_sign_in_with_website_and_register_links() {
    let mut app = headless();
    *app.world_mut().resource_mut::<LobbyState>() = LobbyState::new();
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(state.central_pending(), "launch begins at Central sign-in");
    assert!(state.lobby.gateway_chosen());
    assert_eq!(
        crate::lobby::source::address_of(FrontPress::OpenWebsite, state).as_deref(),
        Some("https://baylee.app")
    );
    assert_eq!(
        crate::lobby::source::address_of(FrontPress::OpenCentralRegister, state).as_deref(),
        Some("https://baylee.app/register")
    );
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::OpenCommunityDialog(
            super::super::front::CommunityKind::Website
        )))
    );
    assert!(found.contains(&Press::Front(FrontPress::OpenCentralRegister)));
    press(
        &mut app,
        Press::Front(FrontPress::OpenCommunityDialog(
            super::super::front::CommunityKind::Website,
        )),
    );
    app.update();
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::OpenWebsite)));
}

/// What a native launch leaves: Central's sign-in, and behind it the device's
/// own gateways, each with its answer in hand, so nothing here reaches a
/// network. No address is set either, as [`headless`] sets none.
fn launched(device: &[&str]) -> App {
    let mut app = headless();
    let mut state = LobbyState::new();
    state.gateway.clear();
    for url in device {
        state.probes.insert((*url).to_string(), Probe::Asking);
    }
    state
        .central
        .as_mut()
        .expect("a launch turns Central on")
        .local_gateways = device.iter().map(ToString::to_string).collect();
    *app.world_mut().resource_mut::<LobbyState>() = state;
    settle(&mut app);
    app
}

const WITHOUT_ACCOUNT: Press = Press::Front(FrontPress::WithoutAccount);
const BACK_TO_CENTRAL: Press = Press::Front(FrontPress::BackToCentral);

/// Nobody stands before a closed door: the form a launch opens has no `‹`,
/// and the door past it leads to the device's own gateways, whose rows are
/// chosen as on any list.
#[test]
fn the_form_a_launch_opens_has_a_door_to_the_devices_gateways() {
    let device = "https://device.invalid";
    let mut app = launched(&[device]);
    let found = presses(&mut app);
    assert!(found.contains(&WITHOUT_ACCOUNT), "{found:?}");
    assert!(
        !found.contains(&Press::Front(FrontPress::LeaveGateway)),
        "{found:?}"
    );

    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.central_pending(), "Central is off");
    assert!(!state.lobby.gateway_chosen(), "the list is up");
    assert_eq!(state.gateways, vec![device.to_string()]);
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::SelectGateway(0))),
        "{found:?}"
    );
    assert!(found.contains(&BACK_TO_CENTRAL), "{found:?}");
    assert!(!found.contains(&WITHOUT_ACCOUNT), "{found:?}");
}

/// The form goes as it stood, its door still under it and no `‹` come from
/// anywhere: the two panels stand on one stage while it leaves.
#[test]
fn the_form_a_launch_opens_leaves_the_stage_as_it_stood() {
    let mut app = launched(&["https://device.invalid"]);
    press(&mut app, WITHOUT_ACCOUNT);
    let found = presses(&mut app);
    assert!(found.contains(&WITHOUT_ACCOUNT), "{found:?}");
    assert!(
        found.contains(&Press::Front(FrontPress::OpenCentralRegister)),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Front(FrontPress::LeaveGateway)),
        "{found:?}"
    );
}

/// Central's form keeps the device's addresses to itself; the player's
/// leaving it is what lets them be asked.
#[test]
fn the_devices_gateways_are_asked_once_the_player_is_past_the_sign_in() {
    // A closed port on this machine: nothing leaves it.
    let device = "http://127.0.0.1:9";
    let mut app = launched(&[]);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .central
        .as_mut()
        .expect("Central")
        .local_gateways = vec![device.to_string()];
    assert!(
        !app.world()
            .resource::<LobbyState>()
            .probes
            .contains_key(device)
    );
    press(&mut app, WITHOUT_ACCOUNT);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .probes
            .contains_key(device)
    );
}

/// What was typed for Central is not for any gateway, and a refusal from it
/// is not news on the list. The username is the player's, and comes along.
#[test]
fn nothing_typed_for_central_stays_behind_the_door() {
    let mut app = launched(&["https://device.invalid"]);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_field(Field::Username, "synthetic");
        state.lobby.set_field(Field::Password, "synthetic password");
        state.lobby.say("A refusal of Central's");
    }
    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.field(Field::Password), "");
    assert_eq!(state.lobby.status(), "");
    assert_eq!(state.lobby.field(Field::Username), "synthetic");
}

/// The door's banner draws a line with its capital, as a toast does, and the
/// lobby keeps the words it was told.
#[test]
fn the_doors_banner_starts_with_a_capital() {
    let mut app = launched(&["https://device.invalid"]);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .say("this gateway takes no guests");
    settle(&mut app);
    let said = labels(&mut app);
    assert!(
        said.iter().any(|l| l == "This gateway takes no guests"),
        "{said:?}"
    );
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.status(),
        "this gateway takes no guests"
    );
}

/// A sign-in under way holds the door, as it holds `‹`: its answer is about
/// to change what the door would leave.
#[test]
fn a_sign_in_under_way_holds_the_door() {
    let mut app = launched(&["https://device.invalid"]);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_field(Field::Username, "synthetic");
        state.lobby.set_field(Field::Password, "synthetic password");
        assert!(state.lobby.submit().is_some());
        assert!(state.lobby.busy());
    }
    app.update();
    assert!(!presses(&mut app).contains(&WITHOUT_ACCOUNT));
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    super::super::central::without_account(&mut state);
    assert!(state.central_pending(), "the sign-in still stands");
}

/// Back from the list is the form a launch opens, door and all, and the list
/// comes back with what its addresses said: they are not asked twice.
#[test]
fn the_list_goes_back_to_the_form_a_launch_opens() {
    let device = "https://device.invalid";
    let mut app = launched(&[device]);
    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    press(&mut app, BACK_TO_CENTRAL);
    settle(&mut app);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.central_pending());
        assert!(!state.central_bound(), "the form with no `‹`");
        assert!(state.lobby.gateway_chosen());
        assert_eq!(state.gateways, Vec::<String>::new(), "hidden again");
        assert!(matches!(state.probes.get(device), Some(Probe::Asking)));
    }
    let found = presses(&mut app);
    assert!(found.contains(&WITHOUT_ACCOUNT), "{found:?}");
    assert!(!found.contains(&BACK_TO_CENTRAL), "{found:?}");
    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.gateways, vec![device.to_string()]);
    assert!(matches!(state.probes.get(device), Some(Probe::Asking)));
}

/// The list reaches a gateway that takes guests, and the guest comes in on
/// its own face: nothing is signed in or accepted on the way. The gateway's
/// `‹` leads to the list again, with its way back to the sign-in.
#[test]
fn a_gateway_that_takes_guests_is_reached_past_the_sign_in() {
    let mut app = launched(&["https://guests.invalid"]);
    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    settle(&mut app);
    says(&mut app, false, true);
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::GuestFace)));
    press(&mut app, Press::Front(FrontPress::GuestFace));
    settle(&mut app);
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.face(), baylee_client_core::lobby::Face::Guest);
        assert!(state.lobby.token().is_none(), "a face, not yet a guest");
        assert!(!state.terms.up(), "and no terms taken for the player");
    }
    press(&mut app, Press::Front(FrontPress::BackToSignIn));
    settle(&mut app);
    press(&mut app, Press::Front(FrontPress::LeaveGateway));
    settle(&mut app);
    assert!(presses(&mut app).contains(&BACK_TO_CENTRAL));
}

/// A gateway on the list that takes Central's accounts asks for Central's
/// form as ever: with its `‹`, so no second way off it.
#[test]
fn a_central_gateway_past_the_sign_in_opens_centrals_form_with_its_way_back() {
    let mut app = launched(&["https://central.invalid"]);
    press(&mut app, WITHOUT_ACCOUNT);
    settle(&mut app);
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    settle(&mut app);
    says(&mut app, true, false);
    assert!(app.world().resource::<LobbyState>().central_bound());
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::LeaveGateway)),
        "{found:?}"
    );
    assert!(!found.contains(&WITHOUT_ACCOUNT), "{found:?}");
}

/// Signing out comes back to the sign-in, door and all, and not to the list.
#[test]
fn signing_out_of_central_comes_back_to_the_form_with_its_door() {
    let mut app = launched(&[]);
    let (handle, _jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        super::super::central::restored_for_test(&mut state, Vec::new());
        state.central.as_mut().expect("Central").handle = Some(handle);
    }
    settle(&mut app);
    app.update();
    sign_out_from_the_header(&mut app);
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(state.central_pending());
    assert!(!state.central_bound());
    assert!(presses(&mut app).contains(&WITHOUT_ACCOUNT));
}

/// By keys alone: Esc does not leave the form a launch opens, Tab reaches the
/// door and Enter goes through it, and the same way back from the list.
#[test]
fn the_door_and_the_way_back_answer_the_keys() {
    use super::front_keys::{clears_a_despawned_focus, focused_id, press_key, tab};
    fn walk_to(app: &mut App, stop: &str) {
        for _ in 0..40 {
            if focused_id(app) == Some(stop) {
                return;
            }
            tab(app, false);
        }
        panic!("Tab never reaches {stop}");
    }
    let mut app = launched(&["https://device.invalid"]);
    clears_a_despawned_focus(&mut app);
    press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(
        app.world().resource::<LobbyState>().central_pending(),
        "Esc is no way past the sign-in"
    );
    walk_to(&mut app, "without-account");
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    settle(&mut app);
    assert!(!app.world().resource::<LobbyState>().central_pending());
    walk_to(&mut app, "central");
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    settle(&mut app);
    assert!(app.world().resource::<LobbyState>().central_pending());
}

#[test]
fn central_gateway_selection_stays_on_gateway_panel_with_note_tone_and_lazy_groups() {
    use baylee_client_core::lobby::gateway_info::GatewayInfo;
    use baylee_client_core::lobby::gateway_list::GatewayGroup;
    let gateway_info = |name: &str, version: &str, view: u32| {
        Probe::Known(GatewayInfo {
            name: Some(name.to_string()),
            version: version.to_string(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            view_version: view,
            ..GatewayInfo::default()
        })
    };
    let mut app = headless();
    let (handle, _jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let mut urls = vec![
            "https://compat.example".to_string(),
            "https://incompat.example".to_string(),
            "https://offline.example".to_string(),
        ];
        urls.extend((0..16).map(|i| format!("https://extra-{i:02}.example")));
        super::super::central::restored_for_test(&mut state, urls);
        state.central.as_mut().expect("Central on").handle = Some(handle);
        let v = baylee_view::VIEW_VERSION;
        state.probes.insert(
            "https://compat.example".into(),
            gateway_info("Alpha Hall", "0.1.0", v),
        );
        state.probes.insert(
            "https://incompat.example".into(),
            gateway_info("Old Hall", "0.0.1", v + 1),
        );
        state
            .probes
            .insert("https://offline.example".into(), Probe::Silent);
        for i in 0..16 {
            state.probes.insert(
                format!("https://extra-{i:02}.example"),
                gateway_info(&format!("Extra {i:02}"), "0.1.0", v),
            );
        }
    }
    settle(&mut app);

    let found = presses(&mut app);
    assert!(found.contains(&Press::Front(FrontPress::SelectGateway(0))));
    assert!(!found.contains(&Press::Front(FrontPress::SelectGateway(1))));
    assert!(!found.contains(&Press::Front(FrontPress::SelectGateway(2))));
    assert!(!found.contains(&Press::Front(FrontPress::SelectGateway(18))));

    press(
        &mut app,
        Press::Front(FrontPress::ToggleGatewayGroup(GatewayGroup::Compatible)),
    );
    app.update();
    press(
        &mut app,
        Press::Front(FrontPress::ToggleGatewayGroup(GatewayGroup::Offline)),
    );
    app.update();
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::SelectGateway(2))));

    press(
        &mut app,
        Press::Front(FrontPress::ToggleGatewayGroup(GatewayGroup::Offline)),
    );
    press(
        &mut app,
        Press::Front(FrontPress::ToggleGatewayGroup(GatewayGroup::Compatible)),
    );
    app.update();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_field(Field::GatewaySearch, "Alpha");
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .visible_gateway_indices(),
        vec![0]
    );
    press(&mut app, Press::Front(FrontPress::ClearGatewaySearch));
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().gateway_collapsed,
        [false, true, true]
    );

    press(&mut app, Press::Front(FrontPress::SelectGateway(0)));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(state.central_admitting());
    assert_eq!(state.lobby.tone(), Tone::Note);
    assert!(!presses(&mut app).contains(&Press::Front(FrontPress::Submit)));
}

#[test]
fn central_sign_in_wears_lobby_header_allows_decks_before_gateway_and_refreshes_without_flicker() {
    use baylee_client_core::lobby::gateway_info::GatewayInfo;
    let mut app = headless();
    let (handle, _jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let urls = vec!["https://compat.example".to_string()];
        super::super::central::restored_for_test(&mut state, urls);
        let central = state.central.as_mut().expect("Central on");
        central.handle = Some(handle);
        central.profile_handle = Some("AceVik#0007".to_string());
        state.probes.insert(
            "https://compat.example".into(),
            Probe::Known(GatewayInfo {
                name: Some("Alpha Hall".to_string()),
                version: "0.1.0".to_string(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                view_version: baylee_view::VIEW_VERSION,
                ..GatewayInfo::default()
            }),
        );
    }
    settle(&mut app);

    let found = presses(&mut app);
    assert!(found.contains(&Press::Header(HeaderPress::Nav(0))));
    assert!(found.contains(&Press::Header(HeaderPress::Nav(1))));
    assert!(found.contains(&Press::Header(HeaderPress::Account)));
    assert!(found.contains(&Press::Front(FrontPress::SelectGateway(0))));
    assert!(labels(&mut app).iter().any(|l| l == "AceVik#0007"));

    // Switching to Decks in the Lobby header works before any gateway is chosen.
    press(&mut app, Press::Header(HeaderPress::Nav(1)));
    app.update();
    assert_eq!(app.world().resource::<LobbyState>().hub, Hub::Decks);
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table
    );

    // Switching back to Play returns to the Gateway Selection view in the Lobby.
    press(&mut app, Press::Header(HeaderPress::Nav(0)));
    app.update();
    assert!(app.world().resource::<LobbyState>().showing_gateway_hub());
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::SelectGateway(0))));

    // Refreshing gateways must preserve known probes on the refresh frame so rows
    // do not flicker into Offline while background probes are in flight.
    press(&mut app, Press::Front(FrontPress::RefreshGateways));
    assert!(matches!(
        app.world()
            .resource::<LobbyState>()
            .probes
            .get("https://compat.example"),
        Some(Probe::Known(_))
    ));
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::SelectGateway(0))));
}

/// Lands a reply as the mailbox would, a frame later.
fn land(app: &mut App, reply: Reply) {
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(reply);
    app.update();
    settle(app);
}

/// Signed in at Central with one gateway on the list, and that gateway being
/// admitted to. Its address is one no request can be built from (`headless`);
/// the jobs are what the account worker would have run.
fn admitting(app: &mut App) -> std::sync::mpsc::Receiver<super::super::central::Job> {
    let (handle, jobs) = super::super::central::Handle::queued_for_test();
    let mailbox = app.world().resource::<Mailbox>().clone();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        super::super::central::restored_for_test(&mut state, vec![String::new()]);
        state.central.as_mut().expect("Central on").handle = Some(handle);
        assert!(state.select_gateway(0), "the list holds the gateway");
        super::super::central::admit(&mut state, &mailbox);
        assert!(state.central_admitting());
    }
    app.update();
    settle(app);
    jobs
}

/// In a session at a gateway, signed in through Central.
fn in_a_central_session(app: &mut App) -> std::sync::mpsc::Receiver<super::super::central::Job> {
    let jobs = admitting(app);
    assert!(jobs.try_recv().is_ok(), "the admission was asked for");
    land(
        app,
        Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        }),
    );
    assert!(!app.world().resource::<LobbyState>().central_admitting());
    jobs
}

/// A refusal in a live session (a table the gateway would not open, a deck it
/// would not save) is about that table or deck: the session goes on, at its
/// gateway, with the caret where it was. Accepting terms enters it afresh,
/// and "Restart to update" still names it (`resume`: `gateway_selected`).
#[test]
fn a_refusal_inside_a_central_session_leaves_the_gateway_chosen() {
    let mut app = headless();
    let jobs = in_a_central_session(&mut app);
    let before = app.world().resource::<LobbyState>().lobby.focus();
    land(
        &mut app,
        Reply::Event(LobbyEvent::Failed("that table is full".to_string())),
    );
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.token(), Some("tok"), "the session goes on");
        assert!(state.gateway_selected, "at its gateway");
        assert!(state.lobby.gateway_chosen());
        assert_eq!(state.lobby.focus(), before, "and the caret stays");
    }
    let mailbox = app.world().resource::<Mailbox>().clone();
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    super::super::central::readmit(&mut state, &mailbox);
    assert!(jobs.try_recv().is_ok(), "the terms' readmission finds it");
}

/// The one refusal that does send the player back: the admission's own.
#[test]
fn a_refused_admission_goes_back_to_the_account_s_list() {
    let mut app = headless();
    let _jobs = admitting(&mut app);
    land(
        &mut app,
        Reply::Event(LobbyEvent::Failed("not admitted".to_string())),
    );
    let state = app.world().resource::<LobbyState>();
    assert!(!state.central_admitting());
    assert!(!state.gateway_selected);
    assert!(!state.lobby.gateway_chosen());
    assert_eq!(state.lobby.focus(), Field::Gateway);
    assert_eq!(state.lobby.tone(), Tone::Refusal);
    assert_eq!(state.lobby.status(), "not admitted");
}

/// What the chosen gateway's `GET /auth/config` said, and then its refusal.
fn turned_away_by(app: &mut App, central: bool, guests: bool) {
    land(
        app,
        Reply::Registration {
            registration: Registration::Open,
            art_cache: false,
            guests,
            central,
            guests_only: false,
        },
    );
    land(
        app,
        Reply::Event(LobbyEvent::Failed("not admitted".to_string())),
    );
}

/// A gateway that takes guests and no Central account can never admit the
/// account. The refusal is shown with its guest door under it, and nothing
/// moves until the player presses that door; then the gateway's own guest
/// face stands, and Back returns to the list, Central signed in throughout.
#[test]
fn a_guests_gateway_offers_its_door_under_the_refusal_and_waits_for_the_player() {
    let door = Press::Front(FrontPress::GuestFace);
    let mut app = headless();
    let _jobs = admitting(&mut app);
    turned_away_by(&mut app, false, true);
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.status(), "not admitted", "it is said");
        assert_eq!(state.lobby.token(), None, "and nobody came in");
        assert!(!state.lobby.guest());
    }
    assert!(presses(&mut app).contains(&door), "the door stands");

    press(&mut app, door);
    settle(&mut app);
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.face(), baylee_client_core::lobby::Face::Guest);
        assert_eq!(state.lobby.token(), None, "the face asks, nothing is sent");
        assert!(state.central_ready(), "Central stays signed in");
        assert!(state.lobby.gateway_chosen());
    }
    assert!(presses(&mut app).contains(&Press::Front(FrontPress::Submit)));

    press(&mut app, Press::Front(FrontPress::BackToSignIn));
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert!(state.showing_gateway_hub());
    assert!(!state.lobby.gateway_chosen(), "back on the list");
    assert_eq!(state.lobby.focus(), Field::Gateway);
    assert!(presses(&mut app).contains(&door), "the door still stands");
}

/// Coming in through that door is any guest's sign-in: the gateway's terms
/// wait for their sheet and its answer, and the guest is kept for the gateway.
#[test]
fn a_guest_through_that_door_meets_the_terms_sheet() {
    use super::super::front::terms::TermsReply;
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    let _jobs = admitting(&mut app);
    turned_away_by(&mut app, false, true);
    press(&mut app, Press::Front(FrontPress::GuestFace));
    settle(&mut app);
    press(&mut app, Press::Front(FrontPress::Submit));
    assert_ne!(
        app.world().resource::<LobbyState>().lobby.status(),
        "Please use Baylee Central sign-in.",
        "the gateway's own route is asked"
    );
    land(&mut app, Reply::Terms(TermsReply::Stale(true)));
    land(
        &mut app,
        Reply::Event(LobbyEvent::GuestIn(baylee_client_core::lobby::KeptGuest {
            token: "guest-tok".to_string(),
            handle: "Casper#0007".to_string(),
        })),
    );
    let state = app.world().resource::<LobbyState>();
    assert!(state.lobby.guest());
    assert!(state.terms.up(), "the sheet asks; nothing was accepted");
    assert!(state.central_ready());
    assert_eq!(state.guests.len(), 1, "kept for this gateway");
}

/// A gateway that takes Central's accounts may be refusing a ban, and one
/// that takes no guests has no door: neither is offered, the refusal stands.
#[test]
fn no_guest_door_where_the_gateway_takes_central_accounts_or_no_guests() {
    for (central, guests) in [(true, true), (true, false), (false, false)] {
        let mut app = headless();
        let _jobs = admitting(&mut app);
        turned_away_by(&mut app, central, guests);
        assert_eq!(
            app.world().resource::<LobbyState>().lobby.status(),
            "not admitted"
        );
        assert!(
            !presses(&mut app).contains(&Press::Front(FrontPress::GuestFace)),
            "central {central}, guests {guests}"
        );
    }
}

/// The door opens on a refusal only, not on what the gateway said about
/// itself; the next admission asked for closes it again.
#[test]
fn the_guest_door_waits_for_a_refusal_and_goes_with_the_next_admission() {
    let door = Press::Front(FrontPress::GuestFace);
    let mut app = headless();
    let _jobs = admitting(&mut app);
    land(
        &mut app,
        Reply::Registration {
            registration: Registration::Open,
            art_cache: false,
            guests: true,
            central: false,
            guests_only: false,
        },
    );
    assert!(!presses(&mut app).contains(&door), "still being admitted");
    land(
        &mut app,
        Reply::Event(LobbyEvent::Failed("not admitted".to_string())),
    );
    assert!(presses(&mut app).contains(&door));
    let mailbox = app.world().resource::<Mailbox>().clone();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        assert!(state.select_gateway(0));
        super::super::central::admit(&mut state, &mailbox);
    }
    app.update();
    settle(&mut app);
    assert!(!presses(&mut app).contains(&door), "asked again");
}

/// Signed in at Central with two gateways listed, at the picker.
fn at_the_picker(app: &mut App) -> std::sync::mpsc::Receiver<super::super::central::Job> {
    use baylee_client_core::lobby::gateway_info::GatewayInfo;
    let (handle, jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let urls = ["https://one.example", "https://two.example"].map(str::to_string);
        super::super::central::restored_for_test(&mut state, urls.to_vec());
        state.central.as_mut().expect("Central on").handle = Some(handle);
        for url in urls {
            state.probes.insert(
                url,
                Probe::Known(GatewayInfo {
                    version: "0.1.0".to_string(),
                    protocol_version: baylee_protocol::PROTOCOL_VERSION,
                    view_version: baylee_view::VIEW_VERSION,
                    ..GatewayInfo::default()
                }),
            );
        }
        state.lobby.set_field(Field::Gateway, "");
    }
    settle(app);
    jobs
}

fn key(app: &mut App, input: KeyboardInput) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(input);
    app.update();
}

/// Decks and back moves a Central session to the table screen for good, and
/// the picker the Play tab draws there is the same picker: it takes the
/// typed address, and the arrows and Enter walk and choose its rows.
#[test]
fn the_gateway_picker_answers_the_keyboard_after_a_visit_to_decks() {
    let mut app = headless();
    let jobs = at_the_picker(&mut app);
    press(&mut app, Press::Header(HeaderPress::Nav(1)));
    app.update();
    press(&mut app, Press::Header(HeaderPress::Nav(0)));
    app.update();
    assert_eq!(
        *app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table
    );
    assert!(app.world().resource::<LobbyState>().showing_gateway_hub());

    key(&mut app, typed('x'));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Gateway),
        "x"
    );
    key(&mut app, pressed(KeyCode::Backspace, Key::Backspace));
    key(&mut app, pressed(KeyCode::ArrowDown, Key::ArrowDown));
    let row = app.world().resource::<LobbyState>().gateway_cursor;
    assert!(row.is_some(), "the arrows walk the rows");
    key(&mut app, pressed(KeyCode::Enter, Key::Enter));
    let state = app.world().resource::<LobbyState>();
    assert!(state.central_admitting(), "Enter chose the row");
    assert_eq!(state.gateway, state.displayed_gateways()[row.unwrap()]);
    assert!(jobs.try_recv().is_ok());
}

/// The picker's boxes are the Play tab's: on Decks a key goes nowhere near
/// them, and back on Play the caret leaves the box of the tab just left.
#[test]
fn the_picker_and_the_decks_tab_each_keep_their_own_keys() {
    let mut app = headless();
    let _jobs = at_the_picker(&mut app);
    press(&mut app, Press::Header(HeaderPress::Nav(1)));
    app.update();
    key(&mut app, typed('q'));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Gateway),
        "",
        "the picker is not on screen"
    );

    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .focus_on(Field::DeckSearch);
    key(&mut app, typed('d'));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::DeckSearch),
        "d"
    );
    press(&mut app, Press::Header(HeaderPress::Nav(0)));
    app.update();
    key(&mut app, typed('x'));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.field(Field::DeckSearch), "d");
    assert_eq!(state.lobby.field(Field::Gateway), "x");
}

fn casper() -> baylee_client_core::lobby::KeptGuest {
    baylee_client_core::lobby::KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    }
}

/// A guest at a guests' gateway, Central signed in behind it, with a
/// settings file to be kept in.
fn a_central_player_as_a_guest() -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    let _jobs = admitting(&mut app);
    turned_away_by(&mut app, false, true);
    land(&mut app, Reply::Event(LobbyEvent::GuestIn(casper())));
    assert!(app.world().resource::<LobbyState>().lobby.guest());
    app
}

/// A guest made at that door ends at its gateway like any guest: Central has
/// no part in it, so declining the terms is not refused for want of Central.
/// Nothing of it stays on this device, and the account's list stands again.
#[test]
fn a_guest_from_that_door_can_be_deleted_and_the_list_stands_again() {
    let mut app = a_central_player_as_a_guest();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .ask_to_delete_account();
    super::front_terms::click(
        &mut app,
        Press::Settings(SettingsPress::ConfirmAccountDeletion),
    );
    {
        let refusal = app
            .world()
            .resource::<LobbyState>()
            .lobby
            .deleting_account()
            .and_then(|deletion| deletion.refusal.clone())
            .unwrap_or_default();
        assert!(!refusal.contains("Central"), "refused here: {refusal}");
    }

    land(&mut app, Reply::Event(LobbyEvent::AccountDeleted));
    settle(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), None);
    assert!(state.guests.is_empty(), "its dead token is not kept");
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .guests
            .is_empty(),
        "nor in the settings file"
    );
    assert!(state.central_ready(), "Central stays signed in");
    assert!(state.showing_gateway_hub());
    assert!(!state.gateway_selected);
}

/// A guest this device keeps for that gateway is offered by its handle, and
/// is not taken up until the player presses it. No request goes first: the
/// session is the whole of a guest (`Lobby::play_as_guest`), and terms are
/// the sheet's, by `/info` against this device's copy.
#[test]
fn a_kept_guest_comes_through_that_door_only_when_pressed() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .guests
        .insert(String::new(), casper());
    let _jobs = admitting(&mut app);
    turned_away_by(&mut app, false, true);
    assert!(labels(&mut app).contains(&"Continue as Casper#0007".to_string()));
    assert_eq!(app.world().resource::<LobbyState>().lobby.token(), None);

    press(&mut app, Press::Front(FrontPress::GuestFace));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), Some("guest-tok"));
    assert!(state.central_ready(), "Central stays signed in");
}

/// Switch gateway, from the header's gateway card.
fn switch_gateway(app: &mut App) {
    press(app, Press::Header(HeaderPress::Gateway));
    app.update();
    press(app, Press::Header(HeaderPress::SwitchGateway));
    app.update();
    settle(app);
}

/// Switching gateway ends a guest as signing it out does (#269), so it is
/// asked first. Said no to, the guest plays on; said yes to, it is forgotten
/// here too, and Central stays signed in with the account's list.
#[test]
fn a_guest_is_asked_before_switching_gateway_ends_it() {
    let mut app = a_central_player_as_a_guest();
    switch_gateway(&mut app);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(
            matches!(
                state.confirmation,
                Some(super::super::confirm::Destructive::SignOutGuest { to_gateways: true })
            ),
            "the guest is asked"
        );
        assert_eq!(state.lobby.token(), Some("guest-tok"), "and is still here");
    }
    {
        let found = labels(&mut app);
        for said in ["Switch gateway?", "Casper#0007"] {
            assert!(found.contains(&said.to_string()), "{found:?}");
        }
        assert!(
            !found.iter().any(|label| label.starts_with("Sign out")),
            "{found:?}"
        );
    }
    press(&mut app, Press::Shared(SharedPress::CancelDestructive));
    app.update();
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.confirmation.is_none());
        assert_eq!(state.lobby.token(), Some("guest-tok"));
    }

    switch_gateway(&mut app);
    tap_control(&mut app, "yes", |p| {
        *p == Press::Shared(SharedPress::ConfirmDestructive)
    });
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), None);
    assert!(state.guests.is_empty(), "its dead token is not kept");
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .guests
            .is_empty(),
        "nor in the settings file"
    );
    assert!(state.central_ready(), "Central stays signed in");
    assert!(state.showing_gateway_hub());
    assert!(!state.gateway_selected);
}

/// What one account's session bought does not survive into the next
/// session: its online replays and an Undo still counting down.
#[test]
fn switching_gateway_leaves_nothing_of_the_session_behind() {
    use baylee_client_core::replay::library::{HostedCheck, HostedGame, HostedResult, ReplayPage};
    let mut app = headless();
    let _jobs = in_a_central_session(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.undo = Some(super::super::decks::Undo {
            kind: super::super::decks::UndoKind::Restore,
            left: 5.0,
        });
        let page = ReplayPage {
            games: vec![HostedGame {
                game: "g1".to_string(),
                started_at: "2026-11-05T12:00:00Z".to_string(),
                ended_at: None,
                complete: true,
                seat: 0,
                turn: None,
                result: HostedResult::Won,
                public: false,
                check: HostedCheck::Exact,
                build: None,
                seats: Vec::new(),
                archived: false,
            }],
            total: 1,
        };
        state.replays.library.add_page("", 0, &page);
        assert_eq!(state.replays.library.entries().len(), 1);
    }
    app.update();
    settle(&mut app);
    switch_gateway(&mut app);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), None);
    assert!(state.central_ready(), "Central stays signed in");
    assert!(state.undo.is_none(), "no Undo of the old session");
    assert!(
        state.replays.library.entries().is_empty(),
        "no replay of the old session"
    );
}

/// Settings' Switch gateway is the header's: it leaves the gateway, not
/// Central, and asks a guest first.
#[test]
fn settings_switch_gateway_is_the_headers() {
    use crate::lobby::settings_press::SettingsPress;
    use baylee_client_core::settings_map::Section;
    let open_network = |app: &mut App| {
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.settings = SettingsPane::Open;
            state.set_settings_section(Section::Network);
        }
        app.update();
        settle(app);
    };

    let mut app = headless();
    let _jobs = in_a_central_session(&mut app);
    open_network(&mut app);
    press(&mut app, Press::Settings(SettingsPress::SwitchGateway));
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.token(), None);
        assert!(state.central_ready(), "Central stays signed in");
        assert!(state.showing_gateway_hub());
    }

    let mut app = a_central_player_as_a_guest();
    open_network(&mut app);
    press(&mut app, Press::Settings(SettingsPress::SwitchGateway));
    let state = app.world().resource::<LobbyState>();
    assert!(state.confirmation.is_some(), "the guest is asked");
    assert_eq!(state.lobby.token(), Some("guest-tok"));
}

/// A session the gateway ended leaves the list on screen (`Panel::of`), and
/// the keys go to that list, never to the sign-in fields it hides.
#[test]
fn a_central_session_that_expires_leaves_the_keys_on_the_list() {
    let mut app = headless();
    let _jobs = in_a_central_session(&mut app);
    land(&mut app, Reply::Expired);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), None);
    assert!(state.central_ready(), "Central's own session goes on");
    assert!(state.showing_gateway_hub());
    assert!(!state.lobby.gateway_chosen());
    assert_eq!(state.lobby.focus(), Field::Gateway);
}
