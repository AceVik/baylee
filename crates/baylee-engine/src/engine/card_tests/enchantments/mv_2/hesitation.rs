//! `cards/enchantments/mv_2/hesitation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn hesitation() -> CardIndex {
    card_index("11eb31d9-cbcb-4ec5-abab-dfa646addbba")
}

/// Hesitation (`11eb31d9-cbcb-4ec5-abab-dfa646addbba`) — {1}{U} Enchantment (Implemented):
/// "When a player casts a spell, sacrifice this enchantment and counter that spell."
#[test]
fn hesitation_sacrifices_itself_and_counters_a_cast_spell() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hesitation(), forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, hesitation()).is_none());
    assert!(in_graveyard(&engine, P0, hesitation()).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn opponent_spell_is_countered_but_land_play_does_not_use_hesitation() {
    let mut e = lk::game(
        SEED,
        &[hesitation()],
        &[],
        &[forest()],
        &[index::GRIZZLY_BEARS],
    );
    play_land(&mut e, P0, forest());
    lk::settle(&mut e);
    assert!(on_battlefield(&e, P0, hesitation()).is_some());
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, lk::P1, index::GRIZZLY_BEARS);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, P0, hesitation()).is_some());
}
