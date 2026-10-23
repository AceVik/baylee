use super::*;

fn penance() -> CardIndex {
    card_index("7818d6e0-74d8-46ae-94f6-d2a349c9507b")
}

#[test]
fn penance_resolves_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[penance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, penance());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, penance()).is_some());
}
