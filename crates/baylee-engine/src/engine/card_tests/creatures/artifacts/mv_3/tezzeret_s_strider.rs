//! `cards/creatures/artifacts/mv_3/tezzeret_s_strider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tezzeret_s_strider() -> CardIndex {
    card_index("cac79a2b-1de3-4461-95ac-fa0f1129cd6b")
}

/// Tezzeret's Strider — "As long as you control a Tezzeret planeswalker, this
/// creature has menace." The pool holds no Tezzeret planeswalker to put out
/// for the positive half, so this reads the other: with none, there is no
/// menace, and a single blocker may block it.
#[test]
fn tezzeret_s_strider_has_no_menace_without_a_tezzeret() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tezzeret_s_strider()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let strider = on_battlefield(&engine, p0, tezzeret_s_strider()).expect("seated");
    assert_eq!(pt(&engine, strider), (3, 1));
    assert!(!keywords(&engine, strider).contains(KeywordSet::MENACE));

    let options = attack_to_blocks(&mut engine, &[strider]);
    let blocker = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    assert!(may_block(&options, blocker, strider));
    assert!(engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, strider)],
            },
        )
        .is_ok());
}
