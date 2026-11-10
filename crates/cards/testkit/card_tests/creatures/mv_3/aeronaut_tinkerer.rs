//! Oracle: This creature has flying as long as you control an artifact. (It can't be blocked except by creatures with flying or reach.)
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("a15bb807-ea31-4bff-b573-c249c4c78dd6")
}

// Checklist: opponent artifact is insufficient; own artifact grants flying;
// ground blockers become illegal while flying, flying blocker stays legal;
// real Shatter on last own artifact restores ground blocking.
#[test]
fn artifact_condition_changes_actual_blocking_pairs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for state in 0..3 {
        let artifact = state != 0;
        let mut board = vec![subject(), mountain(), mountain()];
        if artifact {
            board.push(sol_ring());
        }
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(
                1,
                &[
                    index::GRAY_OGRE,
                    index::ORNITHOPTER,
                    giant_spider(),
                    sol_ring(),
                ],
            )
            .hand(0, &[shatter()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        assert_eq!(keywords(&e, source).contains(KeywordSet::FLYING), artifact);
        if state == 2 {
            let ring = on_battlefield(&e, p0, sol_ring()).unwrap();
            cast_from_hand(&mut e, p0, shatter());
            aim_at(&mut e, p0, ring);
            pass_until(&mut e, stack_is_empty);
            assert!(!keywords(&e, source).contains(KeywordSet::FLYING));
        }
        let ground = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
        let reach = on_battlefield(&e, p1, giant_spider()).unwrap();
        let flying = on_battlefield(&e, p1, index::ORNITHOPTER).unwrap();
        let pairs = attack_and_collect_blocks(&mut e, source, p1);
        assert!(
            pairs
                .iter()
                .any(|b| b.blocker == reach && b.attackers.contains(&source))
        );
        assert_eq!(
            pairs
                .iter()
                .any(|b| b.blocker == ground && b.attackers.contains(&source)),
            state != 1
        );
        assert!(
            pairs
                .iter()
                .any(|b| b.blocker == flying && b.attackers.contains(&source))
        );
    }
}
