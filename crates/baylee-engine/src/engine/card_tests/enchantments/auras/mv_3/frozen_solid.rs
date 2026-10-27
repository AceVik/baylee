//! `cards/enchantments/auras/mv_3/frozen_solid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn frozen_solid_stops_untapping_and_destroys_enchanted_creature_when_damaged() {
    let frozen_solid = card_index("7101f0ab-89ce-4b13-a5a8-4cabd6c8670d");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), index::PRODIGAL_SORCERER])
        .hand(0, &[frozen_solid])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(giant_id)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, frozen_solid);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to seat 1's main phase: Hill Giant stays tapped through seat 1's untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        is_tapped(&engine, giant_id),
        "enchanted Hill Giant does not untap during its controller's untap step"
    );

    // Advance back to seat 0's main phase and deal 1 damage to Hill Giant with Prodigal Sorcerer.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let sorcerer_id = on_battlefield(&engine, p0, index::PRODIGAL_SORCERER).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sorcerer_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}
