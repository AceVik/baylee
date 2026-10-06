//! `cards/creatures/mv_6/moss_kami.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Moss Kami` is a 5/5 creature costing `{5}{G}` under `Coverage::Implemented`.
/// It prints the trample keyword.
/// When cast from hand off six Forests, it resolves and enters the battlefield
/// with 5/5 power and toughness and the trample keyword.
#[test]
fn moss_kami_enters_as_a_creature_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[moss_kami()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, moss_kami());
    pass_until(&mut engine, stack_is_empty);

    let kami = on_battlefield(&engine, p0, moss_kami())
        .expect("Moss Kami resolved and is on the battlefield");
    assert_eq!(pt(&engine, kami), (5, 5), "printed power/toughness is 5/5");
    assert!(
        keywords(&engine, kami).contains(KeywordSet::TRAMPLE),
        "Moss Kami has trample"
    );
}
