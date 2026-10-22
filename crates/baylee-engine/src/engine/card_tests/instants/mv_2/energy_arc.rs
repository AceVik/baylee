//! `cards/instants/mv_2/energy_arc.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn energy_arc() -> CardIndex {
    card_index("959bcf99-3c9a-4f99-98b2-13514d3dac16")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `Energy Arc` ({W}{U}, Instant, `Coverage::Partial`) untaps any number of
/// target creatures.
#[test]
fn energy_arc_untaps_multiple_target_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), grizzly_bears(), hill_giant()])
        .hand(0, &[energy_arc()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    engine.dev_state_mut(p0).unwrap().set_tapped(bears, true);
    engine.dev_state_mut(p0).unwrap().set_tapped(giant, true);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, energy_arc());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears));
    assert!(!is_tapped(&engine, giant));
}
