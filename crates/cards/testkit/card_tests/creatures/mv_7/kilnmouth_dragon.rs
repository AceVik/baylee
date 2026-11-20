//! `cards/creatures/mv_7/kilnmouth_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_engine::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kilnmouth_dragon() -> CardIndex {
    card_index("5cb73cd3-2cd2-45e2-a593-a498a73b2cf5")
}

/// Kilnmouth Dragon ({5}{R}{R}, 5/5 Dragon):
/// Amplify 3; Flying; {T}: This creature deals damage equal to the number of
/// +1/+1 counters on it to any target.
#[test]
fn kilnmouth_dragon_amplify_3_pings_player_and_creature_for_counter_count() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .battlefield(1, &[index::RAMPAGING_BALOTHS])
        .hand(
            0,
            &[
                kilnmouth_dragon(),
                shivan_dragon(),
                shivan_dragon(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, kilnmouth_dragon());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let bear = in_hand(&engine, P0, grizzly_bears()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        options.len(),
        2,
        "only the two Dragons in hand are eligible"
    );
    assert!(!options.contains(&bear));
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, P0, kilnmouth_dragon()).unwrap();
    assert_eq!(counters_on(&engine, dragon, CounterKind::P1P1), 6);
    assert_eq!(pt(&engine, dragon), (11, 11));

    // Turn 3: ping opponent for 6 damage.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    activate(&mut engine, P0, kilnmouth_dragon(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 14);

    // Turn 5: untap and ping P1's 6/6 Rampaging Baloths to destroy it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 5
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    let baloths = on_battlefield(&engine, P1, index::RAMPAGING_BALOTHS).unwrap();
    activate(&mut engine, P0, kilnmouth_dragon(), 0);
    aim_at(&mut engine, P0, baloths);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P1, index::RAMPAGING_BALOTHS).is_some());
}

#[test]
fn kilnmouth_dragon_has_flying_in_combat() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[kilnmouth_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, kilnmouth_dragon());
    pass_until(&mut engine, stack_is_empty);
    let dragon = on_battlefield(&engine, P0, kilnmouth_dragon()).unwrap();

    // Turn 3: ground Grizzly Bears cannot block flying Kilnmouth Dragon.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    let bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let blockers = attack_and_collect_blocks(&mut engine, dragon, P1);
    assert!(
        !baylee_card_testkit::card_tests::legends_kit::may_block(&blockers, bear, dragon),
        "ground creature cannot block flying Kilnmouth Dragon"
    );
    baylee_card_testkit::card_tests::legends_kit::block(&mut engine, &[]);
    baylee_card_testkit::card_tests::legends_kit::through_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 15);
}

#[test]
fn kilnmouth_dragon_zero_reveals_is_five_five_and_deals_zero_damage() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[kilnmouth_dragon(), shivan_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, kilnmouth_dragon());
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

    let dragon = on_battlefield(&engine, P0, kilnmouth_dragon()).unwrap();
    assert_eq!(counters_on(&engine, dragon, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, dragon), (5, 5));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    activate(&mut engine, P0, kilnmouth_dragon(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
}
