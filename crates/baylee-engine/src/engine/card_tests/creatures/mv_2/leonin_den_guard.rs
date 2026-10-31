//! Leonin Den-Guard's equipped bonus and vigilance, played into combat.

use super::*;

fn leonin_den_guard() -> CardIndex {
    card_index("651220f7-223a-49b3-995d-32f08eb3a16a")
}

#[test]
fn leonin_den_guard_gets_bonus_and_attacks_without_tapping_when_equipped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[leonin_den_guard(), lightning_greaves()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let guard = on_battlefield(&e, p0, leonin_den_guard()).unwrap();
    let equipment = on_battlefield(&e, p0, lightning_greaves()).unwrap();
    assert_eq!(pt(&e, guard), (1, 3));
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: equipment,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![guard],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, guard), (2, 4));
    assert!(keywords(&e, guard).contains(KeywordSet::VIGILANCE));
    attack_and_collect_blocks(&mut e, guard, p1);
    assert!(!is_tapped(&e, guard));
}
