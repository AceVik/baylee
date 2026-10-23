#[allow(clippy::wildcard_imports)]
use super::*;

fn pendrell_flux() -> CardIndex {
    card_index("fe902db2-3b1d-4266-88eb-307f878fc317")
}

#[test]
fn pendrell_flux_grants_upkeep_sacrifice_unless_pay_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[pendrell_flux()])
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, pendrell_flux());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, opp_bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
}
