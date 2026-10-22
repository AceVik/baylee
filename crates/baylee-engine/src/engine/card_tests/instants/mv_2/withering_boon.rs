//! `cards/instants/mv_2/withering_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn withering_boon() -> CardIndex {
    card_index("6d87415b-0618-48e1-9510-320c4611ecda")
}

/// Withering Boon (`{1}{B}`, `Coverage::Implemented`): Instant that costs an
/// additional 3 life to cast and counters target creature spell.
#[test]
fn withering_boon_pays_3_life_and_counters_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[withering_boon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grizzly_bears());
    let bear_spell = on_stack(&engine, grizzly_bears()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, withering_boon());
    aim_at(&mut engine, p1, bear_spell);
    assert_eq!(engine.state().players[1].life, 17);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
