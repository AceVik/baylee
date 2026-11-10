//! `cards/sorceries/mv_3/fiery_gambit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn fiery_gambit() -> CardIndex {
    card_index("78497c54-7179-41b7-9454-7b915fd96cd5")
}

/// Fiery Gambit — {2}{R} Sorcery
/// "Flip a coin until you lose a flip or choose to stop flipping. If you lose a flip, Fiery Gambit has no effect. If you win one or more flips, Fiery Gambit deals 3 damage to target creature. If you win two or more flips, Fiery Gambit deals 6 damage to each opponent. If you win three or more flips, draw nine cards and untap all lands you control."
#[test]
fn fiery_gambit_cumulative_coin_flips_deal_damage_or_fizzle_on_loss() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Try seeds until we find one where P0 wins the first flip, then choose to stop after 1 win.
    let mut found_win = false;
    for seed in 1..50 {
        let mut engine = Duel::new(seed, mountain())
            .hand(0, &[fiery_gambit()])
            .battlefield(0, &[mountain(), mountain(), mountain()])
            .battlefield(1, &[HILL_GIANT])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
        cast_from_hand(&mut engine, p0, fiery_gambit());
        aim_at(&mut engine, p0, giant);

        // Pass priority until either YesNo (won first flip) or stack is empty (lost first flip).
        for _ in 0..10 {
            if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
                break;
            }
            match engine.pending().clone() {
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::YesNo { player, .. } => {
                    // Won 1 flip! Choose false (stop flipping) to lock in 1 win (3 damage to Hill Giant).
                    engine.apply(player, PlayerAction::YesNo(false)).unwrap();
                    found_win = true;
                    break;
                }
                other => panic!("unexpected pending: {other:?}"),
            }
        }
        if found_win {
            pass_until(&mut engine, stack_is_empty);
            assert!(
                on_battlefield(&engine, p1, HILL_GIANT).is_none(),
                "1 win deals 3 lethal damage to target Hill Giant"
            );
            assert_eq!(engine.state().players[1].life, 20);
            break;
        }
    }
    assert!(found_win, "found a seed where first flip was won");
}

fn play_gambit(seed: u64, stop_after: Option<u8>) -> Option<Engine<RegistryLookup>> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, mountain())
        .hand(0, &[fiery_gambit()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, fiery_gambit());
    aim_at(&mut engine, p0, giant);

    let mut wins = 0;
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                if stack_is_empty(&engine) {
                    return stop_after.is_none().then_some(engine);
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                let target = stop_after?;
                wins += 1;
                let again = wins < target;
                engine.apply(player, PlayerAction::YesNo(again)).unwrap();
                if !again {
                    pass_until(&mut engine, stack_is_empty);
                    return Some(engine);
                }
            }
            other => panic!("unexpected Fiery Gambit choice: {other:?}"),
        }
    }
    None
}

#[test]
fn fiery_gambit_covers_two_and_three_wins_and_a_loss() {
    let engine = (1..100)
        .find_map(|seed| play_gambit(seed, None))
        .expect("loss seed");
    assert!(on_battlefield(&engine, PlayerId::new(1), HILL_GIANT).is_some());
    assert_eq!(engine.state().players[1].life, 20, "a loss has no effect");

    let engine = (1..100)
        .find_map(|seed| play_gambit(seed, Some(2)))
        .expect("two-win seed");
    assert!(on_battlefield(&engine, PlayerId::new(1), HILL_GIANT).is_none());
    assert_eq!(engine.state().players[1].life, 14);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(0)))
            .is_empty()
    );

    let engine = (1..200)
        .find_map(|seed| play_gambit(seed, Some(3)))
        .expect("three-win seed");
    let p0 = PlayerId::new(0);
    assert!(on_battlefield(&engine, PlayerId::new(1), HILL_GIANT).is_none());
    assert_eq!(engine.state().players[1].life, 14);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 9);
    assert!(
        all_on_battlefield(&engine, p0, mountain())
            .into_iter()
            .all(|land| !is_tapped(&engine, land))
    );
}
