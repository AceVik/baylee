//! Oracle checklist: {T} cost; target must be another permanent (not self); land and creature targets are both legal; only chosen tapped target becomes untapped.
#[allow(clippy::wildcard_imports)]
use super::*;
fn kiora_s_follower() -> CardIndex {
    card_index("22c044ad-77d7-4c93-953d-e2daa9686ff7")
}

#[test]
fn follower_untaps_another_target_permanent_but_not_itself() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[kiora_s_follower(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let follower = on_battlefield(&e, p0, kiora_s_follower()).unwrap();
    let land = on_battlefield(&e, p0, forest()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    tap_mana_where(&mut e, p0, |id| id == land || id == elf);
    activate(&mut e, p0, kiora_s_follower(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("expected permanent target prompt: {:?}", e.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&land), "a land is a permanent");
    assert!(options.contains(&elf), "a creature is a permanent");
    assert!(!options.contains(&follower), "another excludes the source");
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(is_tapped(&e, follower));
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, elf));
    assert!(
        is_tapped(&e, land),
        "the unchosen tapped permanent stays tapped"
    );
    assert!(is_tapped(&e, follower));
}

#[test]
fn follower_can_choose_a_land_as_its_permanent_target() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[kiora_s_follower(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let follower = on_battlefield(&e, p0, kiora_s_follower()).unwrap();
    let lands: Vec<_> = e
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(lands.len(), 2);
    let target = lands[0];
    tap_mana_where(&mut e, p0, |id| id == target);
    activate(&mut e, p0, kiora_s_follower(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected permanent target prompt: {:?}", e.pending());
    };
    assert!(options.contains(&target));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, target));
    assert!(is_tapped(&e, follower));
    assert!(!is_tapped(&e, lands[1]), "the other land was never tapped");
}
