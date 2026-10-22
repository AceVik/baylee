//! `cards/enchantments/mv_4/thought_lash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thought_lash() -> CardIndex {
    card_index("81fe7c5e-503e-488c-b06c-1615ed38e5a3")
}

/// Thought Lash ({2}{U}{U} Enchantment):
/// - "Exile the top card of your library: Prevent the next 1 damage that would be dealt to you this turn."
#[test]
fn thought_lash_prevents_one_damage_to_you_per_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[thought_lash(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Activate Thought Lash once to prevent the next 1 damage dealt to P0 this turn.
    activate(&mut engine, p0, thought_lash(), 0);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 1 of the 3 damage from Lightning Bolt was prevented -> P0 at 18 life.
    assert_eq!(engine.state().players[0].life, 18);
}
