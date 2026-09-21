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
        Press::PlayOffline,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    assert!(
        !found.contains(&Press::Focus(Field::DisplayName)),
        "the display name is only asked for when registering"
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
        Field::Email,
        "with nothing stored the caret starts at the top"
    );

    let known = LobbyState::from_settings(crate::settings::ClientSettings {
        last_email: "mail@acevik.de".to_string(),
        ..crate::settings::ClientSettings::default()
    });
    assert_eq!(known.lobby.field(Field::Email), "mail@acevik.de");
    assert_eq!(known.lobby.focus(), Field::Password);
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
        assert!(state.add_gateway());
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
