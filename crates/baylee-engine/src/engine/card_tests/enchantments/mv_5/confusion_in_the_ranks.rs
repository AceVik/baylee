//! `cards/enchantments/mv_5/confusion_in_the_ranks.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn confusion_in_the_ranks() -> CardIndex {
    card_index("e477abb9-9b99-48a8-8c09-07b120b72540")
}

/// Confusion in the Ranks — {3}{R}{R} Enchantment
/// "Whenever an artifact, creature, or enchantment enters, its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents."
#[test]
fn confusion_in_the_ranks_exchanges_control_of_entering_creature_with_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[grizzly_bears()])
        .battlefield(0, &[confusion_in_the_ranks(), forest(), forest()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, grizzly_bears());
    // Wait until Confusion in the Ranks triggers and asks P0 for a target.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == p0),
    );
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    let bear =
        on_battlefield(&engine, p1, grizzly_bears()).expect("P1 now controls the Grizzly Bears");
    assert_eq!(engine.state().object(bear).unwrap().controller, p1);
    assert_eq!(engine.state().object(giant).unwrap().controller, p0);
}
