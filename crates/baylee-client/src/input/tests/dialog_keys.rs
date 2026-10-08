//! Which surface a keystroke belongs to while a sheet stands over the table. Space means both "I am done here" and "pass priority", and a `ChooseCards { min: 0 }` arriving mid-rhythm was answered with nothing found, no trace and no undo — so in this dialog the key ticks the focused row and the same press takes it back, while with no dialog up it is a pass again. Enter is the mirror: `the_click` answers the card under the pointer first, and a card under the pointer is the ordinary state of a table with a sheet over it, so the dialog has to take the frame before the dispatch ever reaches the table. Every test drives `browser_answer_keys` or `the_click` with a `Fired` built from a real chord, and each counter-test is what stops "the key now does nothing" from passing; typing into the search box itself is `tray`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// AE6, and the whole reason the dialog has a keyboard of its own.
///
/// The confirm key is Space and means two things — "I am done here" and
/// "pass priority". Three triggers go on the stack, the player passes
/// through them, and a `ChooseCards { min: 0 }` arrives mid-rhythm: the
/// next press answered it with "nothing found", with no trace and no
/// undo, and the land the search was for never entered play.
///
/// `docs/redesign-proposal.md` §6 says Space toggles in this dialog, and
/// that is also the fix: a stray press now does something visible that
/// the same key takes back.
#[test]
fn the_confirm_key_cannot_throw_a_search_away() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = duel_searching(0);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);

    assert!(browser_answer_keys(Fired::of(&keys, &keymap), &mut duel));
    assert!(
        duel.outbox().is_empty(),
        "the search was answered with nothing"
    );
    let it = duel.interaction.as_ref().expect("the question stands");
    assert!(it.is_selected(obj(1)), "the focused row was ticked instead");
    // And the same key again takes it back, which is what makes the
    // stray press harmless rather than merely slower.
    assert!(browser_answer_keys(Fired::of(&keys, &keymap), &mut duel));
    let it = duel
        .interaction
        .as_ref()
        .expect("the question still stands");
    assert!(!it.is_selected(obj(1)));
    assert!(duel.outbox().is_empty());
}

/// The counter-test, because "Space does nothing now" would pass the one
/// above: with no dialog holding the question the key is a pass again.
#[test]
fn the_confirm_key_still_passes_priority_with_no_dialog_up() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = window_with(vec![], vec![]);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);
    let fired = Fired::of(&keys, &keymap);

    assert!(
        !browser_answer_keys(fired, &mut duel),
        "no dialog, so the dialog's keyboard declines the frame"
    );
    let mut prefs = crate::prefs::Prefs::default();
    answer_the_question(fired, &mut duel, &mut prefs);
    assert_eq!(duel.outbox(), &[PlayerAction::PassPriority]);
}

/// And a search that *must* take a card is no different: the key was
/// never able to answer that one, and it still ticks rather than
/// reaching past the dialog to a confirm that would be refused.
#[test]
fn a_search_with_a_minimum_is_ticked_by_the_same_key() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = duel_searching(1);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);

    assert!(browser_answer_keys(Fired::of(&keys, &keymap), &mut duel));
    let it = duel.interaction.as_ref().expect("the question stands");
    assert!(it.is_selected(obj(1)));
    assert!(duel.outbox().is_empty(), "ticking is not sending");
}

/// §6 gives the dialog Enter, and the table kept taking it.
///
/// `the_click` answers the card under the pointer before anything else,
/// and a card under the pointer is the ordinary state of a table with a
/// sheet standing over it — the permanent whose ability asked the
/// question is usually the very card the pointer is resting on. So the
/// one key the dialog needs was the one key it was least likely to get,
/// and the press went to the table instead, silently.
///
/// The second half of the test is what says the precedence matters: the
/// same press, on the same duel, is taken by `the_click` and spent on a
/// card that is not even part of the question.
#[test]
fn the_dialog_answers_enter_rather_than_the_card_under_the_pointer() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let keymap = Keymap::standard();
    let space = press(bevy::prelude::KeyCode::Space);
    let enter = press(bevy::prelude::KeyCode::Enter);

    let mut duel = duel_searching(1);
    // A row ticked, and the pointer left on something else entirely —
    // the fetchland that asked the question, lying in the graveyard.
    browser_answer_keys(Fired::of(&space, &keymap), &mut duel);
    duel.hovered = Some(obj(7));

    assert!(
        browser_answer_keys(Fired::of(&enter, &keymap), &mut duel),
        "the dialog takes the frame, so the dispatch never reaches the table"
    );
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }],
        "Enter sent the answer the player had built"
    );

    // And what that precedence is holding back.
    let mut table = duel_searching(1);
    table.hovered = Some(obj(7));
    let mut prefs = crate::prefs::Prefs::default();
    assert!(
        the_click(Fired::of(&enter, &keymap), &mut table, &mut prefs),
        "the hovered card would have eaten the key"
    );
    assert!(
        table.outbox().is_empty(),
        "…and answered nothing with it, which is how the press vanished"
    );
}

/// While an arrangement holds a card the cursor keys move it and reach
/// nothing else; with nothing held they are the table's again, and the
/// counter-case is what stops "the arrows are swallowed now" from passing.
#[test]
fn the_cursor_keys_move_a_held_card_and_nothing_else() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = duel_arranging();
    let keymap = Keymap::standard();
    let right = press(bevy::prelude::KeyCode::KeyD);

    assert!(
        !arrange_keys(Fired::of(&right, &keymap), &mut duel),
        "nothing held, so the cursor keys are the table's"
    );
    let space = press(bevy::prelude::KeyCode::Space);
    assert!(browser_answer_keys(Fired::of(&space, &keymap), &mut duel));
    let it = duel.interaction.as_ref().expect("the question stands");
    assert!(it.is_selected(obj(1)), "the tick took the focused card up");

    assert!(arrange_keys(Fired::of(&right, &keymap), &mut duel));
    let it = duel.interaction.as_ref().expect("the question stands");
    assert_eq!(
        it.confirm(),
        Some(PlayerAction::Arrange {
            piles: vec![vec![obj(2), obj(1), obj(3)]]
        }),
        "one place further from the top"
    );
    assert!(it.is_selected(obj(1)), "and still held for the next move");
    assert!(duel.outbox().is_empty(), "moving is not sending");
}

/// A search sheet a question opened, its filter box holding the keyboard
/// (as `browser_takes_the_keyboard` leaves it), in an app with the table's
/// whole keyboard system.
fn a_search_sheet_typing() -> (bevy::prelude::App, bevy::prelude::Entity) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;
    let mut app = App::new();
    let mut duel = duel_searching(1);
    duel.browser.start_typing();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<Keystrokes>()
        .insert_resource(duel)
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(Update, keyboard);
    let window = app.world_mut().spawn_empty().id();
    app.update();
    (app, window)
}

fn a_key(
    app: &mut bevy::prelude::App,
    window: bevy::prelude::Entity,
    code: bevy::prelude::KeyCode,
    logical: bevy::input::keyboard::Key,
    text: Option<&str>,
) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.world_mut()
        .resource_mut::<Keystrokes>()
        .0
        .push(KeyboardInput {
            key_code: code,
            logical_key: logical,
            state: bevy::input::ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

/// The library search a tutor opened (beta.6 QA): its filter box held the
/// keyboard, so Space typed a blank into an empty filter and Enter only let
/// go of the box — the two keys the dialog answers with did nothing. Into
/// an empty box Space ticks the row and Enter sends; a letter still types.
#[test]
fn space_and_enter_answer_a_search_whose_filter_box_is_empty() {
    use bevy::input::keyboard::Key;
    use bevy::prelude::KeyCode;
    let (mut app, window) = a_search_sheet_typing();
    a_key(&mut app, window, KeyCode::Space, Key::Space, Some(" "));
    {
        let duel = app.world().resource::<crate::Duel>();
        assert_eq!(duel.browser.filter(), "", "a blank typed into the box");
        let it = duel.interaction.as_ref().expect("the question stands");
        assert!(it.is_selected(obj(1)), "Space ticked the focused row");
    }
    a_key(&mut app, window, KeyCode::Enter, Key::Enter, None);
    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        &[PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }],
        "Enter sent the tick"
    );
}

/// The counter-test: with words in the box the keys are the box's.
#[test]
fn a_letter_and_a_blank_still_type_into_a_search_with_words() {
    use bevy::input::keyboard::Key;
    use bevy::prelude::KeyCode;
    let (mut app, window) = a_search_sheet_typing();
    a_key(
        &mut app,
        window,
        KeyCode::KeyA,
        Key::Character("a".into()),
        Some("a"),
    );
    a_key(&mut app, window, KeyCode::Space, Key::Space, Some(" "));
    a_key(
        &mut app,
        window,
        KeyCode::KeyB,
        Key::Character("b".into()),
        Some("b"),
    );
    let duel = app.world().resource::<crate::Duel>();
    assert_eq!(duel.browser.filter(), "a b");
    assert!(duel.outbox().is_empty());
}
