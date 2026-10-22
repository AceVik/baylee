//! `cards/enchantments/mv_3/dystopia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dystopia() -> CardIndex {
    card_index("d0e6203b-cef2-470f-8c79-7b139290e2d5")
}

fn mesa_pegasus() -> CardIndex {
    card_index("8161f5b8-6aab-4133-ba2c-2e7b5774153e")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

/// `Dystopia` ({1}{B}{B}, Enchantment) has cumulative upkeep—Pay 1 life, and at
/// the beginning of each player's upkeep, that player sacrifices a green or
/// white permanent of their choice.
#[test]
fn dystopia_makes_each_player_sacrifice_green_or_white_permanent_and_charges_cumulative_upkeep_life()
 {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), mesa_pegasus()])
        .battlefield(1, &[grizzly_bears(), drudge_skeletons()])
        .hand(0, &[dystopia()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, dystopia());
    pass_until(&mut engine, stack_is_empty);
    let dys = on_battlefield(&engine, p0, dystopia()).expect("Dystopia enters");

    // Advance to P1's upkeep: P1 must sacrifice Grizzly Bears (green), while Drudge Skeletons (black) is spared.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
            || in_graveyard(e, p1, grizzly_bears()).is_some()
    });
    if let Pending::ChooseCards { .. } = engine.pending() {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, drudge_skeletons()).is_some());

    // Advance to P0's upkeep: pay 1 life for cumulative upkeep (1 age counter) and sacrifice Mesa Pegasus (white).
    let pegasus = on_battlefield(&engine, p0, mesa_pegasus()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    for _ in 0..8 {
        if stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::YesNo { .. } => {
                engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pegasus],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending on P0 upkeep: {other:?}"),
        }
    }
    assert_eq!(
        counters_on(&engine, dys, baylee_cards_dsl::counters::AGE),
        1
    );
    assert_eq!(engine.state().players[0].life, 19);
    assert!(in_graveyard(&engine, p0, mesa_pegasus()).is_some());
    assert!(on_battlefield(&engine, p0, dystopia()).is_some());
}
