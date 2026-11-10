//! `cards/creatures/mv_2/black_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Black Knight — First strike; Protection from white. A white instant
/// cannot even name it as a target.
#[test]
fn black_knight_has_first_strike_and_protection_from_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[black_knight(), pearled_unicorn()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let knight = on_battlefield(&engine, p0, black_knight()).expect("seated");
    let unicorn =
        on_battlefield(&engine, p0, pearled_unicorn()).expect("a legal, unprotected target");
    assert!(keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Swords to Plowshares' target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&unicorn));
    assert!(
        !options.contains(&knight),
        "protection from white: a white spell cannot target it: {options:?}"
    );
}
