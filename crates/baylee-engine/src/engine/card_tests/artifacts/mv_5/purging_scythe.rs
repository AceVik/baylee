#[allow(clippy::wildcard_imports)]
use super::*;

fn purging_scythe() -> CardIndex {
    card_index("4410e3c4-b8c5-46d7-9418-7490bb69d8c4")
}

#[test]
fn purging_scythe_deals_two_damage_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[purging_scythe()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
}
