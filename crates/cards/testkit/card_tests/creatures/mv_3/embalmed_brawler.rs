//! `cards/creatures/mv_3/embalmed_brawler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_core::ids::Defender;
use baylee_engine::choice::ChoicePrompt;
use baylee_engine::event::GameEvent;
use baylee_engine::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn embalmed_brawler() -> CardIndex {
    card_index("42e95ad5-c10a-46fe-ab97-c7a734138586")
}

/// Casts Embalmed Brawler with `reveal_indices` selected from the offered
/// Amplify options (verifying non-Zombie cards are excluded).
fn cast_and_amplify(
    engine: &mut Engine<RegistryLookup>,
    reveal_count: usize,
    excluded: &[CardIndex],
) -> ObjectId {
    let brawler = embalmed_brawler();
    cast_from_hand(engine, P0, brawler);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        ) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt: ChoicePrompt::Amplify,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, P0);
        assert_eq!(min, 0);
        assert_eq!(usize::from(max), options.len());
        for &bad in excluded {
            if let Some(bad_id) = in_hand(engine, P0, bad) {
                assert!(
                    !options.contains(&bad_id),
                    "non-Zombie hand card {bad:?} must not be eligible for Embalmed Brawler Amplify"
                );
            }
        }
        let chosen: Vec<ObjectId> = options.into_iter().take(reveal_count).collect();
        assert_eq!(chosen.len(), reveal_count);
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: chosen })
            .expect("Amplify reveal accepted");
    } else {
        assert_eq!(reveal_count, 0);
    }
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, P0, brawler).expect("Embalmed Brawler entered battlefield")
}

/// Embalmed Brawler ({2}{B}, 2/2 Zombie):
/// Amplify 1; whenever this creature attacks or blocks, you lose 1 life for
/// each +1/+1 counter on it.
#[test]
fn embalmed_brawler_amplifies_zombies_and_drains_controller_on_attack_and_block() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(
            0,
            &[
                embalmed_brawler(),
                scathe_zombies(),
                scathe_zombies(),
                grizzly_bears(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let brawler_id = cast_and_amplify(&mut engine, 2, &[grizzly_bears(), plains()]);
    assert_eq!(counters_on(&engine, brawler_id, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, brawler_id), (4, 4));
    assert!(
        engine
            .journal()
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::Revealed { player, .. } if player == P0)),
        "revealed Zombie cards emit GameEvent::Revealed"
    );

    // Non-Zombie cards in hand remain unchosen; move to P0's next turn to attack.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
            && e.state().turn.number >= 3
    });
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brawler_id, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        18,
        "attacking with 2 +1/+1 counters makes controller lose 2 life"
    );
    assert_eq!(engine.state().players[1].life, 16, "4/4 unblocked deals 4 combat damage");

    // Skip attacking on Turn 4 (P1) and Turn 5 (P0) so Brawler untaps before blocking on Turn 6.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1)
    });
    engine
        .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 5
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // On P1's next turn (Turn 6), P1 attacks with Grizzly Bears and untapped Embalmed Brawler blocks.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.number >= 6
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
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(brawler_id, bear)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        16,
        "blocking with 2 +1/+1 counters also makes controller lose 2 life"
    );
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_none(),
        "4/4 Brawler kills 2/2 blocking Grizzly Bears"
    );
}

#[test]
fn embalmed_brawler_with_zero_reveals_has_no_counters_and_loses_zero_life() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[embalmed_brawler(), scathe_zombies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let brawler_id = cast_and_amplify(&mut engine, 0, &[]);
    assert_eq!(counters_on(&engine, brawler_id, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, brawler_id), (2, 2));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.number >= 3 && stack_is_empty(e)
    });
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brawler_id, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 18);
}
