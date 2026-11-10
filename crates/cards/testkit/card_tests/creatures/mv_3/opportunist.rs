//! `cards/creatures/mv_3/opportunist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn opportunist() -> CardIndex {
    card_index("5b5524f7-3c68-4c7b-be1b-745ffcad8006")
}

/// Opportunist (`5b5524f7-3c68-4c7b-be1b-745ffcad8006`) — {2}{R} Creature — Human Soldier:
/// "{T}: This creature deals 1 damage to target creature that was dealt damage this turn."
#[test]
fn opportunist_deals_one_damage_to_damaged_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[opportunist(), index::PRODIGAL_SORCERER])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    let source = on_battlefield(&engine, P0, opportunist()).unwrap();
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == source),
        "no damaged creature means no legal activation"
    );
    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, P0, opportunist(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
