//! `cards/enchantments/mv_5/noble_purpose.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "84528204-faf2-45cb-bce0-365b8adf5339"
fn noble_purpose() -> CardIndex {
    card_index("84528204-faf2-45cb-bce0-365b8adf5339")
}

/// `Noble Purpose` (`Coverage::Generated`):
/// "Whenever a creature you control deals combat damage, you gain that much life."
///
/// Verifies that `Noble Purpose` triggers when a creature controlled by its
/// controller deals combat damage (gaining that much life), and does not trigger
/// on noncombat damage or when an opponent's creature deals combat damage.
#[test]
fn noble_purpose_gains_life_equal_to_combat_damage_dealt_by_controlled_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9503, plains())
        .battlefield(
            0,
            &[noble_purpose(), grizzly_bears(), index::PRODIGAL_SORCERER],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear0 = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    let tim = on_battlefield(&engine, p0, index::PRODIGAL_SORCERER).expect("P0 Prodigal Sorcerer");
    let bear1 = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");

    // Noncombat damage from Prodigal Sorcerer does NOT trigger Noble Purpose.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "noncombat damage does not trigger Noble Purpose"
    );

    // P0 attacks with Grizzly Bears (2/2) unblocked -> deals 2 combat damage -> P0 gains 2 life.
    declare_attack(&mut engine, p0, &[bear0], p1);
    finish_combat(&mut engine);

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "Noble Purpose gains 2 life when P0's 2/2 deals 2 combat damage"
    );

    // On P1's turn, P1 attacks with Grizzly Bears (2/2) unblocked -> P0 does not gain life.
    reach_their_main_phase(&mut engine, p1);
    declare_attack(&mut engine, p1, &[bear1], p0);
    finish_combat(&mut engine);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "opponent's creature dealing combat damage does not trigger P0's Noble Purpose"
    );
}
