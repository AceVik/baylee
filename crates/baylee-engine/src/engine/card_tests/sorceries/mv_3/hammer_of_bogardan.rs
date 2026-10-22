//! `cards/sorceries/mv_3/hammer_of_bogardan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HAMMER_OF_BOGARDAN;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Hammer of Bogardan (28fd5df7-b584-49d0-9479-302b6c2c38f0) — {1}{R}{R} Sorcery:
/// "Hammer of Bogardan deals 3 damage to any target. {2}{R}{R}{R}: Return
/// Hammer of Bogardan from your graveyard to your hand. Activate only during
/// your upkeep."
#[test]
fn hammer_of_bogardan_deals_three_damage_to_any_target() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[HAMMER_OF_BOGARDAN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HAMMER_OF_BOGARDAN);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}
