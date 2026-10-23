//! `cards/enchantments/mv_2/amok.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn amok() -> CardIndex {
    card_index("2ddbbe65-f928-4b8e-8c4c-a9ce82e2594d")
}

/// Amok (`2ddbbe65-f928-4b8e-8c4c-a9ce82e2594d`) — {1}{R} Enchantment (Implemented):
/// "{1}, Discard a card at random: Put a +1/+1 counter on target creature."
#[test]
fn amok_discards_at_random_and_adds_plus_one_counter() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[amok(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, amok());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));

    activate(&mut engine, P0, amok(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
    assert!(in_graveyard(&engine, P0, forest()).is_some());
}

#[test]
fn cannot_activate_without_a_card_to_discard_even_with_mana() {
    let mut e = lk::game(SEED, &[amok(), index::GRIZZLY_BEARS], &[], &[], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    assert!(!lk::offered(&e, amok(), P0, 0));
}
