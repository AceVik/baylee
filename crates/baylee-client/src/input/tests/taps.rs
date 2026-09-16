//! One finger, one action. Bevy raises a `Pointer<Click>` only when the press and the release land on the same entity, and this client despawns the node under the finger on every hover change and every arriving view, so reconciling press, release and click is the client's own work — these are its four cases: a tap across a rebuild, a press that drifts between one card's children, a tap the tree did hear (sent once, not twice), and a press dragged onto another card (sent not at all). Everything runs `touch::watch_the_finger` and `pointer` against hand-written `Pointer<Press>`/`Release`/`Click` messages, and the two counter-tests are what make the fix worth anything — a flag raised by every release would turn each of them into a second land. Which `PlayerAction` a completed tap becomes is `arming`'s question, and the hover the same pointer leaves behind is `hover`'s.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A tap the tree ate still plays the card.
///
/// `docs/observed-faults.md` 35. Bevy raises a `Pointer<Click>` only when
/// the press and the release land on the same **entity**, and the hand
/// row is rebuilt on every hover change and on every arriving view — so
/// a finger that is down across one of those comes up on a node born
/// after the press and no click is ever raised. The card sank, came back
/// and played nothing.
#[test]
fn a_tap_that_spans_a_rebuild_still_plays_the_card() {
    let mut app = hand_app();
    let before = row_card(&mut app, obj(3));
    finger_down(&mut app, before);
    app.update();

    // The rebuild: the node the finger went down on is despawned and the
    // same card comes back as a different entity.
    app.world_mut().entity_mut(before).despawn();
    let after = row_card(&mut app, obj(3));
    finger_up(&mut app, after);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "the tap reached the card it was made on"
    );
}

/// The other half of the fault text, and no rebuild in it at all.
///
/// A card's art, its text and its rail are separate pickable children, so
/// a press that drifts across that seam is two entities and bevy raises
/// no click either. It rides on the same lineage walk a click does, which
/// is why one fix covers both.
#[test]
fn a_press_that_drifts_across_one_card_is_still_a_tap() {
    use bevy::prelude::*;

    let mut app = hand_app();
    let card = row_card(&mut app, obj(3));
    let art = app.world_mut().spawn(ChildOf(card)).id();
    let text = app.world_mut().spawn(ChildOf(card)).id();

    finger_down(&mut app, art);
    finger_up(&mut app, text);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "the art and the text are one card"
    );
}

/// A tap the tree *did* hear is sent once, not twice.
///
/// The counter-test the fix above is worth nothing without: the flag is
/// raised by every release over the card the finger is on, including the
/// ordinary ones, and is meant to be taken down again by the click that
/// answers them. Read before the clicks instead of after, this plays the
/// land and then plays it again.
#[test]
fn a_tap_the_tree_heard_is_sent_once() {
    let mut app = hand_app();
    let card = row_card(&mut app, obj(3));
    finger_down(&mut app, card);
    app.update();

    // Press and release on the same entity, so bevy raises the click too
    // — all three on the frame the release lands, as they arrive live.
    finger_up(&mut app, card);
    finger_click(&mut app, card);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "one tap is one land"
    );
}

/// A press dragged off its card and let go over another one asks nothing.
///
/// The second counter-test: a flag raised on every release, rather than
/// only on a release over the card the finger went down on, would turn
/// every dragged-off press into a tap on whatever it started on.
#[test]
fn a_press_let_go_over_another_card_sends_nothing() {
    let mut app = hand_app();
    let land = row_card(&mut app, obj(3));
    let other = row_card(&mut app, obj(5));
    finger_down(&mut app, land);
    app.update();
    finger_up(&mut app, other);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [],
        "a press that moved on is not a tap"
    );
}
