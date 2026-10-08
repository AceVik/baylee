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
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Controls,
        )),
    );
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
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Controls,
        )),
    );

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
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Controls,
        )),
    );

    let found = presses(&mut app);
    for action in Action::ALL {
        assert!(
            found.contains(&Press::Settings(SettingsPress::Rebind(action))),
            "{action:?} cannot be rebound from the screen"
        );
    }
    // Automation, where to stop and the presets are Gameplay's (WP5).
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Gameplay,
        )),
    );
    let found = presses(&mut app);
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

/// DESIGN-v7's three device controls: the table angle and the visit camera
/// under Graphics, the priority sound under Audio — drawn there, and each
/// press written to this device's settings.
#[test]
fn the_table_angle_the_visit_camera_and_the_priority_sound_are_settings() {
    use baylee_client_core::settings_map::Section;
    use baylee_client_core::tableview::{RingLean, VisitCamera};
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
    app.update();
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(Section::Graphics)),
    );
    let shown = labels(&mut app);
    for label in [
        "Table angle",
        "Steep",
        "Flat",
        "Looking at a seat",
        "Across",
    ] {
        assert!(
            shown.iter().any(|l| l == label),
            "{label} is not on Graphics"
        );
    }
    press(
        &mut app,
        Press::Settings(SettingsPress::TableLean(RingLean::Gentle)),
    );
    press(
        &mut app,
        Press::Settings(SettingsPress::VisitCamera(VisitCamera::Across)),
    );
    let table = app
        .world()
        .resource::<crate::settings::ClientSettings>()
        .table;
    assert_eq!(table.lean, RingLean::Gentle);
    assert_eq!(table.visit, VisitCamera::Across);
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(Section::Audio)),
    );
    assert!(labels(&mut app).iter().any(|l| l == "Priority sound"));
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .audio
            .priority_cue
    );
    press(&mut app, Press::Settings(SettingsPress::PriorityCue));
    assert!(
        !app.world()
            .resource::<crate::settings::ClientSettings>()
            .audio
            .priority_cue
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
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Audio,
        )),
    );
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
        press(
            &mut app,
            Press::Settings(SettingsPress::Section(
                baylee_client_core::settings_map::Section::Controls,
            )),
        );
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
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::LanguageModels,
        )),
    );
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
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::Privacy,
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
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::LanguageModels,
        )),
    );
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

/// Language models (§12): the profiles stand in a list, and a profile's
/// fields on its sheet — the first ones shown, Advanced's behind its
/// disclosure; Close and Esc put the sheet away, and the list's row opens
/// it again.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_profile_is_edited_on_its_sheet_with_advanced_behind_a_disclosure() {
    use baylee_client_core::llmseat::panel::{Act, Slot};

    let dir = std::env::temp_dir().join(format!("baylee-seatsheet-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(
            baylee_client_core::settings_map::Section::LanguageModels,
        )),
    );
    app.world_mut()
        .resource_mut::<LobbyState>()
        .seat
        .open_at(dir.join(baylee_client_core::llmseat::FILE));
    app.update();
    let name = Phrase::SeatName.text(Lang::En).to_string();
    let tokens = Slot::MaxTokens.label().text(Lang::En).to_string();
    assert!(
        !labels(&mut app).contains(&name),
        "no sheet before a profile"
    );

    press(&mut app, Press::Settings(SettingsPress::Seat(Act::Add)));
    let shown = labels(&mut app);
    assert!(shown.contains(&name), "a new profile opens its sheet");
    assert!(!shown.contains(&tokens), "Advanced starts closed");
    press(&mut app, Press::Settings(SettingsPress::ProfileAdvanced));
    assert!(labels(&mut app).contains(&tokens), "Advanced opened");

    press(&mut app, Press::Settings(SettingsPress::CloseProfile));
    assert!(!labels(&mut app).contains(&name), "Close puts it away");
    assert!(
        presses(&mut app).contains(&Press::Settings(SettingsPress::OpenProfile(0))),
        "the profile is a row of the list"
    );
    press(&mut app, Press::Settings(SettingsPress::OpenProfile(0)));
    assert!(labels(&mut app).contains(&name), "the row opens it again");
    super::front_keys::press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(!labels(&mut app).contains(&name), "Esc puts it away");
    assert!(
        app.world().resource::<LobbyState>().settings.is_open(),
        "and only the sheet"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Opens the settings screen at `section`, signed in, with a settings file.
fn settings_at(section: baylee_client_core::settings_map::Section) -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::Front(FrontPress::FrontMenu));
    press(&mut app, Press::Settings(SettingsPress::OpenSettings));
    press(&mut app, Press::Settings(SettingsPress::Section(section)));
    app
}

/// §17 WP5: a graphics preset writes the device's rows only — never an
/// account field (atmosphere, hold the table still, sky, sound).
#[test]
fn a_preset_never_writes_an_account_field() {
    use baylee_client_core::graphics::Preset;
    let mut app = settings_at(baylee_client_core::settings_map::Section::Graphics);
    let account = |app: &App| {
        serde_json::to_string(app.world().resource::<crate::prefs::Prefs>().all()).unwrap()
    };
    let before = account(&app);
    for preset in [Preset::Low, Preset::Ultra, Preset::Medium, Preset::High] {
        press(
            &mut app,
            Press::Settings(SettingsPress::GraphicsPreset(preset)),
        );
        let device = app
            .world()
            .resource::<crate::settings::ClientSettings>()
            .graphics
            .map(|g| g.preset);
        assert_eq!(device, Some(preset), "the device took {preset:?}");
        assert_eq!(account(&app), before, "{preset:?} wrote the account");
    }
}

/// §17 WP5 (`KEYBOARD.md` §5): a shortcut's new key that another action
/// holds is refused with the holder named, nothing written; asked again,
/// it is taken.
#[test]
fn a_rebind_conflict_is_refused_with_its_reason_and_then_taken() {
    use baylee_client_core::shellkeys::{Refused, ShellAction, ShellChord};
    let mut app = settings_at(baylee_client_core::settings_map::Section::Controls);
    let keys = |app: &App| {
        app.world()
            .resource::<crate::prefs::Prefs>()
            .all()
            .shell_keys
            .clone()
    };
    let before = keys(&app);
    press(
        &mut app,
        Press::Settings(SettingsPress::RebindShell(ShellAction::CreateTable)),
    );
    super::front_keys::press_key(&mut app, KeyCode::Slash, Key::Character("/".into()), &[]);
    let why = crate::settingsui::bindings::refusal(Refused::Held(ShellAction::Search), Lang::En);
    assert!(
        labels(&mut app).iter().any(|l| l.contains(&why)),
        "the refusal names its holder: {why}"
    );
    assert_eq!(keys(&app), before, "a refusal writes nothing");
    press(&mut app, Press::Settings(SettingsPress::TakeShell));
    assert_eq!(
        keys(&app).chords(ShellAction::CreateTable),
        std::slice::from_ref(&ShellChord::ch("/")),
        "asked again, it is taken"
    );
}

/// `KEYBOARD.md` §1.5 (W10 step 1): on the nav, letters are type-ahead —
/// `c` `o` shows Controls, and focus stands on its item.
#[test]
fn letters_on_the_nav_jump_to_the_section_they_begin() {
    use super::front_keys::{focused, press_key};
    use baylee_client_core::settings_map::Section;
    let mut app = settings_at(Section::Graphics);
    for _ in 0..8 {
        if focused(&app).is_some_and(|s| s.id == "nav") {
            break;
        }
        press_key(&mut app, KeyCode::Tab, Key::Tab, &[]);
    }
    assert_eq!(focused(&app).map(|s| s.id), Some("nav"));
    press_key(&mut app, KeyCode::KeyC, Key::Character("c".into()), &[]);
    press_key(&mut app, KeyCode::KeyO, Key::Character("o".into()), &[]);
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.settings_section(), Section::Controls);
    let at = focused(&app).expect("focus");
    assert_eq!((at.id, at.item), ("nav", 3), "focus on Controls");
}

/// The arrangement row says which arrangement is the default: its button
/// stands apart from the seven others. It carried the composite's `Current`
/// for the keyboard and nothing for the eye (beta.6 QA).
#[test]
fn the_default_arrangement_is_drawn_as_chosen() {
    use baylee_client_core::settings_map::Section;
    use baylee_client_core::tableview::Arrangement;
    let mut app = settings_at(Section::Graphics);
    press(
        &mut app,
        Press::Settings(SettingsPress::Arrangement(Arrangement::Spotlight)),
    );
    let grounds: Vec<(Arrangement, Color)> = {
        let world = app.world_mut();
        let mut buttons = world.query::<(&Press, &Children)>();
        let mut faces = world.query::<&BackgroundColor>();
        buttons
            .iter(world)
            .filter_map(|(press, children)| match press {
                Press::Settings(SettingsPress::Arrangement(a)) => children
                    .iter()
                    .find_map(|c| faces.get(world, c).ok())
                    .map(|g| (*a, g.0)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(grounds.len(), Arrangement::ALL.len(), "{grounds:?}");
    let chosen = grounds
        .iter()
        .find(|(a, _)| *a == Arrangement::Spotlight)
        .map(|(_, g)| *g)
        .expect("drawn");
    let others: Vec<Color> = grounds
        .iter()
        .filter(|(a, _)| *a != Arrangement::Spotlight)
        .map(|(_, g)| *g)
        .collect();
    assert!(others.iter().all(|g| *g == others[0]), "{grounds:?}");
    assert_ne!(chosen, others[0], "the default looks like every other");
}

/// Updates draws each of its switches once. Its first row was a settings
/// row with an empty control — the label and help of "check automatically"
/// over nothing — standing above the updater's own switch saying the same
/// (beta.6 QA, "Automatisch prüfen" twice in German).
#[test]
fn updates_says_check_automatically_once() {
    use baylee_client_core::i18n::{Lang, Phrase};
    use baylee_client_core::settings_map::Section;
    let mut app = settings_at(Section::Updates);
    let shown = labels(&mut app);
    let count = |p: Phrase| shown.iter().filter(|l| *l == p.text(Lang::En)).count();
    assert_eq!(count(Phrase::UpdateAutoCheck), 1, "{shown:?}");
    assert_eq!(count(Phrase::RowCheckUpdates), 0, "{shown:?}");
}

/// `KEYBOARD.md` §7.8: `/` and Ctrl/Cmd+F focus the settings search, and
/// the key that opened it is not typed into it (§1.7). Both fired the
/// shell's Search and nothing answered it on this screen (beta.6 QA).
#[test]
fn slash_and_command_f_focus_the_settings_search() {
    use super::front_keys::{focused, press_key};
    use baylee_client_core::settings_map::Section;
    let command = if crate::shellkit::keys::mac() {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    };
    for (code, ch, held) in [
        (KeyCode::Slash, "/", vec![]),
        (KeyCode::KeyF, "f", vec![command]),
    ] {
        let mut app = settings_at(Section::Graphics);
        assert_ne!(focused(&app).map(|s| s.id), Some("search"));
        press_key(&mut app, code, Key::Character(ch.into()), &held);
        assert_eq!(focused(&app).map(|s| s.id), Some("search"), "{ch}");
        assert_eq!(
            app.world().resource::<LobbyState>().settings_query(),
            "",
            "{ch} is not typed into the search"
        );
    }
}
