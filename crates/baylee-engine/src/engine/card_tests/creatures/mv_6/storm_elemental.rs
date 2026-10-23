//! `cards/creatures/mv_6/storm_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn storm_elemental() -> CardIndex {
    card_index("f1fee486-660e-4356-896e-671e8b675ad8")
}

/// Storm Elemental ({5}{U} 3/4 Creature — Elemental, Partial):
/// - Flying
/// - "{U}, Exile the top card of your library: Tap target creature with flying."
/// - "{U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn."
///
/// No cost exiles the top card of a library and no condition reads a snow
/// land, so the card keeps Flying and offers no ability: the first would tap
/// a flyer for {U} alone, the second would pump unconditionally. A pinned
/// limitation: this test moves when the cost and the condition can be said.
#[test]
fn storm_elemental_flies_and_offers_no_ability_without_its_exile_cost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[storm_elemental(), island(), island()])
        .battlefield(1, &[air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let storm = on_battlefield(&engine, p0, storm_elemental()).unwrap();
    let flyer = on_battlefield(&engine, p1, air_elemental()).unwrap();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    assert!(keywords(&engine, storm).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, storm), (3, 4));

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == storm),
        "neither {{U}} ability is offered"
    );
    for ability_index in [0, 1] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: storm,
                        ability_index,
                    },
                )
                .is_err()
        );
    }
    assert!(!is_tapped(&engine, flyer));
    assert_eq!(pt(&engine, storm), (3, 4));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before
    );
}
