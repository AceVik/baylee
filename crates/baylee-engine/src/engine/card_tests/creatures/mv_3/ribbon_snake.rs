//! `cards/creatures/mv_3/ribbon_snake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn ribbon_snake() -> CardIndex {
    card_index("98b53af9-af1b-4d5d-ae5a-bd444c1339d2")
}

/// Ribbon Snake ({1}{U}{U} 2/3 Creature — Snake):
/// "Flying"
/// "{2}: This creature loses flying until end of turn. Any player may activate this ability."
#[test]
fn ribbon_snake_loses_flying_until_end_of_turn_when_opponent_activates_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = ribbon_snake();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    let snake = on_battlefield(&engine, p0, subject).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    assert!(keywords(&engine, snake).contains(KeywordSet::FLYING));
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(snake, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: snake,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );

    assert!(!keywords(&engine, snake).contains(KeywordSet::FLYING));
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == bears && b.attackers.contains(&snake)),
        "Grizzly Bears should be able to block Ribbon Snake after it loses flying"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, snake)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
}
