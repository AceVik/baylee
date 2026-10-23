#[allow(clippy::wildcard_imports)]
use super::*;

fn aura_flux() -> CardIndex {
    card_index("128304af-4555-4276-bf26-638017beb3ff")
}

#[test]
fn aura_flux_grants_upkeep_tax_to_other_enchantments() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[aura_flux()])
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[crusade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aura_flux());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(on_battlefield(&engine, p1, crusade()).is_none());
    assert!(in_graveyard(&engine, p1, crusade()).is_some());
    assert!(on_battlefield(&engine, p0, aura_flux()).is_some());
}
