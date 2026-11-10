//! `cards/artifacts/mv_4/weakstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Weakstone` is an artifact costing `{4}` under `Coverage::Implemented`.
/// It prints "Attacking creatures get -1/-0."
/// While on the battlefield, a creature that is declared as an attacker (such as `Desert Drake`)
/// has its power reduced by 1 (from 2/2 to 1/2), while a non-attacking creature is unaffected.
#[test]
fn weakstone_reduces_power_of_attacking_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[weakstone(), desert_drake()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let drake = on_battlefield(&engine, p0, desert_drake()).expect("Desert Drake is present");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls Llanowar Elves");
    assert_eq!(
        pt(&engine, drake),
        (2, 2),
        "un-attacking drake has full power"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "non-attacking elf has full power");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p1))],
            },
        )
        .unwrap();

    assert_eq!(
        pt(&engine, drake),
        (1, 2),
        "attacking creature gets -1/-0 from Weakstone"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "non-attacking creature remains untouched"
    );
}
