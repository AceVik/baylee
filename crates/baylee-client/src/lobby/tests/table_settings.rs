//! The settings over a table (`table_settings`): the game menu's row asks,
//! the lobby stands its settings screen over the playing table, offers only
//! what leaves the table where it is, answers its presses and keys, and puts
//! it away on `Esc` and with the table.

use super::super::table_settings::OverTable;
#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_client_core::settings_map::Section;

/// The roots the settings stand in over the table.
fn over_table(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<(), With<OverTable>>();
    query.iter(app.world()).count()
}

/// The sections the nav draws.
fn sections(app: &mut App) -> Vec<Section> {
    presses(app)
        .into_iter()
        .filter_map(|p| match p {
            Press::Settings(SettingsPress::Section(s)) => Some(s),
            _ => None,
        })
        .collect()
}

/// A signed-in lobby whose table now plays, the settings last shown on the
/// account's section, and the game menu's Settings pressed.
fn asked_at_a_table() -> App {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .set_settings_section(Section::Account);
    app.update();
    app.insert_resource(crate::Duel::default());
    phase(&mut app, DuelPhase::Playing);
    {
        // The menu stands open, as it does when its row is pressed.
        let mut duel = app.world_mut().resource_mut::<crate::Duel>();
        duel.game_menu = true;
        duel.settings_asked = true;
    }
    app.update();
    app.update();
    app
}

/// The join, from the menu's flag to the screen on the table: the lobby
/// hears the ask, draws the screen in a root of its own (the lobby's own
/// tree stays down), offers only the table's sections, moves off the
/// account's section it was left on, and answers a press there.
#[test]
fn the_game_menu_s_settings_stand_over_the_table() {
    let mut app = asked_at_a_table();
    assert!(
        !app.world().resource::<crate::Duel>().settings_asked,
        "the ask was not heard"
    );
    assert!(
        !app.world().resource::<crate::Duel>().game_menu,
        "the menu stayed open under the settings"
    );
    let state = app.world().resource::<LobbyState>();
    assert!(
        state.settings_over_table(),
        "the settings are not over the table"
    );
    assert!(
        state.settings_section().at_table(),
        "{:?} is shown over a table",
        state.settings_section()
    );
    assert_eq!(over_table(&mut app), 1, "no root over the table");
    assert!(roots(&mut app).is_empty(), "the lobby's own tree is drawn");

    let offered = sections(&mut app);
    let want: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|s| s.at_table() && s.offered(crate::settingsui::builds()))
        .collect();
    let mut drawn = offered.clone();
    drawn.sort();
    let mut wanted = want.clone();
    wanted.sort();
    assert_eq!(drawn, wanted, "the nav over a table");

    // A press on the screen is answered over the table: the lobby's clicks
    // run for it.
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(Section::Controls)),
    );
    assert_eq!(
        app.world().resource::<LobbyState>().settings_section(),
        Section::Controls
    );
    // And no road reaches a section kept in the lobby.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .set_settings_section(Section::Account);
    assert_eq!(
        app.world().resource::<LobbyState>().settings_section(),
        Section::Controls,
        "the account's section was shown over a table"
    );
}

/// `Esc` puts the screen away, and the table never hears it: the key is
/// gone from the frame's input when the lobby has done with it.
#[test]
fn esc_puts_the_settings_over_a_table_away() {
    let mut app = asked_at_a_table();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        !app.world().resource::<LobbyState>().settings_open(),
        "Esc left the settings up"
    );
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Escape),
        "the table would hear the Esc that closed the settings"
    );
    app.update();
    assert_eq!(over_table(&mut app), 0, "the root stayed");
}

/// Back puts them away over a table as it does in the lobby: the press is
/// answered there, and the root goes. (That the table's keys are held while
/// they stand is `input::tests::keyboard`'s.)
#[test]
fn back_puts_the_settings_over_a_table_away() {
    let mut app = asked_at_a_table();
    assert!(app.world().resource::<LobbyState>().settings_over_table());
    press(&mut app, Press::Settings(SettingsPress::CloseSettings));
    assert!(
        !app.world().resource::<LobbyState>().settings_over_table(),
        "Back was not answered over the table"
    );
    app.update();
    assert_eq!(over_table(&mut app), 0, "the root stayed");
}

/// The table put down puts them down too, and the lobby comes back on its
/// own screen, where the settings may show every section again.
#[test]
fn the_settings_go_with_the_table() {
    let mut app = asked_at_a_table();
    phase(&mut app, DuelPhase::Finished);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.settings_open(), "the settings outlived the table");
    assert!(!state.settings_view.over_table);
    app.update();
    assert_eq!(over_table(&mut app), 0, "the root outlived the table");
    phase(&mut app, DuelPhase::Closed);
    app.update();
    assert!(!roots(&mut app).is_empty(), "the lobby did not come back");
    app.world_mut()
        .resource_mut::<LobbyState>()
        .set_settings_section(Section::Account);
    assert_eq!(
        app.world().resource::<LobbyState>().settings_section(),
        Section::Account,
        "the lobby's settings lost a section"
    );
}
