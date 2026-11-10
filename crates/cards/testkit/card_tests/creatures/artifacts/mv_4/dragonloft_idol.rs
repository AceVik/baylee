//! `cards/creatures/artifacts/mv_4/dragonloft_idol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dragonloft_idol() -> CardIndex {
    card_index("736ce43b-28cd-4b8b-9936-3e3975a56b6a")
}

/// Dragonloft Idol — {4} 3/3: "As long as you control a Dragon, this
/// creature gets +1/+1 and has flying and trample." Alone it is a 3/3 without
/// either keyword; a Shivan Dragon cast beside it makes it a 4/4 flier with
/// trample, and the bonus goes when the Dragon does.
#[test]
fn dragonloft_idol_follows_the_dragon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dragonloft_idol(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[shivan_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let idol = on_battlefield(&engine, p0, dragonloft_idol()).expect("seated");
    assert_eq!(pt(&engine, idol), (3, 3));
    assert!(!keywords(&engine, idol).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, idol).contains(KeywordSet::TRAMPLE));

    cast_from_hand(&mut engine, p0, shivan_dragon());
    pass_until(&mut engine, stack_is_empty);
    let dragon = on_battlefield(&engine, p0, shivan_dragon()).expect("it resolved");
    assert_eq!(pt(&engine, idol), (4, 4), "+1/+1");
    assert!(keywords(&engine, idol).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, idol).contains(KeywordSet::TRAMPLE));

    kill(&mut engine, dragon);
    assert!(on_battlefield(&engine, p0, shivan_dragon()).is_none());
    assert_eq!(pt(&engine, idol), (3, 3), "no Dragon, no bonus");
    assert!(!keywords(&engine, idol).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, idol).contains(KeywordSet::TRAMPLE));
}

/// "You control": an opponent's Dragon gives the Idol nothing.
#[test]
fn dragonloft_idol_ignores_an_opponents_dragon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragonloft_idol()])
        .battlefield(1, &[shivan_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let idol = on_battlefield(&engine, p0, dragonloft_idol()).expect("seated");
    assert_eq!(pt(&engine, idol), (3, 3));
    assert!(!keywords(&engine, idol).contains(KeywordSet::FLYING));
}
