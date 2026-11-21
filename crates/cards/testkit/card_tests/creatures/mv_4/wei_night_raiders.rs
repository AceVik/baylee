//! `cards/creatures/mv_4/wei_night_raiders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wei_night_raiders() -> CardIndex {
    card_index("d24ee48a-86a9-4a72-9311-79b6ea3f41f2")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Wei Night Raiders (d24ee48a-86a9-4a72-9311-79b6ea3f41f2) — `{2}{B}{B}` 2/2
/// Creature — Human Soldier:
/// - Horsemanship
/// - Whenever this creature deals damage to an opponent, that player discards a card.
#[test]
fn wei_night_raiders_evades_with_horsemanship_and_forces_damaged_opponent_to_discard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[wei_night_raiders()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, wei_night_raiders());
    pass_until(&mut engine, stack_is_empty);
    let raiders = on_battlefield(&engine, P0, wei_night_raiders()).expect("Raiders entered");
    assert_eq!(pt(&engine, raiders), (2, 2));
    assert!(keywords(&engine, raiders).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let keep = in_hand(&engine, P1, forest()).unwrap();
    let pitch = in_hand(&engine, P1, island()).unwrap();

    let blocks = attack_and_collect_blocks(&mut engine, raiders, P1);
    assert!(blocks.is_empty(), "Grizzly Bears lacks horsemanship");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards for P1 discard, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P1);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&keep) && options.contains(&pitch));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, forest()).is_some());
}

#[test]
fn wei_night_raiders_blocked_by_horsemanship_creature_does_not_trigger_discard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[wei_night_raiders()])
        .battlefield(1, &[wu_elite_cavalry()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let raiders = on_battlefield(&engine, P0, wei_night_raiders()).unwrap();
    let cavalry = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(raiders, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(cavalry, raiders)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_hand(&engine, P1, forest()).is_some());
}
