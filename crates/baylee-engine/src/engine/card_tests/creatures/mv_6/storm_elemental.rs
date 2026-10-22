//! `cards/creatures/mv_6/storm_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn storm_elemental() -> CardIndex {
    card_index("f1fee486-660e-4356-896e-671e8b675ad8")
}

/// Storm Elemental ({5}{U} 3/4 Creature — Elemental):
/// - Flying
/// - "{U}, Exile the top card of your library: Tap target creature with flying."
/// - "{U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn."
#[test]
fn storm_elemental_taps_target_flyer_and_pumps_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[storm_elemental(), island(), island()])
        .battlefield(1, &[air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let storm = on_battlefield(&engine, p0, storm_elemental()).unwrap();
    let flyer = on_battlefield(&engine, p1, air_elemental()).unwrap();
    assert!(keywords(&engine, storm).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    // Ability 0: tap target creature with flying.
    activate(&mut engine, p0, storm_elemental(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![flyer],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, flyer));

    // Ability 1: +1/+1 until end of turn.
    activate(&mut engine, p0, storm_elemental(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, storm), (4, 5));
}
