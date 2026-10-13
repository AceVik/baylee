//! The keys of a payment window (CR 605.3a), whose taps are staged on this
//! side of the wire until Go: `Space` stages the plan and then sends it
//! whole, and `Esc` takes the staged taps back before it takes the cast
//! back. Each test drives the real answering system with a `Fired` built
//! from a real chord, over a duel that went in through the host's edges.

#[allow(clippy::wildcard_imports)] // the input tests' shared fixtures
use super::*;
use crate::keys::Fired;
use crate::owed_tests::seat_with_two_forests;
use baylee_client_core::manaplan::{Step, Tap};
use baylee_core::mana::ManaCost;

/// Presses `key` once through `answer_the_question`.
fn pressed(duel: &mut crate::Duel, key: bevy::prelude::KeyCode) {
    let keymap = baylee_client_core::prefs::Keymap::standard();
    let keys = press(key);
    let mut prefs = crate::prefs::Prefs::default();
    answer_the_question(Fired::of(&keys, &keymap), duel, &mut prefs);
}

/// A window owing `{1}` over two Forests, one of them staged.
fn with_a_forest_staged() -> (crate::Duel, ObjectId) {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let forest = duel
        .view
        .as_ref()
        .expect("the view")
        .battlefield
        .first()
        .expect("a Forest")
        .id;
    assert!(duel.stage(Step {
        source: forest,
        tap: Tap::Intrinsic,
        color: None,
    }));
    (duel, forest)
}

/// `Esc` in a window with something staged takes every staged tap back and
/// sends nothing: the cheapest thing on the table to undo is undone first.
#[test]
fn escape_unstages_everything_and_sends_nothing() {
    let (mut duel, _) = with_a_forest_staged();
    assert!(!duel.pay_staged.is_empty());
    pressed(&mut duel, bevy::prelude::KeyCode::Escape);
    assert!(duel.pay_staged.is_empty(), "the staging was emptied");
    assert!(duel.outbox().is_empty(), "and nothing reached the engine");
    assert!(duel.paying(), "the window itself is still open");
}

/// The counter-test: with nothing staged and no cast to take back, `Esc`
/// in a window sends nothing at all (a stray press must not pass the
/// window unpaid or decline it).
#[test]
fn escape_with_nothing_staged_and_no_cast_sends_nothing() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(duel.pay_staged.is_empty());
    pressed(&mut duel, bevy::prelude::KeyCode::Escape);
    assert!(duel.outbox().is_empty(), "sent: {:?}", duel.outbox());
    assert!(duel.paying());
}

/// The ladder: in a payment window that is a cast's own, the first `Esc`
/// takes the staged taps back and only the next one takes the cast back
/// (CR 732). A player who meant to untap a land must not lose the spell.
#[test]
fn escape_unstages_before_it_takes_the_cast_back() {
    let (mut duel, _) = with_a_forest_staged();
    let mut view = duel.view.clone().expect("the view");
    view.casting = Some(obj(90));
    duel.receive_view(view);
    assert!(
        duel.cancellable_cast().is_some(),
        "the fixture is a cast's window"
    );
    assert!(!duel.pay_staged.is_empty(), "the staging survived the view");

    pressed(&mut duel, bevy::prelude::KeyCode::Escape);
    assert!(duel.pay_staged.is_empty());
    assert!(
        duel.outbox().is_empty(),
        "the first Esc took back only the taps: {:?}",
        duel.outbox()
    );

    pressed(&mut duel, bevy::prelude::KeyCode::Escape);
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::CancelCast],
        "the second Esc takes the cast back"
    );
}

/// `Space` in a window owing `{1}` first stages the plan and sends nothing;
/// the second press is Go and sends exactly one `PayWindow` with the staged
/// tap in it. Two presses, one action on the wire.
#[test]
fn space_stages_first_and_the_second_press_sends_one_pay_window() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));

    pressed(&mut duel, bevy::prelude::KeyCode::Space);
    assert!(duel.outbox().is_empty(), "the first press sends nothing");
    assert_eq!(duel.pay_staged.steps().len(), 1, "it staged the plan");
    let forest = duel.pay_staged.steps()[0].source;

    pressed(&mut duel, bevy::prelude::KeyCode::Space);
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: forest }],
        }],
        "the second press is Go: one action, the staged tap"
    );
}

/// A tap staged by hand counts: with the Forest the player chose staged,
/// the first `Space` is already Go, and it sends that Forest, not the
/// planner's pick.
#[test]
fn space_with_the_cost_covered_by_hand_goes_at_once() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let second = duel.view.as_ref().expect("the view").battlefield[1].id;
    assert!(duel.stage(Step {
        source: second,
        tap: Tap::Intrinsic,
        color: None,
    }));
    pressed(&mut duel, bevy::prelude::KeyCode::Space);
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: second }],
        }]
    );
}
