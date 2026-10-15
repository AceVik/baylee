//! `cards/creatures/artifacts/mv_1/goldhound.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goldhound() -> CardIndex {
    card_index("b5eb72bb-897f-4911-a797-3153c31df3e0")
}

/// "{T}, Sacrifice this creature: Add one mana of any color." Activated, it
/// asks which color, then leaves for the graveyard with exactly that one
/// mana in the pool.
#[test]
fn goldhound_sacrifices_for_one_mana_of_a_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[goldhound()], &[]);
    let me = on_battlefield(&engine, p0, goldhound()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 1));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, goldhound(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected a color question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 5, "any of the five colors");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red is on the menu");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1, "one mana");
    assert!(
        in_graveyard(&engine, p0, goldhound()).is_some(),
        "sacrificed"
    );
}

/// "First strike" and "menace" (CR 702.7, 702.111): a 1/1 first striker
/// attacking into a lone blocker may not be blocked by one creature, and the
/// opponent's single creature cannot block it at all.
#[test]
fn goldhound_cannot_be_blocked_by_a_single_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[goldhound()], &[gray_ogre()]);
    let me = on_battlefield(&engine, p0, goldhound()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    assert!(keywords(&engine, me).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&engine, me).contains(KeywordSet::MENACE));
    let life = engine.state().players[1].life;

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(me, Defender::Player(p1))],
            },
        )
        .expect("attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(ogre, me)],
                },
            )
            .is_err(),
        "menace: one blocker is not enough"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, life - 1);
}
