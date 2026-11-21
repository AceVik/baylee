//! `cards/creatures/mv_3/shimmering_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn shimmering_efreet() -> CardIndex {
    card_index("34f52d00-301d-4065-a86b-8393b4c6e99d")
}

/// Shimmering Efreet ({2}{U} 2/2 Efreet):
/// "Flying; Phasing; Whenever this creature phases in, target creature phases out."
#[test]
fn shimmering_efreet_phases_out_quietly_then_phases_in_and_phases_out_target_creature() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island()],
        &[index::GRIZZLY_BEARS],
        &[shimmering_efreet()],
        &[],
    );
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    cast_from_hand(&mut e, lk::P0, shimmering_efreet());
    lk::settle(&mut e);
    let efreet = lk::obj(&e, lk::P0, shimmering_efreet());

    // First P0 untap: Efreet phases out; Bear stays phased in.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        e.state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        !e.state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // Second P0 untap: Efreet phases in and triggers to phase out target creature.
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);

    assert!(
        !e.state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        e.state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

#[test]
fn shimmering_efreet_must_target_itself_when_it_phases_in_alone() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island()],
        &[],
        &[shimmering_efreet()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, shimmering_efreet());
    lk::settle(&mut e);
    let efreet = lk::obj(&e, lk::P0, shimmering_efreet());

    // Phase out on next P0 untap.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        e.state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // Phase in alone on following P0 untap: mandatory target offers only Efreet itself.
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, min, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(min, 1);
    assert_eq!(options, vec![efreet]);
    lk::aim(&mut e, &[efreet], &[]);
    lk::settle(&mut e);
    assert!(
        e.state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
