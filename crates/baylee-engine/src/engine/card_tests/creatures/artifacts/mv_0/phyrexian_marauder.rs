//! `cards/creatures/artifacts/mv_0/phyrexian_marauder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phyrexian_marauder() -> CardIndex {
    card_index("405674c5-607a-43d3-8b46-8741d2339f80")
}

/// `Phyrexian Marauder` ({X}, 0/0 Artifact Creature — Phyrexian Construct,
/// `Coverage::Partial`) enters with X +1/+1 counters on it and can't block.
#[test]
fn phyrexian_marauder_enters_with_x_p1p1_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[phyrexian_marauder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, phyrexian_marauder());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let marauder = on_battlefield(&engine, p0, phyrexian_marauder()).unwrap();
    assert_eq!(counters_on(&engine, marauder, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, marauder), (3, 3));
}
