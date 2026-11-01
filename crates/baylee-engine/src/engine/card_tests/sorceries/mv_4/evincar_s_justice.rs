//! `cards/sorceries/mv_4/evincar_s_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn evincar_s_justice() -> CardIndex {
    card_index("c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05")
}

/// Evincar's Justice (`c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05`) — {2}{B}{B} Sorcery (Partial):
/// "Buyback {3}. Evincar's Justice deals 2 damage to each creature and each player."
#[test]
fn evincar_s_justice_deals_two_damage_to_each_creature_and_each_player() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), index::HILL_GIANT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[evincar_s_justice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, P0, evincar_s_justice());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(
        crate::engine::card_tests::legends_kit::damage_on(&engine, giant),
        2
    );
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, P0, evincar_s_justice()).is_some());
}
