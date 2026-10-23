#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_war_machine() -> CardIndex {
    card_index("b3b8fe34-4f69-4555-bcac-04563d02c99f")
}

#[test]
fn thran_war_machine_casts_and_has_stats() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[thran_war_machine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thran_war_machine());
    pass_until(&mut engine, stack_is_empty);

    let machine = on_battlefield(&engine, p0, thran_war_machine()).unwrap();
    assert_eq!(pt(&engine, machine), (4, 5));
}
