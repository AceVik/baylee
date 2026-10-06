//! `cards/creatures/mv_3/rishkar_peema_renegade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rishkar, Peema Renegade` prints `When Rishkar enters, put a +1/+1 counter on each of up to two target creatures.` and `Each creature you control with a counter on it has "{{T}}: Add {{G}}."`
///
/// Marked `Coverage::Partial`, its arrival trigger fires `Trigger::ETB` placing a `CounterKind::P1P1` on up to two targets chosen through `Pending::ChooseTargets`.
/// The creatures each grow by +1/+1, while the unsupported static granting `{{T}}: Add {{G}}` to creatures with counters is omitted from `LegalActions::mana_abilities`.
#[test]
fn rishkar_peema_renegade_distributes_counters_and_omits_mana_grant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), young_wolf()])
        .hand(0, &[rishkar_peema_renegade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rishkar_peema_renegade());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 2), "up to two target creatures");

    let rishkar =
        on_battlefield(&engine, p0, rishkar_peema_renegade()).expect("rishkar on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");
    assert!(options.contains(&rishkar));
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![rishkar, wolf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, rishkar, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&engine, wolf, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, rishkar), (3, 3));
    assert_eq!(pt(&engine, wolf), (2, 2));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&rishkar),
        "under `Coverage::Partial` Rishkar is not granted a mana ability"
    );
    assert!(
        !legal.mana_abilities.contains(&wolf),
        "under `Coverage::Partial` wolf is not granted a mana ability"
    );
}
