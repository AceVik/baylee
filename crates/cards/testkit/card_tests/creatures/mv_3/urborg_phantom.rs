//! `cards/creatures/mv_3/urborg_phantom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn urborg_phantom() -> CardIndex {
    card_index("f8b5d2f4-58a8-408c-bb67-6b5f9ba5b678")
}

/// Urborg Phantom ({2}{B} 3/1 Spirit Minion):
/// "This creature can't block. {U}: Prevent all combat damage that would be
/// dealt to and dealt by this creature this turn."
#[test]
fn urborg_phantom_cannot_block_and_prevents_combat_damage_both_ways_but_not_spell_damage() {
    let mut e = lk::game(
        SEED,
        &[urborg_phantom(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[index::SHOCK],
        &[],
    );
    let phantom = lk::obj(&e, lk::P0, urborg_phantom());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    // P0 attacks with Phantom, P1 blocks with Bear (2/2); {U} prevents combat damage both ways.
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1)]);
    lk::use_ability(&mut e, lk::P0, urborg_phantom(), 1, &[], &[]);
    let _ = lk::attack(&mut e, &[phantom]);
    lk::block(&mut e, &[(bear, phantom)]);
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, urborg_phantom()).is_some());
    assert!(on_battlefield(&e, lk::P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::damage_on(&e, bear), 0);

    // Cannot block on P1's turn.
    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, phantom, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // Noncombat damage is NOT prevented even after paying {U}.
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1), (ManaColor::Red, 1)]);
    lk::use_ability(&mut e, lk::P0, urborg_phantom(), 1, &[], &[]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[phantom], &[]);
    assert!(in_graveyard(&e, lk::P0, urborg_phantom()).is_some());
}
