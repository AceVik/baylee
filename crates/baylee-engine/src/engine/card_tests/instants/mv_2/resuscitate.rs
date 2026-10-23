use super::*;

fn resuscitate() -> CardIndex {
    card_index("4390e251-e154-4ef8-9929-0c78aa2c8f10")
}

#[test]
fn resuscitate_casts_and_resolves_to_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[resuscitate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, resuscitate());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, resuscitate()).is_some());
}
