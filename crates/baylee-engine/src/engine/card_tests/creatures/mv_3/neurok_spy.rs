//! Neurok Spy's evasion depends on the defending player's artifacts.

use super::*;

fn neurok_spy() -> CardIndex {
    card_index("d78cb7d8-9cfc-4d16-86ea-d49fc5d65db0")
}

fn may_be_blocked(with_artifact: bool) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let theirs: &[CardIndex] = if with_artifact {
        &[gray_ogre(), quiet_artifact()]
    } else {
        &[gray_ogre()]
    };
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[neurok_spy()])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let spy = on_battlefield(&e, p0, neurok_spy()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    attack_and_collect_blocks(&mut e, spy, p1)
        .iter()
        .any(|option| option.blocker == ogre && option.attackers.contains(&spy))
}

#[test]
fn neurok_spy_is_unblockable_only_while_defender_controls_an_artifact() {
    assert!(may_be_blocked(false));
    assert!(!may_be_blocked(true));
}
