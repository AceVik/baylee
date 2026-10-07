//! `cards/lands/restricted/tectonic_edge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tectonic Edge: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: Destroy
/// target nonbasic land. Activate only if an opponent controls four or more
/// lands."
///
/// Both sides of the gate, because the land had been shipping **ungated** —
/// a Wasteland at one more mana — while `Condition` could not count an
/// opponent's permanents. Three lands across the table offers nothing; the
/// fourth is what turns the ability on, and it is a basic, so the count is
/// over lands and not over the nonbasic the ability then destroys.
#[test]
fn tectonic_edge_destroys_a_nonbasic_land_only_once_an_opponent_has_four() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(109, forest())
        .battlefield(0, &[tectonic_edge(), forest()])
        .battlefield(1, &[irrigated_farmland(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land =
        on_battlefield(&engine, p1, irrigated_farmland()).expect("target land deployed");
    let edge = on_battlefield(&engine, p0, tectonic_edge()).expect("the Edge stands");
    tap_mana_except(&mut engine, p0, edge);

    let offers = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        legal.abilities.contains(&(edge, 1))
    };
    assert!(
        !offers(&engine),
        "three lands across the table is not four, so the ability is not offered"
    );

    // The fourth land, and nothing else about the board changes.
    {
        let state = engine
            .dev_state_mut(p1)
            .expect("the harness may set boards up");
        let name = state.names.intern("Extra Island");
        let id = state.create_bare(
            p1,
            crate::object::ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(id).expect("just created").base_mut().types =
            baylee_core::types::TypeSet::LAND;
    }
    engine.refresh_offer();
    assert!(offers(&engine), "the fourth land is what opens the gate");

    activate(&mut engine, p0, tectonic_edge(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, tectonic_edge()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, irrigated_farmland()).is_some());
    assert!(on_battlefield(&engine, p1, irrigated_farmland()).is_none());
}
