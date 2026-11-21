//! `cards/creatures/mv_4/sylvan_messenger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sylvan_messenger() -> CardIndex {
    card_index("ba310102-8272-4d64-a00b-9e7346cb5f46")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Sylvan Messenger (ba310102-8272-4d64-a00b-9e7346cb5f46) — `{3}{G}` 2/2 Creature — Elf:
/// - Trample
/// - When this creature enters, reveal the top four cards of your library. Put all
///   Elf cards revealed this way into your hand and the rest on the bottom of your
///   library in any order.
#[test]
fn sylvan_messenger_puts_revealed_elves_into_hand_and_tramples_in_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[festering_goblin()])
        .hand(
            0,
            &[
                sylvan_messenger(),
                llanowar_elves(),
                llanowar_elves(),
                grizzly_bears(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let miss_bear = hand_to_library_top(&mut engine, P0, grizzly_bears());
    let _elf_a = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let miss_island = hand_to_library_top(&mut engine, P0, island());
    let _elf_b = hand_to_library_top(&mut engine, P0, llanowar_elves());

    cast_from_hand(&mut engine, P0, sylvan_messenger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_island, miss_bear]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let messenger = on_battlefield(&engine, P0, sylvan_messenger()).expect("Messenger entered");
    assert_eq!(pt(&engine, messenger), (2, 2));
    assert!(keywords(&engine, messenger).contains(KeywordSet::TRAMPLE));
    let elves_in_hand = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .filter(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
        })
        .count();
    assert_eq!(elves_in_hand, 2);

    // Next turn cycle: 2/2 Sylvan Messenger tramples 1 excess damage over 1/1 Festering Goblin.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let blocker = on_battlefield(&engine, P1, festering_goblin()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(messenger, Defender::Player(P1))],
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
                blockers: vec![(blocker, messenger)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, P1, messenger);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19, "1 trample damage dealt");
}
