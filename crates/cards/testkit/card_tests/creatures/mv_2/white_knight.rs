//! `cards/creatures/mv_2/white_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// White Knight — First strike; Protection from black. A black instant
/// cannot even name it as a target while a legal white one sits beside it.
#[test]
fn white_knight_has_first_strike_and_protection_from_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[white_knight(), pearled_unicorn()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    let knight = on_battlefield(&engine, p0, white_knight()).expect("seated");
    let unicorn =
        on_battlefield(&engine, p0, pearled_unicorn()).expect("a legal, unprotected target");
    assert!(
        keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE),
        "first strike is the card's other printed line"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, hero_s_downfall());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Hero's Downfall's target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&unicorn),
        "the unprotected creature is offered"
    );
    assert!(
        !options.contains(&knight),
        "protection from black: a black spell cannot target it: {options:?}"
    );
}
