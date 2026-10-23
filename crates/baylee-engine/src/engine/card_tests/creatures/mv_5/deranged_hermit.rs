#[allow(clippy::wildcard_imports)]
use super::*;

fn deranged_hermit() -> CardIndex {
    card_index("b9cd714b-2ad8-4fdb-a8aa-82b17730e071")
}

#[test]
fn deranged_hermit_creates_four_squirrels_and_pumps_them() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[deranged_hermit()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, deranged_hermit());
    pass_until(&mut engine, stack_is_empty);

    let squirrels: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(squirrels.len(), 4);
    for sq in squirrels {
        assert_eq!(pt(&engine, sq), (2, 2));
    }
}
