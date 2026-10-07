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

    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    assert!(app.world().resource::<LobbyState>().settings.is_open());

    press(
        &mut app,
        Press::Settings(SettingsPress::Rebind(Action::Confirm)),
    );
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
    press(
        &mut app,
        Press::Settings(SettingsPress::ResetBinding(Action::Confirm)),
    );
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
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));

    press(
        &mut app,
        Press::Settings(SettingsPress::Rebind(Action::Cancel)),
    );
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
    press(
        &mut app,
        Press::Settings(SettingsPress::Rebind(Action::Cancel)),
    );
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
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));

    let found = presses(&mut app);
    for action in Action::ALL {
        assert!(
            found.contains(&Press::Settings(SettingsPress::Rebind(action))),
            "{action:?} cannot be rebound from the screen"
        );
    }
    for rule in AutoRule::ALL {
        assert!(
            found.contains(&Press::Settings(SettingsPress::ToggleAuto(rule))),
            "{rule:?} is missing"
        );
    }
    for side in RailSide::BOTH {
        for row in RAIL_ROWS {
            assert!(
                found.contains(&Press::Settings(SettingsPress::ToggleRail(side, row))),
                "{side:?}/{row:?} is missing from the rail"
            );
        }
    }
    for preset in RailPreset::ALL {
        assert!(
            found.contains(&Press::Settings(SettingsPress::SetRail(preset))),
            "{preset:?} cannot be reached from the screen"
        );
    }
    assert!(
        found.contains(&Press::Settings(SettingsPress::CloseSettings)),
        "no way back"
    );

    // A preset writes the whole rail, and the rail it writes is one nothing
    // else on the screen could have produced by accident.
    press(
        &mut app,
        Press::Settings(SettingsPress::SetRail(RailPreset::Competitive)),
    );
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

    // A switch actually flips, both ways. Asked against where it stood and
    // not against a constant: this test once passed only on a machine whose
    // own preferences file had the switch off, and failed wherever the
    // default was read instead.
    let skips = |app: &App| {
        app.world()
            .resource::<crate::prefs::Prefs>()
            .auto()
            .skip_empty_blocks
    };
    let before = skips(&app);
    press(
        &mut app,
        Press::Settings(SettingsPress::ToggleAuto(AutoRule::SkipEmptyBlocks)),
    );
    assert_eq!(skips(&app), !before, "the switch did not take");
    press(
        &mut app,
        Press::Settings(SettingsPress::ToggleAuto(AutoRule::SkipEmptyBlocks)),
    );
    assert_eq!(skips(&app), before, "the switch did not come back");
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
    press(&mut app, Press::Settings(SettingsPress::CloseSettings));
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the builder was lost"
    );
    assert!(
        presses(&mut app).contains(&Press::Build(BuildPress::CloseBuilder)),
        "not redrawn"
    );
}

/// Audio lives behind settings, leaving the header for navigation.
#[test]
fn music_controls_live_in_quick_settings_and_full_settings() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    assert!(!labels(&mut app).iter().any(|l| l == "50 %"));
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    assert!(labels(&mut app).iter().any(|l| l == "50 %"));
    assert!(labels(&mut app).iter().any(|l| l == "Music"));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    assert!(labels(&mut app).iter().any(|l| l == "50 %"));
}

#[test]
fn settings_scroll_by_wheel_and_swipe_and_keep_the_offset_after_an_edit() {
    for width in [390.0, 1400.0] {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, width);
        app.update();
        press(&mut app, Press::Front(FrontPress::FrontMenu));
        press(&mut app, Press::Settings(SettingsPress::OpenSettings));
        let list = app
            .world_mut()
            .query::<(Entity, &Scrollable)>()
            .iter(app.world())
            .find(|(_, s)| s.0 == List::Settings)
            .unwrap()
            .0;
        assert_eq!(
            app.world().get::<Node>(list).unwrap().overflow.y,
            OverflowAxis::Scroll
        );
        app.world_mut().entity_mut(list).insert(ComputedNode {
            size: Vec2::new(width, 300.0),
            content_size: Vec2::new(width, 1900.0),
            ..default()
        });
        let row = press_target(&mut app, Press::Settings(SettingsPress::ResetAllBindings));
        app.world_mut()
            .resource_mut::<Messages<Pointer<Scroll>>>()
            .write(aimed(
                row,
                Scroll {
                    unit: MouseScrollUnit::Pixel,
                    x: 0.0,
                    y: -80.0,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    phase: bevy::input::touch::TouchPhase::Moved,
                },
            ));
        app.update();
        assert!((app.world().get::<ScrollPosition>(list).unwrap().y - 80.0).abs() < 0.01);
        app.world_mut()
            .resource_mut::<Messages<Pointer<Drag>>>()
            .write(aimed(
                row,
                Drag {
                    button: PointerButton::Primary,
                    distance: Vec2::new(0.0, -40.0),
                    delta: Vec2::new(0.0, -40.0),
                },
            ));
        app.update();
        assert!((app.world().get::<ScrollPosition>(list).unwrap().y - 120.0).abs() < 0.01);
        press(
            &mut app,
            Press::Settings(SettingsPress::Rebind(
                baylee_client_core::prefs::Action::Confirm,
            )),
        );
        let position = app
            .world_mut()
            .query::<(&Scrollable, &ScrollPosition)>()
            .iter(app.world())
            .find(|(s, _)| s.0 == List::Settings)
            .unwrap()
            .1
            .y;
        assert!(
            (position - 120.0).abs() < 0.01,
            "settings edit reset scroll: {position}"
        );
    }
}

/// The language-model seat's panel, driven the way a player drives it:
/// a press adds a profile, keys type into its boxes, `Tab` moves on and
/// `Enter` saves. A key pasted into the model's box is refused beside it,
/// Save is gone from the screen, and `Enter` writes nothing; mended, the
/// file is written. In a scratch directory: a test never opens the
/// player's own file.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_seat_panel_is_typed_into_and_saved_from_the_settings_screen() {
    use baylee_client_core::i18n::Phrase;
    use baylee_client_core::llmseat::panel::{Act, Slot, Spot};

    fn keys(app: &mut App, events: Vec<KeyboardInput>) {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for event in events {
            messages.write(event);
        }
        app.update();
    }
    fn text(s: &str) -> Vec<KeyboardInput> {
        s.chars().map(typed).collect()
    }
    fn select_all(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::SuperLeft);
        keys(app, vec![typed('a')]);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::SuperLeft);
    }
    fn focus(app: &App) -> Option<Spot> {
        app.world()
            .resource::<LobbyState>()
            .seat
            .panel()
            .and_then(baylee_client_core::llmseat::panel::SeatPanel::focus)
    }

    let dir = std::env::temp_dir().join(format!("baylee-seatpanel-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(baylee_client_core::llmseat::FILE);

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    app.world_mut()
        .resource_mut::<LobbyState>()
        .seat
        .open_at(path.clone());
    app.update();
    let empty = Phrase::SeatEmpty.text(Lang::En);
    assert!(
        labels(&mut app).iter().any(|l| l == empty),
        "no file, and it says so"
    );

    press(&mut app, Press::Settings(SettingsPress::Seat(Act::Add)));
    // The new profile's name is selected: typing replaces it.
    keys(&mut app, text("mine"));
    keys(&mut app, vec![pressed(KeyCode::Tab, Key::Tab)]);
    assert_eq!(focus(&app), Some(Spot::Profile(0, Slot::Model)));
    select_all(&mut app);
    keys(&mut app, text("sk-ant-api03-AAAABBBBCCCCDDDDEEEE"));
    let refused = Phrase::SeatFaultKeyShaped.text(Lang::En);
    assert!(
        labels(&mut app).iter().any(|l| l == refused),
        "the key is refused beside its box"
    );
    assert!(
        !presses(&mut app).contains(&Press::Settings(SettingsPress::Seat(Act::Save))),
        "Save stands dead while a fault does"
    );
    keys(&mut app, vec![pressed(KeyCode::Enter, Key::Enter)]);
    assert!(!path.exists(), "a key is never written");

    select_all(&mut app);
    keys(&mut app, text("claude-opus-5-5"));
    assert!(presses(&mut app).contains(&Press::Settings(SettingsPress::Seat(Act::Save))));
    keys(&mut app, vec![pressed(KeyCode::Enter, Key::Enter)]);
    let written = std::fs::read_to_string(&path).expect("saved");
    assert!(
        written.contains("\"mine\"") && written.contains("claude-opus-5-5"),
        "{written}"
    );
    assert!(!written.contains("AAAABBBB"), "{written}");
    let saved = Phrase::SeatSavedNote.text(Lang::En);
    assert!(labels(&mut app).iter().any(|l| l == saved));

    // Anything else pressed takes the caret out of the panel.
    press(
        &mut app,
        Press::Settings(SettingsPress::Rebind(
            baylee_client_core::prefs::Action::Confirm,
        )),
    );
    assert_eq!(focus(&app), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A model's key box, as a player meets it: a preset fills an adapter,
/// and its address has a sealed box. What is typed into it is drawn only
/// as dots, never in the clear and with no eye to show it; `Keep` takes
/// it out of the box and hands it to the store. In a test the store is
/// never the player's: the job is answered "unavailable" without a
/// process run, and the box says so.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_key_typed_into_its_box_is_only_ever_drawn_as_dots() {
    use crate::seatpanel::KeyPress;
    use baylee_client_core::i18n::Phrase;
    use baylee_client_core::llmseat::keys::KeyState;
    use baylee_client_core::llmseat::panel::Act;
    use baylee_client_core::llmseat::seating::Preset;

    fn keys(app: &mut App, events: Vec<KeyboardInput>) {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for event in events {
            messages.write(event);
        }
        app.update();
    }
    const SECRET: &str = "sk-test-SECRETSECRETSECRET";

    let dir = std::env::temp_dir().join(format!("baylee-seatkey-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(baylee_client_core::llmseat::FILE);

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    app.world_mut()
        .resource_mut::<LobbyState>()
        .seat
        .open_at(path.clone());
    app.update();
    press(
        &mut app,
        Press::Settings(SettingsPress::Seat(Act::AddPreset(Preset::DeepSeek))),
    );
    // The frame after the box is drawn asks the store about it.
    app.update();

    // Untold, the box asks the store; a test's store is none.
    let unavailable = Phrase::SeatKeyStoreUnavailable.fill(Lang::En, &["not in a test"]);
    assert!(
        labels(&mut app).contains(&unavailable),
        "a test reached for a store: {:?}",
        labels(&mut app)
    );
    assert!(
        !presses(&mut app).contains(&Press::Settings(SettingsPress::SeatKey(KeyPress::Focus(0)))),
        "no box where no store can keep a key"
    );

    // Told that none is kept, the box stands, sealed.
    let entry = app
        .world()
        .resource::<LobbyState>()
        .seat
        .panel()
        .and_then(|panel| panel.key_entry(0, &|_| None))
        .expect("the preset names a key and an address");
    assert_eq!(entry.host(), "api.deepseek.com");
    app.world_mut()
        .resource_mut::<LobbyState>()
        .seat
        .keys_mut()
        .answered(&entry, Ok(KeyState::Absent));
    app.update();
    let none = Phrase::SeatKeyNoneKept.fill(Lang::En, &["api.deepseek.com"]);
    assert!(labels(&mut app).contains(&none));
    assert!(
        !presses(&mut app)
            .iter()
            .any(|p| matches!(p, Press::Shared(SharedPress::Reveal(_)))),
        "a key box has no eye"
    );

    press(
        &mut app,
        Press::Settings(SettingsPress::SeatKey(KeyPress::Focus(0))),
    );
    keys(&mut app, SECRET.chars().map(typed).collect());
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().all(|l| !l.contains("SECRET")),
        "the key is drawn in the clear: {drawn:?}"
    );
    let dots = "\u{2022}".repeat(SECRET.chars().count());
    assert!(
        drawn.iter().any(|l| l.contains(&dots)),
        "the key is not drawn as dots: {drawn:?}"
    );
    assert!(presses(&mut app).contains(&Press::Settings(SettingsPress::SeatKey(KeyPress::Submit))));

    keys(&mut app, vec![pressed(KeyCode::Enter, Key::Enter)]);
    app.update();
    assert!(
        !app.world().resource::<LobbyState>().seat.typing(),
        "the key stayed in its box"
    );
    assert!(labels(&mut app).contains(&unavailable));
    assert!(labels(&mut app).iter().all(|l| !l.contains("SECRET")));
    assert!(!path.exists(), "a key box writes no file");
    let _ = std::fs::remove_dir_all(&dir);
}
