//! `cards/creatures/mv_4/nantuko_disciple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Nantuko Disciple` is a 2/2 creature costing `{3}{G}` under `Coverage::Implemented`.
/// It prints "{G}, {T}: Target creature gets +2/+2 until end of turn."
/// When activated off a Forest targeting `Llanowar Elves`, it taps and pumps
/// the target creature from 1/1 to 3/3 until end of turn.
#[test]
fn nantuko_disciple_pumps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nantuko_disciple(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let disciple = on_battlefield(&engine, p0, nantuko_disciple())
        .expect("Nantuko Disciple is on the battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "base body is 1/1");

    tap_all_mana_but(&mut engine, p0, Some(nantuko_disciple()));
    activate(&mut engine, p0, nantuko_disciple(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Nantuko Disciple, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Llanowar Elves is an offered target: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "target creature gets +2/+2 until end of turn"
    );
    assert!(
        is_tapped(&engine, disciple),
        "Nantuko Disciple tapped to pay its activation cost"
    );
}
