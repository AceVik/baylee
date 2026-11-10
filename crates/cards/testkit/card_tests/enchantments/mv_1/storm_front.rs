//! `cards/enchantments/mv_1/storm_front.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn storm_front() -> CardIndex {
    card_index("3c8044b7-3999-4825-8448-0a1a325f1b48")
}

/// "{G}{G}: Tap target creature with flying." The flier is the only target.
#[test]
fn storm_front_taps_a_flier_and_offers_only_fliers() {
    let mut e = table(
        7181,
        &[],
        &[storm_front()],
        &[index::WIND_DRAKE, index::GRIZZLY_BEARS],
    );
    let drake = on_battlefield(&e, P1, index::WIND_DRAKE).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, P0, storm_front(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("target question expected, got {:?}", e.pending())
    };
    assert!(options.contains(&drake));
    assert!(!options.contains(&bears), "a ground creature is no target");
    drive(&mut e, &[drake], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, drake));
    assert!(!is_tapped(&e, bears));
}
