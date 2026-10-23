//! `cards/enchantments/mv_2/hesitation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
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
