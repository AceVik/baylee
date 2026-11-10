//! `cards/creatures/mv_3/revered_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "e8d18dc7-f44b-47f6-8400-3f563c2f19b7"
fn revered_elder() -> CardIndex {
    card_index("e8d18dc7-f44b-47f6-8400-3f563c2f19b7")
}

/// `Revered Elder` (`Coverage::Generated`):
/// "{1}: Prevent the next 1 damage that would be dealt to this creature this turn."
///
/// Verifies that activating `Revered Elder`'s `{1}` ability creates a 1-damage
/// prevention shield on itself for the turn, allowing the 1/2 creature to
/// survive `Shock` (2 damage).
#[test]
fn revered_elder_prevents_the_next_one_damage_to_itself_per_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9509, plains())
        .battlefield(0, &[revered_elder(), plains(), mountain()])
        .hand(0, &[index::SHOCK])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let elder = on_battlefield(&engine, p0, revered_elder()).expect("P0 Revered Elder");
    let plains_id = on_battlefield(&engine, p0, plains()).expect("P0 Plains");
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("P0 Mountain");

    // Activate `{1}: Prevent the next 1 damage that would be dealt to this creature this turn.`
    tap_mana_where(&mut engine, p0, |id| id == plains_id);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elder,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Shock (2 damage) at Revered Elder (1/2) -> 1 damage prevented, 1 marked, survives!
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    cast_with_floating(&mut engine, p0, index::SHOCK);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elder],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, revered_elder()).is_some(),
        "1/2 Revered Elder survives 2 damage from Shock with a 1-damage prevention shield"
    );
    assert_eq!(
        engine.state().arena.get(elder).unwrap().damage,
        1,
        "1 of the 2 damage was prevented and 1 was dealt"
    );
}
