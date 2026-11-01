//! `cards/instants/mv_3/undermine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Undermine — {U}{U}{B} Instant
/// "Counter target spell. Its controller loses 3 life."
#[test]
fn undermine_counters_target_spell_and_makes_its_controller_lose_three_life() {
    let undermine = card_index("6bcfe240-c165-49f2-a90a-be05c05498a1");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[island(), island(), swamp()])
        .hand(1, &[undermine])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, index::GRIZZLY_BEARS).expect("Grizzly Bears on stack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, undermine);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, undermine).is_some());
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 20);
}
