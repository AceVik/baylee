//! The front door, and the text entry it is made of: the controls the form draws, the keyboard that fills them, and the address a launch remembers from the last sign-in that actually worked. The caret is here because this form is where the client draws its own text boxes — it is a node standing between two runs rather than a measured offset, so the row it sits in is the screen — and with it the selection fill, ⇧Tab and the arrows, the password bullets, the eye that lifts them and the blink `reduce_motion` holds still. The builder's own fields are exercised beside the builder; nothing here knows about decks, tables or a seat.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::super::front::{FrontCard, FrontShade, Panel};

#[test]
fn the_sign_in_screen_builds_with_its_controls() {
    let mut app = headless();
    assert_eq!(roots(&mut app).len(), 1, "exactly one tree");
    let found = presses(&mut app);
    for wanted in [
        Press::Focus(Field::Username),
        Press::Focus(Field::Password),
        Press::Submit,
        Press::ToggleRegistering,
        Press::LeaveGateway,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    for registering in [Field::DisplayName, Field::PasswordAgain] {
        assert!(
            !found.contains(&Press::Focus(registering)),
            "{registering:?} is only asked for when registering"
        );
    }
    for elsewhere in [Press::PlayOffline, Press::AddGateway] {
        assert!(
            !found.contains(&elsewhere),
            "{elsewhere:?} is on the other face of the card"
        );
    }
    let drawn = labels(&mut app);
    let said = |phrase: Phrase| {
        drawn
            .iter()
            .filter(|l| l.as_str() == phrase.text(Lang::En))
            .count()
    };
    assert_eq!(said(Phrase::Continue), 1, "the submit says where it goes");
    assert_eq!(said(Phrase::SignIn), 1, "and the tab alone names the mode");
    assert!(
        drawn
            .iter()
            .any(|l| l == super::super::front::FAN_CONTENT_NOTICE),
        "the policy's notice stands under the form"
    );
}

#[test]
fn creating_an_account_asks_for_the_password_twice_and_names_itself_once() {
    let mut app = headless();
    press(&mut app, Press::ToggleRegistering);
    settle(&mut app);
    let found = presses(&mut app);
    for wanted in [
        Field::Username,
        Field::DisplayName,
        Field::Password,
        Field::PasswordAgain,
    ] {
        assert!(found.contains(&Press::Focus(wanted)), "{wanted:?} missing");
    }
    let drawn = labels(&mut app);
    let said = |text: &str| drawn.iter().filter(|l| l.as_str() == text).count();
    assert_eq!(said(Phrase::CreateAccount.text(Lang::En)), 1);
    assert_eq!(said(Phrase::Continue.text(Lang::En)), 1);
    assert_eq!(said(Phrase::PasswordAgain.text(Lang::En)), 1);
}

#[test]
fn the_open_tab_is_the_lit_one_and_the_other_opens_its_form() {
    fn lit(app: &mut App) -> Vec<(String, Press, Color)> {
        let mut tabs = app.world_mut().query_filtered::<(
            &Press,
            &Children,
            &BackgroundColor,
        ), With<super::super::front::ActiveTab>>();
        tabs.iter(app.world())
            .map(|(press, children, fill)| {
                let label = children
                    .iter()
                    .find_map(|kid| app.world().get::<Text>(kid))
                    .map(|text| text.0.clone())
                    .unwrap_or_default();
                (label, *press, fill.0)
            })
            .collect()
    }
    let mut app = headless();
    assert_eq!(
        lit(&mut app),
        [(
            Phrase::SignIn.text(Lang::En).to_string(),
            Press::PickerNothing,
            palette::PANEL_HOT
        )]
    );
    press(&mut app, Press::ToggleRegistering);
    settle(&mut app);
    assert_eq!(
        lit(&mut app),
        [(
            Phrase::CreateAccount.text(Lang::En).to_string(),
            Press::PickerNothing,
            palette::PANEL_HOT
        )],
        "the tab pressed is the one lit, and pressing it again does nothing"
    );
}

#[test]
fn the_gateway_form_builds_with_its_controls_and_none_of_the_account_s() {
    let mut app = headless();
    to_gateway_face(&mut app);
    assert_eq!(roots(&mut app).len(), 1, "exactly one tree");
    let found = presses(&mut app);
    for wanted in [
        Press::Focus(Field::Gateway),
        Press::AddGateway,
        Press::PlayOffline,
        Press::FrontMenu,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    for elsewhere in [
        Press::Submit,
        Press::Focus(Field::Username),
        Press::LeaveGateway,
        // Behind the gear, until it is pressed.
        Press::OpenSettings,
    ] {
        assert!(
            !found.contains(&elsewhere),
            "{elsewhere:?} is on the other face of the card"
        );
    }
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l == baylee_build::short()),
        "the build is on this face too"
    );
    assert!(
        drawn
            .iter()
            .any(|l| l == super::super::front::FAN_CONTENT_NOTICE)
    );
}

#[test]
fn the_gear_opens_the_languages_and_the_way_to_every_setting() {
    fn open(app: &mut App) -> bool {
        presses(app).contains(&Press::OpenSettings)
    }
    let mut app = headless();
    assert!(!open(&mut app));
    press(&mut app, Press::FrontMenu);
    assert!(open(&mut app), "the gear opens its menu");
    let found = presses(&mut app);
    let drawn = labels(&mut app);
    for offered in Lang::ALL {
        assert!(found.contains(&Press::PickLang(offered)));
        assert!(
            drawn.iter().any(|l| l == offered.name()),
            "each language by its own name: {offered:?}"
        );
    }

    // Picking a language keeps the menu open, to see what was picked.
    press(&mut app, Press::PickLang(Lang::De));
    assert_eq!(app.world().resource::<LobbyState>().lobby.lang(), Lang::De);
    assert!(open(&mut app));

    // A press anywhere outside it closes it: the veil takes that press.
    let veil = {
        let mut veils = app
            .world_mut()
            .query_filtered::<(Entity, &Press), With<GlobalZIndex>>();
        veils
            .iter(app.world())
            .find(|(_, press)| **press == Press::FrontMenu)
            .map(|(entity, _)| entity)
            .expect("a veil behind the menu")
    };
    tap(&mut app, veil);
    app.update();
    assert!(!open(&mut app));

    // Escape closes the menu and only the menu.
    press(&mut app, Press::FrontMenu);
    assert!(open(&mut app));
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Escape, Key::Escape));
    app.update();
    assert!(!open(&mut app));
    assert!(
        app.world().resource::<LobbyState>().lobby.gateway_chosen(),
        "still at the gateway it was at"
    );

    to_gateway_face(&mut app);
    assert!(
        presses(&mut app).contains(&Press::FrontMenu),
        "on both faces"
    );
}

#[test]
fn typing_reaches_the_form() {
    let mut app = headless();
    for ch in ['h', 'i'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "hi"
    );
}

/// The point of the whole exercise: a correction made in the middle of what
/// was typed, which an append-only field could not do at all.
#[test]
fn the_caret_moves_and_typing_lands_where_it_is() {
    let mut app = headless();
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['a', 'b'] {
            messages.write(typed(ch));
        }
        messages.write(pressed(KeyCode::Home, Key::Home));
        messages.write(typed('x'));
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "xab"
    );
}

/// Shift and an arrow select, and the next character replaces the run.
#[test]
fn shift_and_an_arrow_select_what_the_next_key_replaces() {
    let mut app = headless();
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['a', 'b', 'c'] {
            messages.write(typed(ch));
        }
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        messages.write(pressed(KeyCode::ArrowLeft, Key::ArrowLeft));
        messages.write(pressed(KeyCode::ArrowLeft, Key::ArrowLeft));
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .buffer(Field::Username)
            .selection(),
        Some(1..3)
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(typed('z'));
    app.update();
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "az"
    );
}

/// ⇧Tab walks the form backwards.
#[test]
fn shift_tab_moves_the_caret_back_a_field() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Tab, Key::Tab));
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.focus(),
        Field::Password,
        "back from the first field is the last one"
    );
}

/// The caret is a node in the row of runs, so it is drawn between exactly the
/// letters it stands between — no glyph measuring anywhere.
#[test]
fn the_caret_is_drawn_between_the_letters_it_stands_between() {
    let mut app = headless();
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['a', 'b'] {
            messages.write(typed(ch));
        }
        messages.write(pressed(KeyCode::ArrowLeft, Key::ArrowLeft));
    }
    app.update();
    assert_eq!(drawn_field(&mut app, Field::Username), ["a", "|", "b"]);
    let mut carets = app.world_mut().query::<&Caret>();
    assert_eq!(
        carets.iter(app.world()).count(),
        1,
        "one field has the caret, so one bar is drawn"
    );
}

/// A selected run is a run with a fill behind it, and the caret is drawn at
/// the end the player is holding.
#[test]
fn a_selection_is_a_run_with_a_fill_behind_it() {
    let mut app = headless();
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['a', 'b'] {
            messages.write(typed(ch));
        }
        messages.write(pressed(KeyCode::Home, Key::Home));
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::ArrowRight, Key::ArrowRight));
    app.update();
    assert_eq!(
        drawn_field(&mut app, Field::Username),
        ["a", "|", "b"],
        "shift and right selects the a and leaves the caret past it"
    );
    assert_eq!(
        fills(&mut app),
        1,
        "exactly the selected run carries the fill"
    );
}

/// A password is drawn as bullets, and the field still holds the letters —
/// which is what lets the caret be an offset into the text and not into what
/// is on screen.
#[test]
fn a_password_is_drawn_as_bullets_and_an_address_is_not() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .focus_on(Field::Password);
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['p', 'w'] {
            messages.write(typed(ch));
        }
    }
    app.update();
    assert_eq!(
        drawn_field(&mut app, Field::Password),
        ["\u{2022}\u{2022}", "|"]
    );
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Password),
        "pw",
        "the bullets are drawn, never stored"
    );
}

/// The blink is a phase of how long the caret has stood still, so nothing has
/// to be kept in step with it — and `reduce_motion` stops it dead.
#[test]
fn the_caret_blinks_unless_it_was_asked_to_hold_still() {
    assert!(caret_lit(0.0, false), "solid the moment it moves");
    assert!(caret_lit(0.4, false));
    assert!(!caret_lit(0.6, false), "dark through the second half");
    assert!(caret_lit(1.1, false), "and lit again on the next round");
    assert!(caret_lit(0.6, true), "reduce_motion is a promise it holds");
}

/// A field nobody is typing into shows no selection either.
///
/// The fill and the ring are the same accent saying the same thing, so a
/// selection left standing in a field the caret has walked out of put that
/// claim in two boxes at once. Found by looking at the screen.
#[test]
fn a_field_without_the_caret_shows_no_selection() {
    let mut app = headless();
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['a', 'b'] {
            messages.write(typed(ch));
        }
        messages.write(pressed(KeyCode::Home, Key::Home));
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::ArrowRight, Key::ArrowRight));
    app.update();
    assert_eq!(fills(&mut app), 1, "the premise: a selection was made");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Tab, Key::Tab));
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.focus(),
        Field::Password,
        "the caret left the field"
    );
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .buffer(Field::Username)
            .selection()
            .is_some(),
        "and the selection is still in the buffer, undrawn"
    );
    assert_eq!(fills(&mut app), 0, "nothing is drawn selected any more");
}

/// The eye beside a password shows what is being typed, and covers it again.
#[test]
fn the_eye_shows_the_password_and_the_bullets_come_back() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .focus_on(Field::Password);
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in ['p', 'w'] {
            messages.write(typed(ch));
        }
    }
    app.update();
    assert!(
        presses(&mut app).contains(&Press::Reveal(Field::Password)),
        "a password box carries an eye"
    );
    assert!(
        !presses(&mut app).contains(&Press::Reveal(Field::Username)),
        "and a box that is not masked does not"
    );
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .toggle_reveal(Field::Password);
    app.update();
    assert_eq!(drawn_field(&mut app, Field::Password), ["pw", "|"]);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .toggle_reveal(Field::Password);
    app.update();
    assert_eq!(
        drawn_field(&mut app, Field::Password),
        ["\u{2022}\u{2022}", "|"]
    );
}

/// The address is filled in from the settings and the caret starts on the
/// password, because that is the only thing still missing.
///
/// Through `from_settings` rather than `new`, which reads the defaults under
/// this crate's tests on purpose — the point of the split.
#[test]
fn the_sign_in_box_opens_on_the_name_that_used_it_last() {
    let blank = LobbyState::from_settings(crate::settings::ClientSettings::default());
    assert_eq!(blank.lobby.field(Field::Username), "");
    assert_eq!(
        blank.lobby.focus(),
        Field::Gateway,
        "the gateway form is the one on screen at launch"
    );

    let known = LobbyState::from_settings(crate::settings::ClientSettings {
        last_username: "Alice.B".to_string(),
        ..crate::settings::ClientSettings::default()
    });
    assert_eq!(known.lobby.field(Field::Username), "Alice.B");
    assert_eq!(
        known.lobby.focus(),
        Field::Gateway,
        "the gateway form is the one on screen at launch"
    );
}

/// The remembered name outlives the choice of gateway, which is the only
/// way the sign-in form is ever reached.
#[test]
fn choosing_a_gateway_keeps_the_name_that_used_it_last() {
    let mut known = LobbyState::from_settings(crate::settings::ClientSettings {
        last_username: "Alice.B".to_string(),
        gateways: vec!["https://one.example".into()],
        ..crate::settings::ClientSettings::default()
    });
    assert!(known.select_gateway(0));
    assert_eq!(known.lobby.field(Field::Username), "Alice.B");
    assert_eq!(known.lobby.focus(), Field::Password);

    known.leave_gateway();
    assert!(!known.gateway_selected);
    assert_eq!(known.lobby.focus(), Field::Gateway);
    assert!(known.select_gateway(0));
    assert_eq!(
        known.lobby.field(Field::Username),
        "Alice.B",
        "and a round trip through the gateway form"
    );
}

/// And it is written down by the sign-in that worked, not by the attempt:
/// the username the gateway answered, even when an address was typed, as a
/// player from before usernames does until the end of 2026 (#269).
#[test]
fn only_a_sign_in_that_worked_is_worth_remembering() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_field(Field::Username, "mail@acevik.de");
    }

    // A refusal leaves the stored address alone: it may well be the typo.
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::Failed(
            "invalid credentials".to_string(),
        )));
    app.update();
    let settings = app.world().resource::<crate::settings::ClientSettings>();
    assert_eq!(
        settings.last_username, "",
        "a refused attempt says nothing about the address"
    );
    assert_eq!(
        settings.gateway_uses,
        baylee_client_core::lobby::gateway_use::GatewayUses::default(),
        "and is no use of the gateway"
    );

    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: Some("Alice.B".to_string()),
        }));
    app.update();
    let gateway = app.world().resource::<LobbyState>().gateway.clone();
    let settings = app.world().resource::<crate::settings::ClientSettings>();
    assert_eq!(
        settings.last_username, "Alice.B",
        "the name, not the address it was reached by"
    );
    assert_eq!(
        settings.gateway_uses.of(&gateway).count,
        1,
        "the sign-in that worked is a use of its gateway, kept with the address"
    );
}

#[test]
fn issue_187_no_account_request_without_an_explicit_gateway() {
    let mut app = headless();
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings {
        gateways: vec!["https://one.example".into(), "https://two.example/".into()],
        ..default()
    });
    state
        .lobby
        .set_field(Field::Username, "review@example.invalid");
    state
        .lobby
        .set_field(Field::Password, "temporary-test-password");
    assert!(!state.gateway_selected);
    assert!(state.lobby.submit().is_none());
    app.world_mut().insert_resource(state);
    app.update();
    assert!(!presses(&mut app).contains(&Press::Submit));
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        assert!(state.select_gateway(1));
        assert_eq!(state.gateway, "https://two.example");
        assert!(state.lobby.field(Field::Password).is_empty());
        state
            .lobby
            .set_field(Field::Username, "review@example.invalid");
        state
            .lobby
            .set_field(Field::Password, "temporary-test-password");
        assert!(state.lobby.submit().is_some());
        state.lobby.apply(LobbyEvent::Failed("test".into()));
    }
    app.update();
    assert!(presses(&mut app).contains(&Press::Submit));
    // An old registration reply cannot enable this gateway's registration.
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .unwrap()
        .push(Reply::Remote(
            0,
            Box::new(Reply::Registration {
                enabled: true,
                art_cache: false,
                guests: false,
            }),
        ));
    app.update();
    assert!(
        !app.world()
            .resource::<LobbyState>()
            .lobby
            .registration_enabled()
    );
}

#[test]
fn issue_187_saved_gateways_round_trip_without_selecting_one() {
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
    for url in ["https://example.test/", "https://example.test"] {
        state.lobby.set_field(Field::Gateway, url);
        let asked = state.check_gateway().expect("an address");
        state.gateway_answered(asked, Probe::Older);
    }
    assert_eq!(state.gateways, ["https://example.test"]);
    assert!(!state.gateway_selected, "saving is not selecting");
    let settings = crate::settings::ClientSettings {
        gateways: state.gateways.clone(),
        ..default()
    };
    let json = serde_json::to_string(&settings).unwrap();
    let restored = LobbyState::from_settings(serde_json::from_str(&json).unwrap());
    assert_eq!(restored.gateways, state.gateways);
    assert!(!restored.gateway_selected);
}

#[test]
fn issue_187_sign_out_ignores_late_account_responses() {
    let mut app = headless();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::LoggedIn {
            token: "first-account".into(),
            username: None,
        });
    app.update();
    let epoch = app.world().resource::<LobbyState>().gateway_epoch;
    press(&mut app, Press::SignOut);
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .unwrap()
        .push(Reply::Remote(
            epoch,
            Box::new(Reply::Event(LobbyEvent::LoggedIn {
                token: "stale-account".into(),
                username: None,
            })),
        ));
    app.update();
    assert!(app.world().resource::<LobbyState>().lobby.token().is_none());
}

#[test]
fn deeply_nested_button_contents_still_activate_their_button() {
    let mut app = headless();
    let button = app.world_mut().spawn(Press::PlayOffline).id();
    let mut leaf = button;
    for _ in 0..12 {
        let child = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(leaf).add_child(child);
        leaf = child;
    }
    tap(&mut app, leaf);
    app.update();
    assert!(app.world().resource::<LobbyState>().offline.is_some());
}

/// A gateway answering `/info` as this client's own build would.
fn gateway_info(name: Option<&str>, version: &str, view: u32) -> Probe {
    Probe::Known(baylee_client_core::lobby::gateway_info::GatewayInfo {
        name: name.map(str::to_string),
        version: version.to_string(),
        protocol_version: baylee_protocol::PROTOCOL_VERSION,
        view_version: view,
    })
}

#[test]
fn an_address_is_saved_only_once_a_gateway_answers_there() {
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
    state
        .lobby
        .set_field(Field::Gateway, "https://typo.example/");
    let asked = state.check_gateway().expect("an address");
    assert_eq!(asked, "https://typo.example");
    assert!(
        state.gateways.is_empty(),
        "nothing is saved before the answer"
    );

    assert!(!state.gateway_answered("https://other.example".into(), Probe::Older));
    assert_eq!(
        state.adding.as_deref(),
        Some("https://typo.example"),
        "an answer about another address settles nothing"
    );

    assert!(!state.gateway_answered(asked, Probe::Silent));
    assert!(state.gateways.is_empty());
    assert_eq!(state.adding, None);
    assert_eq!(state.lobby.tone(), Tone::Refusal);
    assert!(state.lobby.status().contains("https://typo.example"));
    assert_eq!(
        state.lobby.field(Field::Gateway),
        "https://typo.example/",
        "what was typed stays, to be corrected"
    );

    // An incompatible gateway is still a gateway: saved, and drawn with its
    // warning.
    let asked = state.check_gateway().expect("an address");
    let newer = gateway_info(None, "9.9.9", baylee_view::VIEW_VERSION + 1);
    assert!(state.gateway_answered(asked, newer));
    assert_eq!(state.gateways, ["https://typo.example"]);
    assert!(state.lobby.field(Field::Gateway).is_empty());
    assert_eq!(state.lobby.tone(), Tone::Note);
}

#[test]
fn the_save_button_waits_for_the_answer_the_mailbox_brings() {
    let mut app = headless();
    to_gateway_face(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_field(Field::Gateway, "https://new.example");
        state.check_gateway().expect("an address");
    }
    app.update();
    assert!(
        !presses(&mut app).contains(&Press::AddGateway),
        "no second question while one is out"
    );
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .unwrap()
        .push(Reply::Gateway {
            url: "https://new.example".into(),
            probe: gateway_info(Some("New Hall"), "0.1.0", baylee_view::VIEW_VERSION),
        });
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().gateways,
        ["https://new.example"]
    );
    let pressable = presses(&mut app);
    assert!(pressable.contains(&Press::AddGateway));
    assert!(pressable.contains(&Press::SelectGateway(0)));
    let drawn = labels(&mut app);
    assert!(drawn.iter().any(|l| l == "New Hall"), "the name leads");
    assert!(
        drawn.iter().any(|l| l == "https://new.example"),
        "and the address stays in sight under it"
    );
}

#[test]
fn enter_in_the_gateway_address_saves_it_and_signs_nothing_in() {
    let mut app = headless();
    to_gateway_face(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_field(Field::Gateway, "https://new.example");
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.focus(),
        Field::Gateway,
        "the premise: the caret is in the address"
    );
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Enter, Key::Enter));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        state.adding.as_deref(),
        Some("https://new.example"),
        "the address is asked about, as the Save button would"
    );
    assert_ne!(
        state.lobby.tone(),
        Tone::Refusal,
        "and no sign-in was tried: {:?}",
        state.lobby.status()
    );
    assert_eq!(
        app.world_mut().resource_mut::<LobbyState>().check_gateway(),
        None,
        "and a second Enter asks nothing while the first answer is owed"
    );
}

#[test]
fn a_gateway_whose_games_would_not_open_is_marked_and_says_why() {
    let mut app = headless();
    to_gateway_face(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateways = vec![
            "https://newer.example".into(),
            "https://same.example".into(),
            "https://older.example".into(),
            "https://silent.example".into(),
        ];
        let newer = gateway_info(None, "9.9.9+build.7", baylee_view::VIEW_VERSION + 1);
        let same = gateway_info(None, "0.1.0", baylee_view::VIEW_VERSION);
        state.probes.insert("https://newer.example".into(), newer);
        state.probes.insert("https://same.example".into(), same);
        state
            .probes
            .insert("https://older.example".into(), Probe::Older);
        state
            .probes
            .insert("https://silent.example".into(), Probe::Silent);
    }
    app.update();
    let mut query = app.world_mut().query::<(&Text, &TextColor)>();
    let inks: Vec<(String, Color)> = query
        .iter(app.world())
        .map(|(text, colour)| (text.0.clone(), colour.0))
        .collect();
    let ink = |label: &str| inks.iter().find(|(text, _)| text == label).map(|i| i.1);
    // The row says the release alone; the build is in its hint.
    assert_eq!(ink("v9.9.9"), Some(palette::DANGER));
    assert_eq!(ink("v0.1.0"), Some(palette::HEAL));
    assert_eq!(ink("v?"), Some(palette::ACTIVE));

    let mut hints = app
        .world_mut()
        .query::<(Entity, &super::super::hint::HoverHint)>();
    let said: Vec<(Entity, String)> = hints
        .iter(app.world())
        .map(|(entity, hint)| (entity, hint.0.clone()))
        .collect();
    assert!(
        said.iter().any(|(_, s)| s == "9.9.9+build.7"),
        "the version in full is one point away: {said:?}"
    );
    let dots = |phrase: Phrase| {
        said.iter()
            .filter(|(_, s)| s == phrase.text(Lang::En))
            .count()
    };
    assert_eq!(dots(Phrase::GatewayAnswering), 3, "newer, same and older");
    assert!(
        dots(Phrase::GatewayNotAnswering) >= 1,
        "the silent one's dot"
    );

    // The marks are the cells the warning glyph stands in.
    let mut glyphs = app.world_mut().query::<(&Text, &ChildOf)>();
    let cells: Vec<Entity> = glyphs
        .iter(app.world())
        .filter(|(text, _)| text.0 == super::super::gateway::WARNING_GLYPH.to_string())
        .map(|(_, parent)| parent.parent())
        .collect();
    let marks: Vec<(Entity, String)> = said
        .iter()
        .filter(|(entity, _)| cells.contains(entity))
        .cloned()
        .collect();
    assert_eq!(
        marks.len(),
        2,
        "the newer and the older are marked; the compatible one is not, and the silent one \
         says so with its dot alone: {marks:?}"
    );
    let newer = (baylee_view::VIEW_VERSION + 1).to_string();
    let (mark, _) = marks
        .iter()
        .find(|(_, said)| said.contains(&newer))
        .expect("the mismatch names the gateway's view version");

    // Pointing at the mark draws its sentence; leaving takes it away. The
    // sentence is the label the pointing added, not any label holding the
    // number: the build line holds the commit id, and an id that happens to
    // hold the same two digits failed "no label says it" with the hint gone.
    let before = labels(&mut app);
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut()
        .write_message(aimed(*mark, Over { hit: hit.clone() }));
    app.update();
    let sentence = labels(&mut app)
        .into_iter()
        .find(|l| l.contains(&newer) && !before.contains(l))
        .expect("pointing at the mark drew a sentence naming the version");
    app.world_mut().write_message(aimed(*mark, Out { hit }));
    app.update();
    assert!(!labels(&mut app).contains(&sentence), "{sentence:?} stayed");
}

/// Each front door panel standing, with its scale across and whether it is
/// drawn in front, in the order the panels are named.
fn card_poses(app: &mut App) -> Vec<(Panel, f32, i32)> {
    let mut cards = app
        .world_mut()
        .query::<(&FrontCard, &bevy::ui::UiTransform, &ZIndex)>();
    let mut found: Vec<(Panel, f32, i32)> = cards
        .iter(app.world())
        .map(|(card, pose, z)| (card.0, pose.scale.x, z.0))
        .collect();
    found.sort_by_key(|(panel, ..)| *panel as u8);
    found
}

/// How opaque the first text reading `label` is drawn on `panel`.
fn ink_on(app: &mut App, panel: Panel, label: &str) -> Option<f32> {
    let card = {
        let mut cards = app.world_mut().query::<(Entity, &FrontCard)>();
        cards
            .iter(app.world())
            .find(|(_, card)| card.0 == panel)
            .map(|(entity, _)| entity)?
    };
    let world = app.world();
    let mut waiting = vec![card];
    while let Some(node) = waiting.pop() {
        if let (Some(text), Some(ink)) = (world.get::<Text>(node), world.get::<TextColor>(node))
            && text.0 == label
        {
            return Some(ink.0.alpha());
        }
        if let Some(children) = world.get::<Children>(node) {
            waiting.extend(children.iter());
        }
    }
    None
}

/// A fifth of a motion per frame.
fn frames_of_a_fifth(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
}

/// Runs frames until the front door stands still, and says how many it took.
fn frames_to_land(app: &mut App) -> usize {
    for frame in 1..=10 {
        app.update();
        if card_poses(app).len() == 1 {
            return frame;
        }
    }
    panic!("still moving after ten frames: {:?}", card_poses(app));
}

#[test]
fn choosing_a_gateway_goes_into_it_and_back_comes_out_the_same_way() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateways = vec!["https://hall.example".into()];
        let hall = gateway_info(Some("Hall"), "0.1.0", baylee_view::VIEW_VERSION);
        state.probes.insert("https://hall.example".into(), hall);
    }
    to_gateway_face(&mut app);
    assert_eq!(
        card_poses(&mut app),
        [(Panel::Gateway, 1.0, 1)],
        "a panel at rest carries no pose"
    );

    frames_of_a_fifth(&mut app);
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    app.update();
    let poses = card_poses(&mut app);
    let [
        (Panel::Gateway, list, list_z),
        (Panel::SignIn, form, form_z),
    ] = poses[..]
    else {
        panic!("both panels stand while it moves: {poses:?}");
    };
    assert!(list > 1.0, "the list grows towards the viewer: {list}");
    assert!(
        (0.9..1.0).contains(&form),
        "the form comes up out of the distance: {form}"
    );
    assert!(list_z > form_z, "the list passes in front of the form");
    let continuing = Phrase::Continue.text(Lang::En);
    assert_eq!(
        ink_on(&mut app, Panel::SignIn, continuing),
        Some(0.0),
        "the form is not in sight yet"
    );
    let choosing = Phrase::ChooseGateway.text(Lang::En);
    let fading = ink_on(&mut app, Panel::Gateway, choosing).expect("the list's title");
    assert!(
        fading > 0.0 && fading < 1.0,
        "the list is on its way out: {fading}"
    );

    // Pressed while it moves, neither panel answers.
    press(&mut app, Press::FrontMenu);
    assert!(!app.world().resource::<LobbyState>().front_menu);

    let landed = frames_to_land(&mut app);
    assert!(landed <= 4, "landed in {landed} more frames");
    assert_eq!(card_poses(&mut app), [(Panel::SignIn, 1.0, 1)]);
    let found = presses(&mut app);
    assert!(found.contains(&Press::Submit) && found.contains(&Press::LeaveGateway));
    assert!(!found.contains(&Press::AddGateway));
    assert_eq!(
        ink_on(&mut app, Panel::SignIn, continuing),
        Some(1.0),
        "and whole once it stands"
    );
    assert!(
        labels(&mut app).iter().any(|l| l == "Hall"),
        "the account form is titled with its gateway"
    );

    // Back is the same film the other way: the list comes back from past
    // the viewer, in front, and the form sinks away behind it.
    app.world_mut().resource_mut::<LobbyState>().leave_gateway();
    app.update();
    let poses = card_poses(&mut app);
    let [
        (Panel::Gateway, list, list_z),
        (Panel::SignIn, form, form_z),
    ] = poses[..]
    else {
        panic!("both panels stand on the way back: {poses:?}");
    };
    assert!(list > 1.4, "the list starts where going in left it: {list}");
    assert!(form < 1.0 && list_z > form_z);
    app.update();
    let (_, before, _) = card_poses(&mut app)[0];

    // Asked in again half-way back, it turns round from where it is rather
    // than starting over.
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    app.update();
    let (_, after, _) = card_poses(&mut app)[0];
    assert!(
        after > before,
        "the list goes on growing from {before} and not from 1: {after}"
    );
    let landed = frames_to_land(&mut app);
    assert!(
        landed <= 2,
        "and has only what it came back to go: {landed}"
    );
    assert_eq!(card_poses(&mut app), [(Panel::SignIn, 1.0, 1)]);

    app.world_mut().resource_mut::<LobbyState>().leave_gateway();
    frames_to_land(&mut app);
    assert_eq!(card_poses(&mut app), [(Panel::Gateway, 1.0, 1)]);
    assert!(presses(&mut app).contains(&Press::AddGateway));
}

#[test]
fn the_tabs_turn_the_form_round_with_both_sides_in_sight() {
    let mut app = headless();
    frames_of_a_fifth(&mut app);
    press(&mut app, Press::ToggleRegistering);
    app.update();
    let poses = card_poses(&mut app);
    let [(Panel::SignIn, leaving, _), (Panel::Create, coming, _)] = poses[..] else {
        panic!("both sides stand while it turns: {poses:?}");
    };
    assert!(
        leaving > 0.0 && leaving < 1.0 && coming > 0.0 && coming < 1.0,
        "each foreshortened as it turns: {poses:?}"
    );
    // Nothing fades on a carousel: the side turning away is darkened, not
    // thinned.
    let continuing = Phrase::Continue.text(Lang::En);
    assert_eq!(ink_on(&mut app, Panel::SignIn, continuing), Some(1.0));
    assert_eq!(ink_on(&mut app, Panel::Create, continuing), Some(1.0));
    let mut shades = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<FrontShade>>();
    let dark: Vec<f32> = shades.iter(app.world()).map(|s| s.0.alpha()).collect();
    assert!(
        dark.iter().filter(|a| **a > 0.0).count() == 2,
        "both sides are turned away from the light by some amount: {dark:?}"
    );

    frames_to_land(&mut app);
    assert_eq!(card_poses(&mut app), [(Panel::Create, 1.0, 1)]);
    assert!(presses(&mut app).contains(&Press::Focus(Field::PasswordAgain)));
}

#[test]
fn under_reduce_motion_the_front_door_changes_panel_at_once() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().gateways = vec!["https://hall.example".into()];
    to_gateway_face(&mut app);
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = true;
    app.update();
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    app.update();
    assert!(presses(&mut app).contains(&Press::Submit));
    assert_eq!(
        card_poses(&mut app),
        [(Panel::SignIn, 1.0, 1)],
        "and nothing moved on the way"
    );
    // As the gateway's answer would.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_registration_enabled(true);
    app.update();
    press(&mut app, Press::ToggleRegistering);
    app.update();
    assert_eq!(card_poses(&mut app), [(Panel::Create, 1.0, 1)]);
}

#[test]
fn a_saved_gateway_leaves_the_list_only_once_the_player_says_so() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateways = vec!["https://a.example".into(), "https://b.example".into()];
        state.uses.record("https://b.example");
    }
    to_gateway_face(&mut app);
    assert_eq!(
        app.world().resource::<LobbyState>().gateways,
        ["https://b.example", "https://a.example"],
        "the used one is drawn first"
    );
    let found = presses(&mut app);
    assert!(found.contains(&Press::ForgetGateway(0)) && found.contains(&Press::ForgetGateway(1)));

    press(&mut app, Press::ForgetGateway(0));
    assert!(
        labels(&mut app)
            .iter()
            .any(|l| l.contains("https://b.example")),
        "the question names the gateway"
    );
    press(&mut app, Press::CancelDestructive);
    assert_eq!(app.world().resource::<LobbyState>().gateways.len(), 2);

    press(&mut app, Press::ForgetGateway(0));
    press(&mut app, Press::ConfirmDestructive);
    assert_eq!(
        app.world().resource::<LobbyState>().gateways,
        ["https://a.example"]
    );
    let settings = app.world().resource::<crate::settings::ClientSettings>();
    assert_eq!(
        settings.gateways,
        ["https://a.example"],
        "and it stays gone"
    );
    assert_eq!(
        settings.gateway_uses.of("https://b.example").count,
        0,
        "with what this device knew about it"
    );
}

/// Presses one key that has no text of its own, and lets it land.
fn key(app: &mut App, code: KeyCode, key: Key) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(code, key));
    app.update();
}

#[test]
fn the_arrows_walk_the_saved_gateways_and_enter_goes_into_one() {
    let mut app = headless();
    app.world_mut().resource_mut::<LobbyState>().gateways = vec![
        "https://a.example".into(),
        "https://b.example".into(),
        "https://c.example".into(),
    ];
    to_gateway_face(&mut app);
    let cursor = |app: &App| app.world().resource::<LobbyState>().gateway_cursor;
    key(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    assert_eq!(cursor(&app), Some(0), "down from nowhere is the first row");
    for _ in 0..3 {
        key(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    }
    assert_eq!(
        cursor(&app),
        Some(2),
        "and the last row is as far as it goes"
    );
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(cursor(&app), Some(1));

    let lit = {
        let mut rows = app.world_mut().query::<(&Press, &BorderColor)>();
        rows.iter(app.world())
            .filter(|(press, border)| {
                matches!(press, Press::SelectGateway(_)) && border.top == palette::ACCENT
            })
            .map(|(press, _)| *press)
            .collect::<Vec<_>>()
    };
    assert_eq!(lit, [Press::SelectGateway(1)], "the row the arrows are on");

    key(&mut app, KeyCode::Enter, Key::Enter);
    let state = app.world().resource::<LobbyState>();
    assert!(state.lobby.gateway_chosen());
    assert_eq!(state.gateway, "https://b.example");

    settle(&mut app);
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert!(
        !app.world().resource::<LobbyState>().lobby.gateway_chosen(),
        "Escape is the way back out"
    );
}

#[test]
fn a_long_list_of_gateways_scrolls_in_a_frame_that_keeps_its_height() {
    /// The scrollbars on screen, and the height of the frame the list
    /// stands in.
    fn framed(app: &mut App, count: usize) -> (usize, Option<Val>) {
        app.world_mut().resource_mut::<LobbyState>().gateways = (0..count)
            .map(|n| format!("https://gw{n}.example"))
            .collect();
        to_gateway_face(app);
        let bars = app
            .world_mut()
            .query::<&bevy::ui_widgets::Scrollbar>()
            .iter(app.world())
            .count();
        let list = app
            .world_mut()
            .query::<(Entity, &Scrollable)>()
            .iter(app.world())
            .find(|(_, s)| s.0 == List::Gateways)
            .map(|(entity, _)| entity)
            .expect("the gateway list");
        let world = app.world();
        let frame = world
            .get::<ChildOf>(list)
            .and_then(|host| world.get::<ChildOf>(host.parent()))
            .and_then(|frame| world.get::<Node>(frame.parent()))
            .map(|node| node.height);
        (bars, frame)
    }
    let mut app = headless();
    let (bars, _) = framed(&mut app, super::super::gateway::ROWS_IN_SIGHT);
    assert_eq!(bars, 0, "a list that fits does not scroll");
    let (bars, eight) = framed(&mut app, 8);
    assert_eq!(bars, 1);
    let (_, twelve) = framed(&mut app, 12);
    assert!(matches!(eight, Some(Val::Px(h)) if h > 0.0), "{eight:?}");
    assert_eq!(eight, twelve, "more gateways do not make the panel taller");

    // The row the arrows reach is scrolled into sight.
    let offset = |app: &mut App| {
        app.world_mut()
            .query::<(&Scrollable, &ScrollPosition)>()
            .iter(app.world())
            .find(|(s, _)| s.0 == List::Gateways)
            .map(|(_, at)| at.y)
            .expect("the gateway list")
    };
    assert!(offset(&mut app) <= 0.0);
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(
        app.world().resource::<LobbyState>().gateway_cursor,
        Some(11)
    );
    let bottom = offset(&mut app);
    assert!(bottom > 0.0, "the last row is brought into sight: {bottom}");
    for _ in 0..11 {
        key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    }
    let top = offset(&mut app);
    assert!(top.abs() < 0.5, "and the first: {top}");
}

/// The gateway's mirror serves only a session (#273), so card art comes from
/// it only while signed in there, and the session goes with it; signed out,
/// or at a gateway without a mirror, art comes from Scryfall and the mirror
/// is shown nothing.
///
/// Through `art_source` rather than the system that applies it: the base is
/// process-wide, and a test that set it would move every other test's art.
#[test]
fn card_art_comes_from_the_mirror_only_while_signed_in_there() {
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
    state.gateway = "http://127.0.0.1:28766".to_string();
    state.art_cache = true;
    assert_eq!(art_source(&state), (None, None), "signed out: Scryfall");

    state.lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: None,
    });
    assert_eq!(
        art_source(&state),
        (Some("http://127.0.0.1:28766/art".to_string()), Some("tok"))
    );

    state.art_cache = false;
    assert_eq!(
        art_source(&state).0,
        None,
        "a gateway without a mirror leaves art with Scryfall"
    );

    state.art_cache = true;
    state.lobby.sign_out();
    assert_eq!(art_source(&state), (None, None), "and signing out ends it");
}
