//! `cards/creatures/mv_4/daru_stinger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_core::ids::Defender;
use baylee_engine::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn daru_stinger() -> CardIndex {
    card_index("696bff10-4e46-48a7-b2af-13387b9d9da0")
}

/// Daru Stinger ({3}{W}, 1/1 Soldier):
/// Amplify 1; {T}: This creature deals damage equal to the number of +1/+1
/// counters on it to target attacking or blocking creature.
#[test]
fn daru_stinger_amplifies_soldiers_and_hits_attacking_and_blocking_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), savannah_lions()])
        .battlefield(1, &[grizzly_bears()])
        .hand(
            0,
            &[
                daru_stinger(),
                index::BENALISH_HERO,
                index::BENALISH_HERO,
                bird_maiden(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, daru_stinger());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let non_soldier = in_hand(&engine, P0, bird_maiden()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2, "only the two Soldiers in hand are eligible");
    assert!(!options.contains(&non_soldier));
    engine.apply(P0, PlayerAction::ChooseObjects { objects: options }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let stinger = on_battlefield(&engine, P0, daru_stinger()).unwrap();
    assert_eq!(counters_on(&engine, stinger, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, stinger), (3, 3));

    // Outside combat on turn 3, there are no attacking/blocking creatures so {T} has no legal target.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|(src, _)| *src == stinger),
        "Daru Stinger cannot activate outside combat with no attacking or blocking creature"
    );

    // P0 attacks with Savannah Lions; P1 blocks with Grizzly Bears -> Stinger pings blocking Bear for 2!
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let lions = on_battlefield(&engine, P0, savannah_lions()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lions, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
    );
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_bear, lions)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    activate(&mut engine, P0, daru_stinger(), 0);
    aim_at(&mut engine, P0, p1_bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P1, grizzly_bears()).is_some(),
        "2 damage from amplified Daru Stinger kills the 2/2 blocking Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P0, savannah_lions()).is_some(),
        "Savannah Lions survives because blocking Bear died before combat damage"
    );
}

#[test]
fn daru_stinger_hits_attacking_creature_during_combat() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[daru_stinger(), index::BENALISH_HERO, index::BENALISH_HERO])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, daru_stinger());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine.apply(P0, PlayerAction::ChooseObjects { objects: options }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance past Turn 2 & Turn 3 (so Daru Stinger loses summoning sickness at P0's upkeep),
    // then on Turn 4 (P1's turn) P1 attacks with Grizzly Bears -> P0 activates Daru Stinger to destroy it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1)
    });
    engine
        .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.number >= 4
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1)
    });
    let bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    activate(&mut engine, P0, daru_stinger(), 0);
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
}
