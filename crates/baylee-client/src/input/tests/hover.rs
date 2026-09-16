//! `Duel::hovered` has four writers and `pointer_hover` is only one of them, so every test here is about *whose* hover a value is and when it stops being true. The pointer's claim is about an entity and dies when that entity is despawned or when its card changes zone underneath it — the hand card that was played, the fetchland cracked into the graveyard — while a card that merely repacks, taps or lifts keeps it. The keyboard cursor's claim is a position in `cursor_grid` and survives both, which is why two tests assert opposite outcomes from the same event and neither is the permissive fallback of the other. Nothing here presses a key or puts a `PlayerAction` on the wire; the hover is the whole subject.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Playing the card under the pointer must take its preview with it.
///
/// The hand zone is rebuilt whole on every board change, so the node the
/// pointer was over is *despawned* — and Bevy fires no `Out` for an
/// entity that no longer exists. The card preview therefore stayed open
/// over the middle of the table until the player happened to hover
/// something else, which is how a screenshot of a live game found it.
///
/// The second half is the part a bare "does this object still exist"
/// check would fail: a land goes on playing under the same `ObjectId`,
/// now as a permanent, so the hover is only stale because it came from
/// the *hand*.
#[test]
fn a_hand_card_that_is_played_takes_its_hover_with_it() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, pointer_hover);

    let card = app
        .world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(3) })
        .id();
    hover(&mut app, card);
    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(3)),
        "the pointer over a hand card is a hover"
    );

    // The land is played: the hand zone is rebuilt without it, and the
    // same object arrives on the table. No `Out` is fired, and the
    // pointer does not move.
    app.world_mut().entity_mut(card).despawn();
    app.world_mut().spawn(crate::table::CardVisual {
        object: obj(3),
        count: 1,
    });
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        None,
        "a hand card that left the hand is not still hovered"
    );
}

/// The cure must not bring back the disease.
///
/// `Duel::hovered` has four writers and `pointer_hover` is only one of
/// them: the keyboard cursor writes it too, and the cursor walking off a
/// permanent and onto a hand card is exactly that. Held against the
/// *pointer's* last source, that write would be checked against the table,
/// not found there and cleared on the next frame — the stall of "the
/// pointer only speaks when it moves" back through a different door. So a
/// hover this system did not write is nobody's kind in particular.
#[test]
fn a_hover_this_system_did_not_write_is_left_alone() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, pointer_hover);

    let permanent = app
        .world_mut()
        .spawn(crate::table::CardVisual {
            object: obj(7),
            count: 1,
        })
        .id();
    app.world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(3) });
    hover(&mut app, permanent);
    assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(7)));

    // What `move_cursor` does when the keyboard walks onto the hand: it
    // writes the hover directly, and the pointer has not moved.
    app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(3));
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(3)),
        "the keyboard cursor survives a pointer that is resting elsewhere"
    );
}

/// The other half of `a_hand_card_that_is_played_takes_its_hover_with_it`.
///
/// The same event — the card under the hover is played, its hand node is
/// despawned and a permanent appears under the same `ObjectId` — and the
/// answer is the opposite one, because the two hovers are valid for
/// different reasons. The pointer's is over an entity that no longer
/// exists, so it goes. The keyboard's is a position in `cursor_grid`,
/// the card is still in that grid one row down, and taking it away would
/// send the player's next arrow key back to the start of their hand.
///
/// Written as a test rather than left to the comment because the union in
/// the `Elsewhere` arm reads like the permissive fallback of the other
/// two, and the next person to tighten it will have this fail.
#[test]
fn the_keyboard_cursor_follows_a_card_it_played_onto_the_table() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, pointer_hover);

    // A land in hand, with the keyboard cursor on it: written straight to
    // the resource, which is what `move_cursor` does.
    let in_hand = app
        .world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(5) })
        .id();
    app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(5));
    app.update();

    // It is played. The hand zone is rebuilt without it and the same
    // object is now a permanent.
    app.world_mut().entity_mut(in_hand).despawn();
    app.world_mut().spawn(crate::table::CardVisual {
        object: obj(5),
        count: 1,
    });
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(5)),
        "the cursor should follow the card it just played, not reset"
    );
}

/// A fetchland cracked under the pointer, which is the everyday way into
/// this and the way it was found.
///
/// The card is sacrificed, glides to the graveyard and becomes the top of
/// it — keeping the very entity it had on the battlefield, because
/// `SceneIndex::cards` reuses one entity per object. So it is still drawn
/// and still a `CardVisual`, the pointer has not moved so no `Out` is
/// fired, and the hover crossed the table with it. `the_click` answers a
/// hover before anything else, so the `Enter` meant for the search the
/// fetchland had just opened opened the graveyard instead.
#[test]
fn a_card_that_leaves_the_battlefield_leaves_the_pointer_behind() {
    use baylee_client_core::test_support::{ViewBuilder, printed};

    let on_the_field = ViewBuilder::new(2)
        .with_battlefield(0, [printed(9, 0, "Marsh Flats", 4)])
        .build();
    let (mut app, card) = hover_app(on_the_field);
    hover(&mut app, card);
    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(9)),
        "the pointer over a permanent is a hover"
    );

    // Cracked. The same entity is now the top of the graveyard, and
    // nothing else about the frame has changed.
    app.world_mut().resource_mut::<crate::Duel>().view = Some(
        ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(9, 0, "Marsh Flats", 4)])
            .build(),
    );
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        None,
        "the hover followed the card into the graveyard"
    );
}

/// The cure must not take the ordinary hover away.
///
/// Cards on this table move constantly — a lane repacks, a permanent taps,
/// a hovered card lifts — and the pointer is meant to keep its card
/// through all of it. Only a card that has changed *zone* has left the
/// place the pointer is making a claim about.
#[test]
fn a_permanent_that_only_moves_keeps_its_hover() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let alone = ViewBuilder::new(2)
        .with_battlefield(0, [printed(9, 0, "Birds of Paradise", 4)])
        .build();
    let (mut app, card) = hover_app(alone);
    hover(&mut app, card);
    assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(9)));

    // A second creature arrives, the lane repacks, and the hovered card
    // is drawn somewhere else entirely. It is still on the battlefield.
    app.world_mut().resource_mut::<crate::Duel>().view = Some(
        ViewBuilder::new(2)
            .with_battlefield(
                0,
                [
                    printed(9, 0, "Birds of Paradise", 4),
                    token(11, 0, "Saproling", 1, 1),
                ],
            )
            .build(),
    );
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(9)),
        "a repacked lane is not a card leaving the pointer"
    );
}
