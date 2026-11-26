//! `cards/enchantments/mv_2/saproling_infestation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn saproling_infestation() -> CardIndex {
    card_index("dd257eb3-4112-41e5-9c7d-00d1f683dbdb")
}

fn faerie_squadron() -> CardIndex {
    card_index("109f7a7d-dc08-4c8f-8065-93c620a9194e")
}

/// Saproling Infestation ({1}{G} Enchantment):
/// "Whenever a player kicks a spell, you create a 1/1 green Saproling creature token."
#[test]
fn saproling_infestation_triggers_on_any_kicked_spell_not_unkicked() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island(), island()])
        .hand(0, &[saproling_infestation(), faerie_squadron()])
        .hand(1, &[faerie_squadron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, saproling_infestation());
    pass_until(&mut engine, stack_is_empty);

    // Unkicked spell does NOT trigger.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, faerie_squadron());
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let tokens_after_unkicked = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).is_some_and(|o| o.card.is_none()))
        .count();
    assert_eq!(tokens_after_unkicked, 0);

    // Opponent casting a kicked spell DOES create a Saproling under our control.
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, faerie_squadron());
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let saprolings = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P0
                    && o.card.is_none()
                    && engine.state().names.get(o.characteristics().name) == "Saproling"
            })
        })
        .count();
    assert_eq!(saprolings, 1);
}
