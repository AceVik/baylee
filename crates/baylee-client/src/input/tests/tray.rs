//! The zone panel's own controls, and the keyboard it borrows while it is open. A sheet with a search box in it is a box that has to hold fifteen of the twenty-six bound letters — `T` latches the text view and writes it to disk, `K`/`B` and `Y`/`N` would answer a mulligan or a question outright — so the box takes the keyboard a frame *after* the sheet opens, hands it back when the player says so, and answers what a text field answers: a caret by character and by word, Home and End, a shift-extended selection, Delete beside Backspace, and the command chord that selects rather than typing an "a". The view buttons sit here for the neighbouring reason: they are the one control on this panel that writes to the settings store, and they must write only the view. Answering a *game question* through the same sheet is `dialog_keys`, and the tap that opens a pile is `arming`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A view button is the one control on the dialog that writes to the
/// settings store rather than to the `Browser`.
///
/// Two halves, and the second is the one worth a test. The first is that
/// the click lands at all — three new buttons that change nothing is the
/// defect the sort key already shipped once. The second is that it writes
/// **only** the view: the sheet's remembered rectangle lives in the same
/// resource, and a handler that wrote the whole store back would park a
/// sheet nobody had moved.
#[test]
fn a_view_button_writes_the_view_and_nothing_else() {
    use crate::settings::ClientSettings;
    use baylee_client_core::browser::ViewMode;

    let (mut app, _, _) = menu_app(crate::Duel::default());
    let grid = app
        .world_mut()
        .spawn(crate::hud::TrayView {
            mode: ViewMode::Grid,
        })
        .id();
    let detailed = app
        .world_mut()
        .spawn(crate::hud::TrayView {
            mode: ViewMode::Detailed,
        })
        .id();

    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Detailed,
        "the list is what a player who has chosen nothing gets"
    );

    click(&mut app, grid);
    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Grid,
        "the grid button did not reach the setting the panel is drawn from"
    );
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_none(),
        "changing the view wrote a place nobody chose"
    );

    click(&mut app, detailed);
    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Detailed,
        "the view is a choice, not a ratchet"
    );
}

/// The graveyard is searchable, and the search term stays out of the game.
///
/// "Sortierbar, durchsuchbar, scrollbar" — the first and the last were
/// there and the middle one was not: `Browser::set_filter` was written
/// and no key or click ever reached it. What is pinned here is both
/// halves of the bargain: the box holds the keyboard from the moment the
/// sheet opens, and it lets go when the player says so, after which the
/// panel can stand open for a whole turn with the letters belonging to
/// the game again.
///
/// The first half is the owner's bug of 14.09., and it is pinned with the
/// word they actually typed. Fifteen of the twenty-six letters are bound,
/// so a German search term is a handful of game actions: `T` latched the
/// text view on and persisted it — every card in the duel drawn as its own
/// rules text — and `K`/`B`, `Y`/`N` would have answered a mulligan or a
/// yes/no question outright.
#[test]
fn a_search_term_reaches_the_box_and_never_the_game() {
    use bevy::prelude::*;

    let (mut app, window) = a_zone_dialog_that_has_just_opened();
    assert!(
        app.world().resource::<crate::Duel>().browser.is_typing(),
        "the sheet opened and left the keyboard with the table"
    );

    // The owner's own search term, typed into the panel they had just
    // opened. `S`, `T`, `E` and `F` are all bound; `T` is `ToggleTextView`
    // and the one that stayed, because it is written to disk.
    for (code, c) in [
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
    ] {
        type_letter(&mut app, window, code, c);
    }
    assert_eq!(
        filter_reads(&app),
        "sturmtief",
        "the search term missed the box"
    );
    assert!(panel_stands(&app), "a letter in the term closed the panel");
    assert!(
        !app.world()
            .resource::<crate::settings::ClientSettings>()
            .prefer_text_view,
        "the search term latched the text view on"
    );
    assert!(
        app.world().resource::<crate::Duel>().outbox().is_empty(),
        "the search term sent something to the engine"
    );
}

/// The other half of the same bargain: the box lets go on request, and
/// then the sheet can stand open for a whole turn with the letters
/// belonging to the game again.
#[test]
fn a_released_filter_box_hands_the_letters_back() {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    let (mut app, window) = a_zone_dialog_that_has_just_opened();
    type_letter(&mut app, window, KeyCode::KeyM, 'm');
    type_letter(&mut app, window, KeyCode::KeyO, 'o');
    assert_eq!(filter_reads(&app), "mo");

    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Backspace,
        logical_key: Key::Backspace,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(filter_reads(&app), "m", "backspace did not reach the box");

    // `G` is `ToggleBrowser`, so the claim is not merely that the filter
    // stopped growing — a key that went nowhere at all would satisfy
    // that. The action has to have *fired*.
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .browser
        .stop_typing();
    let was = panel_stands(&app);
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert_eq!(filter_reads(&app), "m", "the box typed after letting go");
    assert_ne!(panel_stands(&app), was, "a released box ate a bound key");
}

/// The browser had a pointer route and no keyboard one, which is exactly
/// the promise `docs/keyboard-map.md` makes and the reason the action was
/// added rather than the chip being the only way in.
///
/// It is also the one test that walks the whole `G` path with both systems
/// registered, which is what makes it the place the `typed.clear()` guard
/// is held: the frame `G` opens the sheet on writes a `KeyboardInput` that
/// nothing reads, and messages live two frames, so without the guard that
/// `g` is waiting in the queue when the box takes the keyboard a frame
/// later. The panel would open with `g` already typed into it.
///
/// The second half of the old test — `G` again shuts it — was true and is
/// no longer, which is the *point* of the change rather than a regression:
/// a box with the keyboard is a box a bound letter cannot reach past. The
/// way out is `Esc`, and then the latch is a latch again.
#[test]
fn the_browser_key_opens_the_tray_and_the_box_then_holds_the_letters() {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<crate::Duel>()
        .init_resource::<Keystrokes>()
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(
            Update,
            (browser_takes_the_keyboard.before(keyboard), keyboard),
        );
    let window = app.world_mut().spawn_empty().id();

    // `reset_all` and not `clear`: a key that is still held is not pressed
    // again, and the second press would fire nothing at all.
    let press = |app: &mut App, key: KeyCode| {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(key);
        app.update();
    };

    assert!(!panel_stands(&app));
    // A real press of `G`, character and all — because the character is
    // the thing that must not be typed.
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert!(panel_stands(&app), "the browser key did not open the tray");
    assert!(
        !app.world().resource::<crate::Duel>().browser.is_typing(),
        "the box takes the keyboard a frame later, not on the frame the \
         sheet opens: that frame's keystroke belongs to the game"
    );

    // The frame after, which in the client is simply the next one.
    app.update();
    assert!(
        app.world().resource::<crate::Duel>().browser.is_typing(),
        "the sheet opened and nothing gave the filter box the keyboard"
    );
    assert_eq!(
        filter_reads(&app),
        "",
        "the keystroke that opened the panel was typed into it"
    );

    // Now the same key is a letter. The latch is out of reach.
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert_eq!(filter_reads(&app), "g");
    assert!(
        panel_stands(&app),
        "a letter typed into the box shut the panel"
    );

    // `Esc` twice: the first empties the box, the second hands the
    // keyboard back. Two presses because a player who has typed a term
    // means the term, not the panel.
    press(&mut app, KeyCode::Escape);
    assert_eq!(filter_reads(&app), "");
    assert!(app.world().resource::<crate::Duel>().browser.is_typing());
    press(&mut app, KeyCode::Escape);
    assert!(!app.world().resource::<crate::Duel>().browser.is_typing());
    assert!(panel_stands(&app), "letting go of the box shut the panel");

    // And with the letters back at the table it is a latch again.
    press(&mut app, KeyCode::KeyG);
    assert!(
        !panel_stands(&app),
        "it is a latch, so the same key shuts it"
    );
}

/// The search box answers the keys a text field answers.
///
/// The owner named the lobby's boxes as the thing this one should be, and
/// this is the half a player presses: a caret that moves by character and
/// by word, Home and End, shift extending a selection, Delete beside
/// Backspace, and ⌘A. The box was a `String` with characters pushed onto
/// the end of it, so every one of these did nothing — and ⌘A typed an
/// "a", which is the one that also *corrupts* the search.
#[test]
fn the_search_box_answers_the_keys_a_text_field_answers() {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<crate::Duel>()
        .init_resource::<Keystrokes>()
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(Update, keyboard);
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<crate::Duel>().browser.open();
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .browser
        .start_typing();
    app.update();

    // One key, with whatever modifiers are named held down for it.
    let chord = |app: &mut App, code: KeyCode, key: Key, mods: &[KeyCode]| {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            for m in mods {
                keys.press(*m);
            }
            keys.press(code);
        }
        app.world_mut()
            .resource_mut::<Keystrokes>()
            .0
            .push(KeyboardInput {
                key_code: code,
                logical_key: key,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    };
    let caret = |app: &App| {
        app.world()
            .resource::<crate::Duel>()
            .browser
            .filter_field()
            .cursor()
    };

    for c in "Wald".chars() {
        type_letter(&mut app, window, KeyCode::KeyW, c);
    }
    assert_eq!(filter_reads(&app), "Wald");
    assert_eq!(caret(&app), 4);

    // Home, then one character right, then a letter typed *inside* the
    // word — the whole thing a caret is for.
    chord(&mut app, KeyCode::Home, Key::Home, &[]);
    assert_eq!(caret(&app), 0);
    chord(&mut app, KeyCode::ArrowRight, Key::ArrowRight, &[]);
    type_letter(&mut app, window, KeyCode::KeyU, 'u');
    assert_eq!(
        filter_reads(&app),
        "Wuald",
        "the caret was not where it said"
    );

    // ⇧End selects to the end, and typing replaces what is selected.
    chord(&mut app, KeyCode::End, Key::End, &[KeyCode::ShiftLeft]);
    assert_eq!(
        app.world()
            .resource::<crate::Duel>()
            .browser
            .filter_field()
            .selection(),
        Some(2..5),
        "shift did not extend a selection"
    );
    type_letter(&mut app, window, KeyCode::KeyO, 'o');
    assert_eq!(filter_reads(&app), "Wuo");

    // Delete forwards from the start, which Backspace cannot do.
    chord(&mut app, KeyCode::Home, Key::Home, &[]);
    chord(&mut app, KeyCode::Delete, Key::Delete, &[]);
    assert_eq!(filter_reads(&app), "uo");

    // And ⌘A selects the box instead of typing an "a" into it.
    chord(
        &mut app,
        KeyCode::KeyA,
        Key::Character("a".into()),
        &[KeyCode::SuperLeft],
    );
    assert_eq!(
        filter_reads(&app),
        "uo",
        "the command chord typed its own letter into the search"
    );
    chord(&mut app, KeyCode::Backspace, Key::Backspace, &[]);
    assert_eq!(
        filter_reads(&app),
        "",
        "select-all and one press empties it"
    );
}
