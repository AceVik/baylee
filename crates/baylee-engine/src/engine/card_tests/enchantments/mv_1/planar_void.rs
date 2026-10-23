#[allow(clippy::wildcard_imports)]
use super::*;

fn planar_void() -> CardIndex {
    card_index("994e27e1-0bff-47c9-a1e6-b9e3ae4ffb1e")
}

#[test]
fn planar_void_exiles_cards_put_into_opponents_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[planar_void(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bear_id);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        crate::zone::Zone::Exile
    );
}
