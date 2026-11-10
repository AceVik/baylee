//! `cards/creatures/mv_3/daring_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn daring_apprentice() -> CardIndex {
    card_index("863ad4b8-094b-46b3-b6cd-0640e1f01d6d")
}

/// Daring Apprentice (863ad4b8-094b-46b3-b6cd-0640e1f01d6d) — {1}{U}{U} 1/1:
/// "{T}, Sacrifice this creature: Counter target spell."
#[test]
fn daring_apprentice_taps_and_sacrifices_to_counter_target_spell() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[daring_apprentice()])
        .battlefield(1, &[mountain(), mountain(), mountain(), mountain()])
        .hand(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, HILL_GIANT);
    let spell_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();

    // Pass priority to P0 so P0 can counter Hill Giant.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, P0, daring_apprentice(), 0);
    aim_at(&mut engine, P0, spell_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, daring_apprentice()).is_none());
    assert!(in_graveyard(&engine, P0, daring_apprentice()).is_some());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, HILL_GIANT).is_some());
}
