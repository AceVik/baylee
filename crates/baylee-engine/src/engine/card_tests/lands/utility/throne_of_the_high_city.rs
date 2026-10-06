//! `cards/lands/utility/throne_of_the_high_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Throne of the High City: "{4}, {T}, Sacrifice this land: You become the monarch."
/// Four Forests pay the generic cost to activate Throne of the High City.
/// The land is sacrificed as a cost, and upon resolution, its controller becomes the monarch.
#[test]
fn throne_of_the_high_city_sacrifices_to_make_controller_monarch() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(66, forest())
        .battlefield(
            0,
            &[
                throne_of_the_high_city(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().monarch, None);
    let throne = on_battlefield(&engine, p0, throne_of_the_high_city()).expect("Throne deployed");
    tap_mana_except(&mut engine, p0, throne);

    activate(&mut engine, p0, throne_of_the_high_city(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().monarch, Some(p0), "p0 became the monarch");
    assert!(on_battlefield(&engine, p0, throne_of_the_high_city()).is_none());
    assert!(in_graveyard(&engine, p0, throne_of_the_high_city()).is_some());
}
