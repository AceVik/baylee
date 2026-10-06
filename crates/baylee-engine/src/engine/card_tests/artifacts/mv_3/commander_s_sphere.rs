//! `cards/artifacts/mv_3/commander_s_sphere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Commander's Sphere` is an artifact costing `{3}` under `Coverage::Implemented`.
/// It prints "{T}: Add one mana of any color in your commander's color identity."
/// and "Sacrifice this artifact: Draw a card."
/// With a mono-blue commander like `jin_gitaxias()`, it taps to add `{U}`,
/// and can be sacrificed while tapped to draw a card.
#[test]
fn commanders_sphere_taps_for_identity_mana_and_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[jin_gitaxias()])
        .battlefield(0, &[commander_s_sphere()])
        .hand(0, &[])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphere = on_battlefield(&engine, p0, commander_s_sphere())
        .expect("Commander's Sphere is on the battlefield");

    activate(&mut engine, p0, commander_s_sphere(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "produced one blue mana matching commander's identity"
    );
    assert!(
        is_tapped(&engine, sphere),
        "sphere is tapped after mana activation"
    );

    activate(&mut engine, p0, commander_s_sphere(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, commander_s_sphere()).is_none(),
        "sphere was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, commander_s_sphere()).is_some(),
        "sphere is in graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew one card"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "blue mana is still in the pool"
    );
}
