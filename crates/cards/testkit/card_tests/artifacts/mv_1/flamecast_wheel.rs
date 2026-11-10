//! Oracle: {5}, {T}, Sacrifice this artifact: It deals 3 damage to target creature.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("58f70225-17b5-4aa6-a62a-89c57e31ecc1")
}

// Checklist: exact mana, tap and self-sacrifice costs; own/opposing creature
// targets, noncreature exclusions, exact damage and unchanged bystanders.
#[test]
fn damage_and_sacrifice_cost_are_exact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for target_own in [false, true] {
        let mut board = vec![subject(), index::HILL_GIANT];
        board.extend(vec![island(); 5]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[index::HILL_GIANT, sol_ring()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let own = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
        let foe = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
        let artifact = on_battlefield(&e, p1, sol_ring()).unwrap();
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("target prompt")
        };
        assert!(options.contains(&own));
        assert!(options.contains(&foe));
        assert!(!options.contains(&artifact));
        assert!(player_options.is_empty());
        let target = if target_own { own } else { foe };
        aim_at(&mut e, p0, target);
        assert!(
            in_graveyard(&e, p0, subject()).is_some(),
            "sacrifice is paid before resolving"
        );
        assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 0);
        pass_until(&mut e, stack_is_empty);
        assert!(in_graveyard(&e, if target_own { p0 } else { p1 }, index::HILL_GIANT).is_some());
        assert_eq!(
            e.state()
                .object(if target_own { foe } else { own })
                .unwrap()
                .damage,
            0
        );
        assert!(on_battlefield(&e, p1, sol_ring()).is_some());
    }
}
#[test]
fn insufficient_mana_and_tapped_source_are_not_offered() {
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut board = vec![subject()];
        board.extend(vec![island(); if tapped { 5 } else { 4 }]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[index::HILL_GIANT])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        if tapped {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(source)
                .unwrap()
                .status
                .insert(Status::TAPPED);
            e.refresh_offer();
        }
        tap_all_mana(&mut e, p0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
        assert!(on_battlefield(&e, p0, subject()).is_some());
    }
}

// Resolution boundary: cost remains paid when the only creature target leaves.
#[test]
fn target_leaving_in_response_prevents_damage_but_not_cost() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![subject()];
    board.extend(vec![island(); 5]);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[island(), index::HILL_GIANT, index::GRAY_OGRE])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let target = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    let bystander = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, subject(), 0);
    aim_at(&mut e, p0, target);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    aim_at(&mut e, p1, target);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p1, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&e, p0, subject()).is_some());
    assert_eq!(e.state().object(bystander).unwrap().damage, 0);
}

// Exact amount boundary: a 4-toughness survivor distinguishes three from four.
#[test]
fn surviving_creature_has_exactly_three_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[subject(), island(), island(), island(), island(), island()],
        )
        .battlefield(1, &[fangren_hunter(), index::GRAY_OGRE])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let target = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    let bystander = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    assert_eq!(pt(&e, target), (4, 4));
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, subject(), 0);
    aim_at(&mut e, p0, target);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(on_battlefield(&e, p1, fangren_hunter()), Some(target));
    assert_eq!(e.state().object(target).unwrap().damage, 3);
    assert_eq!(e.state().object(bystander).unwrap().damage, 0);
    assert!(in_graveyard(&e, p0, subject()).is_some());
}
