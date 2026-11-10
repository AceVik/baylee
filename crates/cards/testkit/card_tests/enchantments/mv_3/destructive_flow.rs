//! `cards/enchantments/mv_3/destructive_flow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Destructive Flow — {B}{R}{G} Enchantment
/// "At the beginning of each player's upkeep, that player sacrifices a nonbasic land of their choice."
#[test]
fn destructive_flow_forces_active_player_to_sacrifice_a_nonbasic_land_at_upkeep() {
    let flow = card_index("f7544a96-46b3-4cdc-8b14-c7b6daecdabd");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[swamp(), mountain(), forest()])
        .hand(0, &[flow])
        .battlefield(1, &[island(), index::ADARKAR_WASTES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, flow);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, flow).is_some());

    let wastes_id =
        on_battlefield(&engine, p1, index::ADARKAR_WASTES).expect("Adarkar Wastes on board");

    // Advance into P1's turn; at upkeep, P1 must sacrifice a nonbasic land.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1)
            || in_graveyard(e, p1, index::ADARKAR_WASTES).is_some()
    });
    if let Pending::ChooseCards { player, .. } = engine.pending().clone()
        && player == p1
    {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![wastes_id],
                },
            )
            .unwrap();
    }
    assert!(walk_to_own_main(&mut engine, p1));

    assert!(on_battlefield(&engine, p1, index::ADARKAR_WASTES).is_none());
    assert!(in_graveyard(&engine, p1, index::ADARKAR_WASTES).is_some());
    assert!(
        on_battlefield(&engine, p1, island()).is_some(),
        "basic land is spared by Destructive Flow"
    );
}
