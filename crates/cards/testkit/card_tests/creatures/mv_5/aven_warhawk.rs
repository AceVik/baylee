//! `cards/creatures/mv_5/aven_warhawk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_core::ids::Defender;
use baylee_engine::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aven_warhawk() -> CardIndex {
    card_index("7d3e5d95-2658-4c1a-9ed3-df1f554d87ea")
}

/// Aven Warhawk ({4}{W}, 2/2 Bird Soldier):
/// Amplify 1; Flying.
#[test]
fn aven_warhawk_amplifies_bird_and_soldier_cards_and_flies() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .battlefield(1, &[grizzly_bears(), bird_maiden()])
        .hand(
            0,
            &[
                aven_warhawk(),
                aven_warhawk(),
                bird_maiden(),
                index::BENALISH_HERO,
                scathe_zombies(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, aven_warhawk());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let other_warhawk = in_hand(&engine, P0, aven_warhawk()).unwrap();
    let bird = in_hand(&engine, P0, bird_maiden()).unwrap();
    let soldier = in_hand(&engine, P0, index::BENALISH_HERO).unwrap();
    let zombie = in_hand(&engine, P0, scathe_zombies()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 3);
    assert!(options.contains(&other_warhawk));
    assert!(options.contains(&bird));
    assert!(options.contains(&soldier));
    assert!(!options.contains(&zombie));
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let warhawk = on_battlefield(&engine, P0, aven_warhawk()).unwrap();
    assert_eq!(
        counters_on(&engine, warhawk, CounterKind::P1P1),
        3,
        "Bird Soldier card in hand counts once per card, not twice (CR 702.38a)"
    );
    assert_eq!(pt(&engine, warhawk), (5, 5));

    // Turn 3: Aven Warhawk attacks with flying -> Grizzly Bears cannot block, Bird Maiden can.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(warhawk, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
    );
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_bird = on_battlefield(&engine, P1, bird_maiden()).unwrap();
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_bear, warhawk)],
                }
            )
            .is_err(),
        "ground Grizzly Bears cannot block flying Aven Warhawk"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_bird, warhawk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, P1, bird_maiden()).is_some());
    assert!(on_battlefield(&engine, P0, aven_warhawk()).is_some());
}

#[test]
fn aven_warhawk_zero_reveals_enters_as_two_two() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[aven_warhawk(), bird_maiden()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, aven_warhawk());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let warhawk = on_battlefield(&engine, P0, aven_warhawk()).unwrap();
    assert_eq!(counters_on(&engine, warhawk, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, warhawk), (2, 2));
}
