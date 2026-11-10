//! `cards/creatures/mv_3/goblin_king.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin King — "Other Goblins get +1/+1 and have mountainwalk." The king
/// leaves itself and a non-Goblin beside it untouched.
#[test]
fn goblin_king_pumps_other_goblins_and_grants_mountainwalk() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[goblin_king(), mons_s_goblin_raiders(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    let king = on_battlefield(&engine, p0, goblin_king()).expect("seated");
    let goblin = on_battlefield(&engine, p0, mons_s_goblin_raiders()).expect("another Goblin");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("not a Goblin");

    assert_eq!(pt(&engine, king), (2, 2), "it does not pump itself");
    assert!(!keywords(&engine, king).contains(KeywordSet::MOUNTAINWALK));
    assert_eq!(
        pt(&engine, goblin),
        (2, 2),
        "1/1 printed, +1/+1 from the king"
    );
    assert!(keywords(&engine, goblin).contains(KeywordSet::MOUNTAINWALK));
    assert_eq!(pt(&engine, elf), (1, 1), "not a Goblin: untouched");
    assert!(!keywords(&engine, elf).contains(KeywordSet::MOUNTAINWALK));
}
