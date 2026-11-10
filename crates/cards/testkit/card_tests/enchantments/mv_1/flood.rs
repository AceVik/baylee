//! `cards/enchantments/mv_1/flood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn flood() -> CardIndex {
    card_index("e8eb2abb-daf9-43e0-b909-d96b679f71c2")
}

/// "{U}{U}: Tap target creature without flying." The flier is no target.
#[test]
fn flood_taps_a_creature_without_flying_and_refuses_fliers() {
    let mut e = table(
        7141,
        &[],
        &[flood()],
        &[index::WIND_DRAKE, index::GRIZZLY_BEARS],
    );
    let drake = on_battlefield(&e, P1, index::WIND_DRAKE).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    assert!(!is_tapped(&e, bears));
    activate(&mut e, P0, flood(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("target question expected, got {:?}", e.pending())
    };
    assert!(options.contains(&bears));
    assert!(!options.contains(&drake), "a flier is no target");
    drive(&mut e, &[bears], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, bears));
    assert!(!is_tapped(&e, drake));
}
