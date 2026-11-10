//! `cards/lands/utility/nephalia_drownyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nephalia Drownyard: "{1}{U}{B}, {T}: Target player mills three cards."
///
/// A player is the only thing this can be pointed at, and the *object* list
/// for such a spec is empty by construction — so an ability `LegalActions`
/// had just offered was refused by `apply` with "no legal targets", the
/// disagreement between two probes this engine treats as the worst kind.
/// Three implemented lands print it and all three were dead: the Drownyard,
/// Duskmantle, House of Shadow and Orzhova, the Church of Deals.
#[test]
fn a_land_that_mills_target_player_can_be_activated() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(203, forest())
        .battlefield(0, &[nephalia_drownyard(), island(), island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drownyard = on_battlefield(&engine, p0, nephalia_drownyard()).expect("the Drownyard");
    // Its own tap is part of the ability's cost, so it is the one land that
    // must not be spent on the mana.
    tap_mana_except(&mut engine, p0, drownyard);
    let before = library_size(&engine, p1);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: drownyard,
                ability_index: 1,
            },
        )
        .expect("two Islands and a Swamp pay {1}{U}{B}");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the mill asks whose library: {:?}", engine.pending())
    };
    assert!(options.is_empty(), "a seat is not an object");
    assert!(
        player_options.contains(&p1),
        "and the other seat is one of the answers"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert_eq!(
        library_size(&engine, p1),
        before - 3,
        "three cards off the top of the library that was named"
    );
}
