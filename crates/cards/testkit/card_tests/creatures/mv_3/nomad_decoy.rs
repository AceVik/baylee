//! `cards/creatures/mv_3/nomad_decoy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn nomad_decoy() -> CardIndex {
    card_index("73c40477-7061-47db-af8f-2a5cf6c22c9b")
}

/// Nomad Decoy ({2}{W} 1/2 Human Nomad):
/// "{W}, {T}: Tap target creature.
/// Threshold — {W}{W}, {T}: Tap two target creatures. Activate only if there
/// are seven or more cards in your graveyard."
#[test]
fn nomad_decoy_taps_one_below_threshold_and_unlocks_two_targets_at_seven_cards() {
    let mut e = lk::game(
        SEED,
        &[
            nomad_decoy(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
        ],
        &[grizzly_bears(), pearled_unicorn()],
        &[],
        &[],
    );
    let decoy = lk::obj(&e, lk::P0, nomad_decoy());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    let all_plains = all_on_battlefield(&e, lk::P0, plains());
    for &p in &all_plains[..6] {
        e.dev_state_mut(lk::P0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(lk::P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 2)]);
    assert!(lk::offered(&e, nomad_decoy(), lk::P0, 0));
    assert!(
        !lk::offered(&e, nomad_decoy(), lk::P0, 1),
        "6 cards in graveyard (< 7) cannot activate Threshold ability"
    );

    // Move 7th card into graveyard -> Threshold ability (index 1) unlocks!
    e.dev_state_mut(lk::P0)
        .unwrap()
        .move_object(
            all_plains[6],
            ZoneLocation::Graveyard(lk::P0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    assert!(lk::offered(&e, nomad_decoy(), lk::P0, 1));

    activate(&mut e, lk::P0, nomad_decoy(), 1);
    let Pending::ChooseTargets { min, max, .. } = e.pending().clone() else {
        panic!("expected 2-creature target prompt, got {:?}", e.pending());
    };
    assert_eq!((min, max), (2, 2));
    lk::aim(&mut e, &[bear, unicorn], &[]);
    lk::settle(&mut e);

    assert!(lk::tapped(&e, decoy));
    assert!(lk::tapped(&e, bear) && lk::tapped(&e, unicorn));

    // Base {W}, {T} ability taps a single target creature without Threshold.
    let mut e2 = lk::game(SEED, &[nomad_decoy()], &[grizzly_bears()], &[], &[]);
    let bear2 = lk::obj(&e2, lk::P1, grizzly_bears());
    lk::float(&mut e2, lk::P0, &[(ManaColor::White, 1)]);
    lk::use_ability(&mut e2, lk::P0, nomad_decoy(), 0, &[bear2], &[]);
    assert!(lk::tapped(&e2, bear2));
}
