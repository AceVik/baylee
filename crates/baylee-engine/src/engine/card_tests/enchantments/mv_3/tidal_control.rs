//! `cards/enchantments/mv_3/tidal_control.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tidal_control() -> CardIndex {
    card_index("855e200e-5375-4aac-b1f7-5113162e7e14")
}

/// Tidal Control ({1}{U}{U} Enchantment):
/// - Cumulative upkeep {2}
/// - "Pay 2 life or {2}: Counter target red or green spell. Any player may activate this ability."
#[test]
fn tidal_control_counters_red_or_green_spells_by_paying_two_life_or_two_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[tidal_control(), island(), island(), island(), island()],
        )
        .battlefield(1, &[mountain(), forest(), forest()])
        .hand(1, &[lightning_bolt(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    // P0 pays Tidal Control's cumulative upkeep {2} on turn 1 upkeep.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let islands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == island()))
        })
        .collect();
    for &id in &islands[..2] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    tap_all_mana(&mut engine, p1);

    // P1 casts Lightning Bolt (red); P0 pays 2 life (ability 1) to counter it.
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, p0, tidal_control(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 18);
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());

    // P1 casts Grizzly Bears (green); P0 pays {2} (ability 2) to counter it.
    let bears = in_hand(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bears })
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tidal_control(), 2);
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

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
}
