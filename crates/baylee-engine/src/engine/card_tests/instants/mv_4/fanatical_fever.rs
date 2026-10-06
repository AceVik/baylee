//! `cards/instants/mv_4/fanatical_fever.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fanatical Fever` is an instant costing `{2}{G}{G}` under `Coverage::Implemented`.
/// It prints "Target creature gets +3/+0 and gains trample until end of turn."
/// When cast from hand off four Forests targeting `Llanowar Elves`, the target gets +3/+0
/// (growing from 1/1 to 4/1) and gains trample until end of turn.
#[test]
fn fanatical_fever_pumps_power_and_grants_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[fanatical_fever()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "base body is 1/1");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "target does not have trample initially"
    );

    cast_from_hand(&mut engine, p0, fanatical_fever());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Fanatical Fever, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Llanowar Elves is offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (4, 1),
        "creature gets +3/+0 until end of turn"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "creature gains trample until end of turn"
    );
    assert!(
        in_graveyard(&engine, p0, fanatical_fever()).is_some(),
        "Fanatical Fever resolves to the graveyard"
    );
}
