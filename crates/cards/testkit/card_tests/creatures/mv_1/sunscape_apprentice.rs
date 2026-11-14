//! `cards/creatures/mv_1/sunscape_apprentice.rs`, played.
//!
//! Oracle:
//! - "{G}, {T}: Target creature gets +1/+1 until end of turn."
//! - "{U}, {T}: Put target creature you control on top of its owner's library."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sunscape_apprentice() -> CardIndex {
    card_index("4ab1411e-b255-4a6e-b81a-5de7b2d4ea12")
}

#[test]
fn sunscape_apprentice_pumps_any_creature_and_bounces_own_creature_to_library_top() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                island(),
                plains(),
                sunscape_apprentice(),
                sunscape_apprentice(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[sunscape_apprentice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let copies = all_on_battlefield(&engine, P0, sunscape_apprentice());
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    assert_eq!(pt(&engine, copies[0]), (1, 1));

    tap_all_mana(&mut engine, P0);

    // Ability 0: "{G}, {T}: Target creature gets +1/+1 until end of turn."
    activate(&mut engine, P0, sunscape_apprentice(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(options.contains(&lions), "any creature is a legal target");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![lions],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (3, 2));

    // Ability 1: "{U}, {T}: Put target creature you control on top of its owner's library."
    let p0_lib_before = library_size(&engine, P0);
    activate(&mut engine, P0, sunscape_apprentice(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&lions),
        "must only target a creature you control"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, P0), p0_lib_before + 1);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        index::GRIZZLY_BEARS
    );

    // Pump on Savannah Lions expires at end of turn.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, lions), (2, 1));
}
