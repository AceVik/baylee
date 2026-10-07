//! The shell keymap over the lobby (`KEYBOARD.md` §2, §7.1): a key typed
//! into the lobby's own fields fires nothing, the command chords pass them,
//! and `1 2 3` move between the screens where no field types.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::shellkit::keys::ShellLog;
use baylee_client_core::shellkeys::ShellAction;

fn signed_in() -> App {
    let mut app = headless();
    stocked(&mut app);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.screen(),
        &Screen::Table
    );
    app
}

fn command() -> KeyCode {
    if crate::shellkit::keys::mac() {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    }
}

fn key(app: &mut App, code: KeyCode, ch: &str, held: &[KeyCode]) {
    {
        let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for k in held {
            codes.press(*k);
        }
        codes.press(code);
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: Key::Character(ch.into()),
        state: bevy::input::ButtonState::Pressed,
        text: Some(ch.into()),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

fn fired(app: &App) -> Vec<ShellAction> {
    app.world().resource::<ShellLog>().fired.clone()
}

/// Play's search box owns every printable key, so `c` and `2` are typed
/// into it and fire nothing (§2.3); the screen stays where it was.
#[test]
fn a_key_typed_into_plays_search_fires_nothing() {
    let mut app = signed_in();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .focus_on(Field::Search);
    app.update();
    assert!(app.world().resource::<LobbyState>().lobby.typing_here());
    key(&mut app, KeyCode::KeyC, "c", &[]);
    key(&mut app, KeyCode::Digit2, "2", &[]);
    assert!(fired(&app).is_empty(), "{:?}", fired(&app));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.hub, Hub::Play);
    assert_eq!(state.lobby.field(Field::Search), "c2");
}

/// `Ctrl/Cmd+,` passes the search box and opens Settings; there no field
/// types, so `2` goes to Decks (closing Settings) and `1` back to Play.
#[test]
fn the_digits_move_between_the_screens_where_nothing_types() {
    let mut app = signed_in();
    key(&mut app, KeyCode::Comma, ",", &[command()]);
    assert!(app.world().resource::<LobbyState>().settings.is_open());
    key(&mut app, KeyCode::Digit2, "2", &[]);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(!state.settings.is_open());
        assert_eq!(state.hub, Hub::Decks);
    }
    key(&mut app, KeyCode::Comma, ",", &[command()]);
    key(&mut app, KeyCode::Digit1, "1", &[]);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.settings.is_open());
    assert_eq!(state.hub, Hub::Play);
    assert_eq!(
        fired(&app),
        [
            ShellAction::OpenSettings,
            ShellAction::GoDecks,
            ShellAction::OpenSettings,
            ShellAction::GoPlay
        ]
    );
}

/// The `?` overlay holds the keyboard: typed into its search, a key does
/// not also land in Play's search behind it.
#[test]
fn the_overlay_holds_the_lobbys_keys() {
    let mut app = signed_in();
    key(&mut app, KeyCode::Slash, "/", &[command()]);
    assert!(
        app.world()
            .resource::<crate::shellkit::overlay::Overlay>()
            .open
    );
    key(&mut app, KeyCode::KeyX, "x", &[]);
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Search),
        "",
        "typed behind the overlay"
    );
}

/// With no field typing on Play, `/` puts the caret in the search box and
/// is not typed there (`KEYBOARD.md` §1.7: the key that opened something is
/// not typed into it); the next key is.
#[test]
fn slash_moves_into_the_search_without_typing_itself() {
    let mut app = signed_in();
    assert!(!app.world().resource::<LobbyState>().lobby.typing_here());
    key(&mut app, KeyCode::Slash, "/", &[]);
    assert_eq!(fired(&app), [ShellAction::Search]);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.lobby.typing_here());
        assert_eq!(state.lobby.field(Field::Search), "");
    }
    key(&mut app, KeyCode::KeyC, "c", &[]);
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Search),
        "c"
    );
    assert_eq!(fired(&app), [ShellAction::Search], "c fired from the field");
}
