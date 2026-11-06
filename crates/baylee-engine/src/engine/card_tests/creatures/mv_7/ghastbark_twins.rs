//! `cards/creatures/mv_7/ghastbark_twins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ghastbark_twins() -> CardIndex {
    card_index("bdd0d01d-b9f2-4c3f-9b3b-04e122013de5")
}

/// Ghastbark Twins — a 7/7 trampler for {5}{G}{G}.
#[test]
fn ghastbark_twins_costs_seven_and_tramples() {
    let mut engine = table7(&lots(forest(), 7), &[ghastbark_twins()], &[]);
    let twins = cast_and_resolve(&mut engine, ghastbark_twins());
    assert_body(&engine, twins, (7, 7), KeywordSet::TRAMPLE);
}

/// "This creature can block an additional creature each combat": against two
/// attacking Ogres the Twins are offered with room for two, a plain Ogre of
/// the same controller is not, and blocking both kills both.
#[test]
fn ghastbark_twins_block_two_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ghastbark_twins(), gray_ogre()])
        .battlefield(1, &[gray_ogre(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let twins = on_battlefield(&engine, p0, ghastbark_twins()).expect("out");
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let theirs = all_on_battlefield(&engine, p1, gray_ogre());
    assert_eq!(theirs.len(), 2);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: theirs.iter().map(|o| (*o, Defender::Player(p0))).collect(),
            },
        )
        .expect("both Ogres attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { capacity, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(
        capacity
            .iter()
            .any(|c| c.blocker == twins && c.most == Some(2)),
        "the Twins may block two: {capacity:?}"
    );
    assert!(
        !capacity.iter().any(|c| c.blocker == mine),
        "an ordinary Ogre blocks one at most"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(twins, theirs[0]), (twins, theirs[1])],
            },
        )
        .expect("one creature blocking two attackers is legal for the Twins");
    // The Twins split their 7 damage over the two attackers they block: two
    // each is lethal, and the last takes whatever is left.
    let mut asked = 0;
    loop {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
                || (e.state().turn.phase == Phase::SecondMain && stack_is_empty(e))
        });
        let Pending::ChooseNumber {
            reason: crate::choice::NumberPrompt::CombatDamage { left, .. },
            ..
        } = engine.pending().clone()
        else {
            break;
        };
        asked += 1;
        engine
            .apply(p0, PlayerAction::ChooseNumber(left.min(2)))
            .expect("two is within the damage left");
    }
    assert_eq!(asked, 1, "the first is asked, the last takes the rest");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2,
        "7 damage split 2 and 2 kills both 2/2s"
    );
    assert_eq!(life_of(&engine, 0), 20, "nothing got through");
}

/// The Twins attacking the opponent's Oko at a table that asks, both of seat
/// 1's Ogres on them, the question about to come.
fn twins_at_oko_blocked_by_two() -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ghastbark_twins()])
        .battlefield(1, &[forest(), island(), forest(), gray_ogre(), gray_ogre()])
        .hand(1, &[oko_thief_of_crowns()])
        .start();
    engine.house_rules.divide_trample = true;
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);
    let oko = on_battlefield(&engine, p1, oko_thief_of_crowns()).expect("resolved");
    activate(&mut engine, p1, oko_thief_of_crowns(), 0);
    pass_until(&mut engine, stack_is_empty);
    let twins = on_battlefield(&engine, p0, ghastbark_twins()).expect("out");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&twins)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(twins, Defender::Planeswalker(oko))],
            },
        )
        .expect("at the Oko");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let ogres = all_on_battlefield(&engine, p1, gray_ogre());
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: ogres.iter().map(|ogre| (*ogre, twins)).collect(),
            },
        )
        .expect("both Ogres block");
    (engine, oko, ogres)
}

/// The next share asked, as its recipient, `min`, `max` and `lethal`.
fn next_share(engine: &mut Engine<RegistryLookup>) -> (ObjectId, u32, u32, Option<u32>) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        min,
        max,
        reason:
            crate::choice::NumberPrompt::CombatDamage {
                recipient, lethal, ..
            },
        ..
    } = engine.pending().clone()
    else {
        panic!("a share of the Twins' damage: {:?}", engine.pending())
    };
    (recipient, min, max, lethal)
}

/// "Trample": at a table that asks, the Twins' controller gives each Ogre
/// lethal and what is left goes to the planeswalker they attack, not past it
/// to the player (CR 702.19b, 702.19f).
#[test]
fn ghastbark_twins_trample_over_two_blockers_onto_the_planeswalker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, oko, ogres) = twins_at_oko_blocked_by_two();
    assert_eq!(next_share(&mut engine), (ogres[0], 0, 7, Some(2)));
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(
        next_share(&mut engine),
        (ogres[1], 2, 5, Some(2)),
        "the last Ogre is owed lethal, and the rest is the choice"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2,
        "two to one Ogre and three to the other kills both"
    );
    assert_eq!(
        counters_on(&engine, oko, CounterKind::Loyalty),
        4,
        "6 − the 2 that trampled over"
    );
    assert_eq!(
        life_of(&engine, 1),
        20,
        "the player is not what they attack"
    );
}

/// The walker gone, nothing is there to reach (CR 702.19f): it is a plain
/// division of the Twins' damage among the Ogres, and the last takes the rest.
#[test]
fn ghastbark_twins_with_nothing_to_reach_divide_it_all_among_the_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, oko, ogres) = twins_at_oko_blocked_by_two();
    engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up")
        .move_object(
            oko,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the walker leaves");
    assert_eq!(next_share(&mut engine), (ogres[0], 0, 7, None));
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    let first = engine.state().object(ogres[0]).expect("spared");
    assert_eq!(first.damage, 1);
    assert!(
        !all_on_battlefield(&engine, p1, gray_ogre()).contains(&ogres[1]),
        "the last took the other 6"
    );
    assert_eq!(life_of(&engine, 1), 20);
}
