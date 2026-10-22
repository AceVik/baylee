//! `cards/enchantments/mv_5/bazaar_of_wonders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::BAZAAR_OF_WONDERS;

#[test]
fn bazaar_of_wonders_exiles_all_graveyards_on_etb() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[BAZAAR_OF_WONDERS])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let g0 = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let g1 = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];

    cast_from_hand(&mut engine, p0, BAZAAR_OF_WONDERS);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(g0).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(g1).unwrap().zone, Zone::Exile);
}
