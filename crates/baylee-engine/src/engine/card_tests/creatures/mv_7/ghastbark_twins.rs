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
