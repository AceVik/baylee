//! `cards/1999/src/uds/momentum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn momentum() -> CardIndex {
    card_index("80875914-df4c-4b05-8c4d-658ff477c956")
}

/// Optional growth counters added on your upkeep boost only the enchanted
/// creature by +1/+1 each; declining adds nothing, and opponent upkeeps do not
/// trigger.
#[test]
fn momentum_optional_upkeep_counters_scale_only_enchanted_creature() {
    an_aura_enchants_only_creatures(momentum(), forest());

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[momentum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, momentum());
    aim_at(&mut engine, P0, my_bears);
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, P0, momentum()).expect("Momentum on board");
    assert_eq!(counters_on(&engine, aura, counters::GROWTH), 0);
    assert_eq!(pt(&engine, my_bears), (2, 2));

    // P1's upkeep does not ask either player; reaches P1 main cleanly.
    reach_their_main_phase(&mut engine, P1);

    // P0 upkeep #1: accept Yes -> 1 growth counter, 3/3.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, aura, counters::GROWTH), 1);
    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // P0 upkeep #2: decline No -> stays 1 growth counter, 3/3.
    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, aura, counters::GROWTH), 1);
    assert_eq!(pt(&engine, my_bears), (3, 3));

    // P0 upkeep #3: accept Yes -> 2 growth counters, 4/4.
    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, aura, counters::GROWTH), 2);
    assert_eq!(pt(&engine, my_bears), (4, 4));
}

/// Even when Momentum enchants an opponent's creature, "your upkeep" tracks the
/// Aura's controller (P0), not the creature's controller (P1).
#[test]
fn momentum_on_opponents_creature_still_triggers_on_aura_controllers_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 3])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[momentum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, momentum());
    aim_at(&mut engine, P0, their_bears);
    pass_until(&mut engine, stack_is_empty);

    // Passing through P1's upkeep triggers nothing.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // P0's upkeep prompts P0 and pumps P1's enchanted Bears to 3/3.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, their_bears), (3, 3));
}
