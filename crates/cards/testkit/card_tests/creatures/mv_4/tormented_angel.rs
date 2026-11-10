//! `cards/creatures/mv_4/tormented_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tormented Angel` is a creature costing `{3}{W}` under `Coverage::Implemented`.
/// It prints a 1/5 Angel body with flying.
/// When cast from hand off four Plains, it resolves and enters the battlefield
/// with 1/5 power and toughness and the flying keyword.
#[test]
fn tormented_angel_enters_as_a_flying_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[tormented_angel()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, tormented_angel());
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, p0, tormented_angel())
        .expect("Tormented Angel resolved and is on the battlefield");
    assert_eq!(pt(&engine, angel), (1, 5), "printed power/toughness is 1/5");
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "Tormented Angel has flying"
    );
}
