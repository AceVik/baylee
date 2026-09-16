//! The settings screen, which sits over the lobby rather than beside it. Every action is rebindable from it and every automation switch, rail row and preset is reachable, because a row that cannot be reached is a preference a player does not have; an armed row takes the next key, `Esc` backs out of it instead of binding itself, and `Backspace` unbinds, since a pointer still reaches everything. A preset writes the whole rail at once, and closing the screen has to put the lobby back exactly as it was, halfway through a deck included. What a bound key then does in a duel is decided elsewhere.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The whole rebinding flow, without a window: open the screen, arm a
/// row, press a key, and find it bound. Every step of it is a place the
/// keymap could quietly not be written.
#[test]
fn a_key_can_be_rebound_from_the_settings_screen() {
    use baylee_client_core::prefs::{Action, Chord};

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();

    press(&mut app, Press::OpenSettings);
    assert!(app.world().resource::<LobbyState>().settings.is_open());

    press(&mut app, Press::Rebind(Action::Confirm));
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        Some(Action::Confirm),
        "the row is not waiting for a key"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyP);
    app.update();
    assert_eq!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Confirm),
        &[Chord::key("KeyP")],
        "the key was not bound"
    );
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        None,
        "the row is still armed after taking a key"
    );

    // And it can be put back, one row at a time.
    press(&mut app, Press::ResetBinding(Action::Confirm));
    assert_eq!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Confirm),
        &[Chord::key("Space")]
    );
}

/// Escape is a key a player may legitimately want to bind, so while a row
/// is armed it means "never mind" rather than "cancel". Backspace is the
/// other way out: unbinding is a real answer, since a pointer still
/// reaches everything.
#[test]
fn arming_a_row_can_be_backed_out_of_or_used_to_unbind() {
    use baylee_client_core::prefs::Action;

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::OpenSettings);

    press(&mut app, Press::Rebind(Action::Cancel));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().settings.capturing(),
        None
    );
    assert!(
        !app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Cancel)
            .is_empty(),
        "escape rebound the row instead of backing out of it"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    press(&mut app, Press::Rebind(Action::Cancel));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Backspace);
    app.update();
    assert!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .keymap()
            .chords(Action::Cancel)
            .is_empty(),
        "backspace did not unbind the row"
    );
}

#[test]
fn the_settings_screen_offers_every_switch_and_both_rails() {
    use baylee_client_core::automation::{RAIL_ROWS, RailPreset, RailRow, RailSide};
    use baylee_client_core::prefs::{Action, AutoRule};

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::OpenSettings);

    let found = presses(&mut app);
    for action in Action::ALL {
        assert!(
            found.contains(&Press::Rebind(action)),
            "{action:?} cannot be rebound from the screen"
        );
    }
    for rule in AutoRule::ALL {
        assert!(
            found.contains(&Press::ToggleAuto(rule)),
            "{rule:?} is missing"
        );
    }
    for side in RailSide::BOTH {
        for row in RAIL_ROWS {
            assert!(
                found.contains(&Press::ToggleRail(side, row)),
                "{side:?}/{row:?} is missing from the rail"
            );
        }
    }
    for preset in RailPreset::ALL {
        assert!(
            found.contains(&Press::SetRail(preset)),
            "{preset:?} cannot be reached from the screen"
        );
    }
    assert!(found.contains(&Press::CloseSettings), "no way back");

    // A preset writes the whole rail, and the rail it writes is one nothing
    // else on the screen could have produced by accident.
    press(&mut app, Press::SetRail(RailPreset::Competitive));
    let orders = app
        .world()
        .resource::<crate::prefs::Prefs>()
        .orders()
        .clone();
    assert!(
        orders.is(RailPreset::Competitive),
        "the preset did not take"
    );
    assert!(
        orders.is_skipped(RailSide::Mine, RailRow::Upkeep),
        "a quiet window should be red"
    );
    assert!(
        !orders.is_skipped(RailSide::Theirs, RailRow::Blockers),
        "the row where this seat declares blocks must stay green"
    );

    // A switch actually flips, and the screen redraws to say so.
    press(&mut app, Press::ToggleAuto(AutoRule::SkipEmptyBlocks));
    assert!(
        app.world()
            .resource::<crate::prefs::Prefs>()
            .auto()
            .skip_empty_blocks,
        "the switch did not take"
    );
}

/// Settings sit *over* the lobby: coming back has to land exactly where
/// the player left, including halfway through a deck.
#[test]
fn closing_the_settings_puts_the_lobby_back_as_it_was() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    assert!(matches!(
        app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Build
    ));

    // The builder has no settings button of its own — the screen is
    // reached from the tables or from sign-in — so it is opened here the
    // way a press would.
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    press(&mut app, Press::CloseSettings);
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the builder was lost"
    );
    assert!(
        presses(&mut app).contains(&Press::CloseBuilder),
        "not redrawn"
    );
}
