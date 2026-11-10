//! `cards/creatures/mv_3/seasoned_tactician.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn seasoned_tactician() -> CardIndex {
    card_index("d7c2f27c-8c82-4283-a17d-f979ee636542")
}

/// Seasoned Tactician ({2}{W} 1/3 Creature — Human Advisor, Partial):
/// "{3}, Exile the top four cards of your library: The next time a source of
/// your choice would deal damage to you this turn, prevent that damage."
/// No cost exiles the top cards of a library, and the shield for {3} alone
/// would be cheaper than printed, so the card offers no ability. A pinned
/// limitation: this test moves when that cost can be said.
#[test]
fn seasoned_tactician_offers_no_ability_instead_of_a_shield_without_its_exile_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[seasoned_tactician(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tactician = on_battlefield(&engine, p0, seasoned_tactician()).unwrap();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, _)| *source == tactician),
        "the {{3}} ability is not offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: tactician,
                    ability_index: 0,
                },
            )
            .is_err()
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before,
        "nothing is exiled from the library"
    );
}
