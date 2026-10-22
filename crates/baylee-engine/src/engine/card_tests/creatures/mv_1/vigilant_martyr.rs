//! `cards/creatures/mv_1/vigilant_martyr.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vigilant_martyr() -> CardIndex {
    card_index("122dfe9f-e0e2-4f69-9637-289c956fd54a")
}

/// Vigilant Martyr (`{W}`, `Coverage::Partial`):
/// - Sacrifice Vigilant Martyr: regenerate target creature.
/// - `{W}{W}, {T}`, Sacrifice Vigilant Martyr: counter target spell.
#[test]
fn vigilant_martyr_sacrifices_to_regenerate_creature_and_counters_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vigilant_martyr(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let martyr = on_battlefield(&engine, p0, vigilant_martyr()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Ability 0: sacrifice Vigilant Martyr to put a regeneration shield on Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: martyr,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bear).unwrap().regeneration_shields, 1);

    // Ability 1: {W}{W}, {T}, Sacrifice Vigilant Martyr to counter target spell.
    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[vigilant_martyr(), plains(), plains()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine2);
    reach_their_main_phase(&mut engine2, p1);

    let martyr2 = on_battlefield(&engine2, p0, vigilant_martyr()).unwrap();
    tap_all_mana(&mut engine2, p1);
    cast_with_floating(&mut engine2, p1, grizzly_bears());
    let spell_id = on_stack(&engine2, grizzly_bears()).unwrap();
    pass_until(
        &mut engine2,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine2, p0);
    engine2
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: martyr2,
                ability_index: 1,
            },
        )
        .unwrap();
    engine2
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell_id],
            },
        )
        .unwrap();
    pass_until(&mut engine2, stack_is_empty);

    assert!(on_battlefield(&engine2, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine2, p1, grizzly_bears()).is_some());
}
