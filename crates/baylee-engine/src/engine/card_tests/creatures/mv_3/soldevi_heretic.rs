//! `cards/creatures/mv_3/soldevi_heretic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_heretic() -> CardIndex {
    card_index("eed353a6-757d-47f6-a118-b951ccf95600")
}

/// Soldevi Heretic ({2}{U} 2/2 Creature — Human Cleric):
/// - "{W}, {T}: Prevent the next 2 damage that would be dealt to target creature this turn. Target opponent may draw a card."
#[test]
fn soldevi_heretic_prevents_two_damage_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soldevi_heretic(), grizzly_bears(), plains(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    tap_all_mana_but(&mut engine, p0, Some(soldevi_heretic()));
    activate(&mut engine, p0, soldevi_heretic(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Lightning Bolt (3 damage) at Grizzly Bears; 2 damage is prevented so Bears survive with 1 damage.
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
}
