//! `cards/creatures/artifacts/mv_4/machine_man_model_x_51.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn machine_man() -> CardIndex {
    card_index("853587be-9709-43a4-81c9-5b6dabe523cf")
}

/// Machine Man, Model X-51 — {4} 2/3: "Whenever you cast a noncreature
/// spell, put a +1/+1 counter on Machine Man and he gains flying until end
/// of turn." A creature spell does nothing to him; a Lightning Bolt gives him
/// the counter (3/4) and flying, and flying ends with the turn while the
/// counter stays.
#[test]
fn machine_man_grows_and_flies_on_a_noncreature_spell_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[machine_man(), mountain(), forest(), forest()])
        .hand(0, &[grizzly_bears(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let man = on_battlefield(&engine, p0, machine_man()).expect("seated");
    assert_eq!(pt(&engine, man), (2, 3));

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, man, CounterKind::P1P1),
        0,
        "a creature"
    );
    assert_eq!(pt(&engine, man), (2, 3));
    assert!(!keywords(&engine, man).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17, "the Bolt resolved");
    assert_eq!(counters_on(&engine, man, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, man), (3, 4));
    assert!(keywords(&engine, man).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(counters_on(&engine, man, CounterKind::P1P1), 1, "it stays");
    assert!(
        !keywords(&engine, man).contains(KeywordSet::FLYING),
        "flying lasted until end of turn"
    );
}
