//! "You may choose a nonland card from it. If you do, that player reveals
//! the chosen card, puts it on the bottom of their library, then draws a
//! card" (Vendilion Clique): everything after "if you do" happens only if
//! the choice was made. The instructions are followed in the order written
//! (CR 608.2c), and the ones the sentence makes conditional are skipped
//! with the choice declined or impossible.
//!
//! `Effect::BottomCardFromHand` carries what follows as `then`, run only
//! once a card has gone to the bottom. The draw used to be a sibling
//! instruction, and a target player whose hand held nothing worth taking —
//! or whose card the Clique's controller let them keep — drew a card for
//! free.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, keep_mulligans, pass_until, stack_is_empty,
    walk_to_own_main,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}
fn vendilion_clique() -> CardIndex {
    card_index("244d4807-0802-41bc-9460-55ac38a28a72")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// What seat 1 holds in hand and library, in that order.
fn sizes(engine: &Engine<RegistryLookup>) -> (usize, usize) {
    let zones = &engine.state().zones;
    (
        zones.list(ZoneLocation::Hand(P1)).len(),
        zones.list(ZoneLocation::Library(P1)).len(),
    )
}

/// Seat 0 casts Vendilion Clique at seat 1, whose hand is `theirs`, and
/// the trigger has its target. Returns seat 1's hand and library sizes as
/// the trigger goes on the stack.
fn clique_at(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, (usize, usize)) {
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique()])
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let before = sizes(&engine);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    (engine, before)
}

/// The Clique's controller declines to choose: seat 1 keeps its hand and
/// draws nothing.
#[test]
fn a_declined_choice_draws_nothing() {
    let (mut engine, before) = clique_at(&[counterspell(), counterspell()]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(sizes(&engine), before, "nothing bottomed, nothing drawn");
}

/// A hand with nothing the Clique may choose (lands only): nobody is asked,
/// and seat 1 draws nothing.
#[test]
fn an_impossible_choice_draws_nothing() {
    let (mut engine, before) = clique_at(&[island(), island()]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(sizes(&engine), before, "nothing bottomed, nothing drawn");
}

/// The choice made: one card goes under the library and one is drawn, so
/// both sizes are what they were.
#[test]
fn a_chosen_card_is_bottomed_and_replaced() {
    let (mut engine, before) = clique_at(&[counterspell(), counterspell()]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(sizes(&engine), before, "bottomed one, drew one");
}
