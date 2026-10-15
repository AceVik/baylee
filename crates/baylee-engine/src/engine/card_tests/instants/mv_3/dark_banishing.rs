// Oracle checklist: nonblack own/opponent creatures legal; black and noncreature excluded; actual regeneration shield bypassed; indestructible survives; bystanders stay.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("0df450be-9bd2-45be-ae59-fa8d19f6a391")
}

#[test]
fn nonblack_filter_destruction_and_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (card, dies) in [(fangren_hunter(), true), (darksteel_gargoyle(), false)] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[swamp(), swamp(), swamp(), wild_elephant()])
            .battlefield(1, &[card, lotleth_troll(), quiet_artifact()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let target = on_battlefield(&game, p1, card).unwrap();
        let own = on_battlefield(&game, p0, wild_elephant()).unwrap();
        let black = on_battlefield(&game, p1, lotleth_troll()).unwrap();
        let artifact = on_battlefield(&game, p1, quiet_artifact()).unwrap();
        cast_from_hand(&mut game, p0, subject());
        let options = aim_at(&mut game, p0, target);
        assert!(options.contains(&own));
        assert!(!options.contains(&black));
        assert!(!options.contains(&artifact));
        pass_until(&mut game, stack_is_empty);
        assert_eq!(in_graveyard(&game, p1, card).is_some(), dies);
        assert_eq!(on_battlefield(&game, p1, card).is_some(), !dies);
        assert_eq!(on_battlefield(&game, p0, wild_elephant()), Some(own));
        assert_eq!(on_battlefield(&game, p1, lotleth_troll()), Some(black));
    }
}

#[test]
fn destroys_through_a_real_regeneration_shield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let boa = baylee_core::generated::index::RIVER_BOA;
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), boa])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let target = on_battlefield(&game, p1, boa).unwrap();
    cast_from_hand(&mut game, p0, subject());
    aim_at(&mut game, p0, target);
    game.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut game, p1);
    game.apply(
        p1,
        PlayerAction::ActivateAbility {
            source: target,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut game, |e| {
        e.state().object(target).unwrap().regeneration_shields == 1
    });
    assert!(on_battlefield(&game, p1, boa).is_some());
    pass_until(&mut game, stack_is_empty);
    assert!(in_graveyard(&game, p1, boa).is_some());
}
