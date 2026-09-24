//! The front door, and the text entry it is made of: the controls the form draws, the keyboard that fills them, and the address a launch remembers from the last sign-in that actually worked. The caret is here because this form is where the client draws its own text boxes — it is a node standing between two runs rather than a measured offset, so the row it sits in is the screen — and with it the selection fill, ⇧Tab and the arrows, the password bullets, the eye that lifts them and the blink `reduce_motion` holds still. The builder's own fields are exercised beside the builder; nothing here knows about decks, tables or a seat.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_sign_in_screen_builds_with_its_controls() {
    let mut app = headless();
    assert_eq!(roots(&mut app).len(), 1, "exactly one tree");
    let found = presses(&mut app);
    for wanted in [
        Press::Focus(Field::Email),
        Press::Focus(Field::Password),
        Press::Submit,
        Press::ToggleRegistering,
        Press::LeaveGateway,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    assert!(
        !found.contains(&Press::Focus(Field::DisplayName)),
        "the display name is only asked for when registering"
    );
    for elsewhere in [Press::PlayOffline, Press::AddGateway] {
        assert!(
            !found.contains(&elsewhere),
            "{elsewhere:?} is on the other face of the card"
        );
    }
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
    app.update();
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
        Press::OpenSettings,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    for elsewhere in [
        Press::Submit,
        Press::Focus(Field::Email),
        Press::LeaveGateway,
    ] {
        assert!(
            !found.contains(&elsewhere),
            "{elsewhere:?} is on the other face of the card"
        );
    }
    assert!(
        labels(&mut app).iter().any(|l| l == baylee_build::short()),
        "the build is on this face too"
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
            .field(Field::Email),
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
            .field(Field::Email),
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
            .buffer(Field::Email)
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
            .field(Field::Email),
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
    assert_eq!(drawn_field(&mut app, Field::Email), ["a", "|", "b"]);
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
        drawn_field(&mut app, Field::Email),
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
            .buffer(Field::Email)
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
        !presses(&mut app).contains(&Press::Reveal(Field::Email)),
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
fn the_sign_in_box_opens_on_the_address_that_used_it_last() {
    let blank = LobbyState::from_settings(crate::settings::ClientSettings::default());
    assert_eq!(blank.lobby.field(Field::Email), "");
    assert_eq!(
        blank.lobby.focus(),
        Field::Gateway,
        "the gateway form is the one on screen at launch"
    );

    let known = LobbyState::from_settings(crate::settings::ClientSettings {
        last_email: "mail@acevik.de".to_string(),
        ..crate::settings::ClientSettings::default()
    });
    assert_eq!(known.lobby.field(Field::Email), "mail@acevik.de");
    assert_eq!(
        known.lobby.focus(),
        Field::Gateway,
        "the gateway form is the one on screen at launch"
    );
}

/// The remembered address outlives the choice of gateway, which is the only
/// way the sign-in form is ever reached.
#[test]
fn choosing_a_gateway_keeps_the_address_that_used_it_last() {
    let mut known = LobbyState::from_settings(crate::settings::ClientSettings {
        last_email: "mail@acevik.de".to_string(),
        gateways: vec!["https://one.example".into()],
        ..crate::settings::ClientSettings::default()
    });
    assert!(known.select_gateway(0));
    assert_eq!(known.lobby.field(Field::Email), "mail@acevik.de");
    assert_eq!(known.lobby.focus(), Field::Password);

    known.leave_gateway();
    assert!(!known.gateway_selected);
    assert_eq!(known.lobby.focus(), Field::Gateway);
    assert!(known.select_gateway(0));
    assert_eq!(
        known.lobby.field(Field::Email),
        "mail@acevik.de",
        "and a round trip through the gateway form"
    );
}

/// And it is written down by the sign-in that worked, not by the attempt.
#[test]
fn only_a_sign_in_that_worked_is_worth_remembering() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_field(Field::Email, "mail@acevik.de");
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
    assert_eq!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .last_email,
        "",
        "a refused attempt says nothing about the address"
    );

    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        }));
    app.update();
    assert_eq!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .last_email,
        "mail@acevik.de"
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
        .set_field(Field::Email, "review@example.invalid");
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
            .set_field(Field::Email, "review@example.invalid");
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
        ];
        let newer = gateway_info(None, "9.9.9", baylee_view::VIEW_VERSION + 1);
        let same = gateway_info(None, "0.1.0", baylee_view::VIEW_VERSION);
        state.probes.insert("https://newer.example".into(), newer);
        state.probes.insert("https://same.example".into(), same);
        state
            .probes
            .insert("https://older.example".into(), Probe::Older);
    }
    app.update();
    let mut query = app.world_mut().query::<(&Text, &TextColor)>();
    let inks: Vec<(String, Color)> = query
        .iter(app.world())
        .map(|(text, colour)| (text.0.clone(), colour.0))
        .collect();
    let ink = |label: &str| inks.iter().find(|(text, _)| text == label).map(|i| i.1);
    assert_eq!(ink("9.9.9"), Some(palette::DANGER));
    assert_eq!(ink("0.1.0"), Some(palette::HEAL));
    assert_eq!(
        ink(Phrase::GatewayVersionUnknown.text(Lang::En)),
        Some(palette::ACTIVE)
    );

    let mut hints = app
        .world_mut()
        .query::<(Entity, &super::super::hint::HoverHint)>();
    let offline = Phrase::OfflineBenefit.text(Lang::En);
    let marks: Vec<(Entity, String)> = hints
        .iter(app.world())
        .map(|(entity, hint)| (entity, hint.0.clone()))
        .filter(|(_, said)| said != offline)
        .collect();
    assert_eq!(marks.len(), 2, "the compatible gateway carries no mark");
    let newer = (baylee_view::VIEW_VERSION + 1).to_string();
    let (mark, _) = marks
        .iter()
        .find(|(_, said)| said.contains(&newer))
        .expect("the mismatch names the gateway's view version");

    // Pointing at the mark draws its sentence; leaving takes it away.
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut()
        .write_message(aimed(*mark, Over { hit: hit.clone() }));
    app.update();
    assert!(labels(&mut app).iter().any(|l| l.contains(&newer)));
    app.world_mut().write_message(aimed(*mark, Out { hit }));
    app.update();
    assert!(!labels(&mut app).iter().any(|l| l.contains(&newer)));
}

/// The front door card's scale across and its drift sideways, as posed.
fn card_pose(app: &mut App) -> (f32, f32) {
    let mut cards = app
        .world_mut()
        .query_filtered::<&bevy::ui::UiTransform, With<super::super::front::FrontCard>>();
    let pose = cards.single(app.world()).expect("one card");
    let drift = match pose.translation.x {
        Val::Px(px) => px,
        _ => 0.0,
    };
    (pose.scale.x, drift)
}

#[test]
fn choosing_a_gateway_turns_the_card_over_and_back_turns_it_home() {
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateways = vec!["https://hall.example".into()];
        let hall = gateway_info(Some("Hall"), "0.1.0", baylee_view::VIEW_VERSION);
        state.probes.insert("https://hall.example".into(), hall);
    }
    to_gateway_face(&mut app);
    assert_eq!(
        card_pose(&mut app),
        (1.0, 0.0),
        "a card at rest carries no pose"
    );

    // A little over a quarter of the turn per frame.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    assert!(
        app.world_mut()
            .resource_mut::<LobbyState>()
            .select_gateway(0)
    );
    app.update();
    let (scale, drift) = card_pose(&mut app);
    assert!(scale > 0.0 && scale < 0.95, "part way round: {scale}");
    assert!(drift < 0.0, "going forward it leans left: {drift}");
    assert!(
        presses(&mut app).contains(&Press::AddGateway),
        "the gateway face stays up until the card is edge-on"
    );
    // Pressed while it turns, the half-gone face answers nothing.
    tap_control(&mut app, "save gateway", |p| *p == Press::AddGateway);
    assert_eq!(app.world().resource::<LobbyState>().adding, None);

    app.update();
    app.update();
    assert_eq!(card_pose(&mut app), (1.0, 0.0), "landed, and at rest again");
    let found = presses(&mut app);
    assert!(found.contains(&Press::Submit) && found.contains(&Press::LeaveGateway));
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l == "Hall"),
        "the account face names its gateway"
    );
    assert!(drawn.iter().any(|l| l == baylee_build::short()));

    app.world_mut().resource_mut::<LobbyState>().leave_gateway();
    app.update();
    let (_, drift) = card_pose(&mut app);
    assert!(drift > 0.0, "coming back it leans the other way: {drift}");
    for _ in 0..3 {
        app.update();
    }
    assert!(presses(&mut app).contains(&Press::AddGateway));
    assert_eq!(card_pose(&mut app), (1.0, 0.0));
}

#[test]
fn under_reduce_motion_the_card_changes_face_at_once() {
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
        card_pose(&mut app),
        (1.0, 0.0),
        "and nothing moved on the way"
    );
}
