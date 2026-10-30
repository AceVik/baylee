//! Nim Shrieker's artifact count and flying, played.

use super::*;

fn nim_shrieker() -> CardIndex {
    card_index("1dff7c82-e49f-45e8-8f66-d5801f420fa6")
}

#[test]
fn nim_shrieker_gains_power_and_cannot_be_blocked_by_ground_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[nim_shrieker(), forest()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let shrieker = on_battlefield(&e, p0, nim_shrieker()).unwrap();
    cast_from_hand(&mut e, p0, quiet_artifact());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, shrieker), (1, 1));
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    let options = attack_and_collect_blocks(&mut e, shrieker, p1);
    assert!(
        !options
            .iter()
            .any(|option| { option.blocker == ogre && option.attackers.contains(&shrieker) })
    );
}
