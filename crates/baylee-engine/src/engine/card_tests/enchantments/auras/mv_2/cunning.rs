//! `cards/enchantments/auras/mv_2/cunning.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cunning() -> CardIndex {
    card_index("d5e48041-5f21-48e6-bd0c-000f02dfbe8a")
}

/// Cunning (`d5e48041-5f21-48e6-bd0c-000f02dfbe8a`) — {1}{U} Enchantment — Aura (Implemented):
/// "Enchant creature. Enchanted creature gets +3/+3. When enchanted creature attacks or blocks, sacrifice this Aura at the beginning of the next cleanup step."
#[test]
fn cunning_grants_plus_3_plus_3_and_sacrifices_itself_at_next_cleanup_when_enchanted_creature_attacks()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), index::GRIZZLY_BEARS])
        .hand(0, &[cunning()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, cunning());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (5, 5));

    kit::declare_attack(&mut engine, &[bear], P1);
    pass_until(&mut engine, |e| e.state().turn.active == P1);

    assert!(on_battlefield(&engine, P0, cunning()).is_none());
    assert!(in_graveyard(&engine, P0, cunning()).is_some());
}
