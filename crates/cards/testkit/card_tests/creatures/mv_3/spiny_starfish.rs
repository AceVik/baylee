//! `cards/creatures/mv_3/spiny_starfish.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn spiny_starfish() -> CardIndex {
    card_index("663d6c61-e269-4f2f-9d8f-49c4c23f05d8")
}

/// Spiny Starfish ({2}{U} 0/1 Creature — Starfish):
/// - "{U}: Regenerate this creature."
#[test]
fn spiny_starfish_regenerates_from_lethal_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[spiny_starfish(), island(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let starfish = on_battlefield(&engine, p0, spiny_starfish()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, spiny_starfish(), 0);
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![starfish],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, spiny_starfish()).is_some());
    assert!(is_tapped(&engine, starfish));
    assert_eq!(engine.state().object(starfish).unwrap().damage, 0);
}

/// "{U}: Regenerate this creature" is a shield that has to be paid for: with
/// no activation, lethal damage destroys the Starfish.
#[test]
fn spiny_starfish_is_destroyed_by_lethal_damage_without_a_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[spiny_starfish(), island(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let starfish = on_battlefield(&engine, p0, spiny_starfish()).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![starfish],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, spiny_starfish()).is_none());
    assert!(in_graveyard(&engine, p0, spiny_starfish()).is_some());
}
