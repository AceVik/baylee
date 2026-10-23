//! `cards/instants/mv_1/culling_the_weak.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn culling_the_weak() -> CardIndex {
    card_index("6385fd0d-256d-4400-9319-afa8170024db")
}

/// Culling the Weak (`6385fd0d-256d-4400-9319-afa8170024db`) — {B} Instant (Implemented):
/// "As an additional cost to cast this spell, sacrifice a creature. Add {B}{B}{B}{B}."
#[test]
fn culling_the_weak_sacrifices_creature_and_adds_four_black_mana() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[culling_the_weak()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, culling_the_weak());
    kit::settle(&mut engine, P0, |_, p| kit::choose_cards(p, &[bear]));

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
}

#[test]
fn culling_the_weak_cannot_sacrifice_an_opponents_creature() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[culling_the_weak()],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 10)]);
    let card = in_hand(&e, P0, culling_the_weak()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&card) && !legal.payable.contains(&card));
}
