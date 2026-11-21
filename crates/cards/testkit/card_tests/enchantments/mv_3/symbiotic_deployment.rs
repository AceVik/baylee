//! `cards/enchantments/mv_3/symbiotic_deployment.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn symbiotic_deployment() -> CardIndex {
    card_index("a6872d7a-b647-4baf-bd1e-06707ccbb71a")
}

/// Skips only your draw step and offers "{1}, Tap two untapped creatures
/// you control: Draw a card."
#[test]
fn symbiotic_deployment_skips_your_draw_step_and_taps_two_creatures_to_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            symbiotic_deployment(),
            llanowar_elves(),
            llanowar_elves(),
            grizzly_bears(),
        ],
        &[],
        &[],
        &[],
    );
    let my_elves = all_on_battlefield(&e, p0, llanowar_elves());

    // P1 still draws normally on P1's turn; P0 skips draw step on P0's turn.
    let p1_lib = library_size(&e, p1);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(library_size(&e, p1), p1_lib - 1);

    let p0_lib = library_size(&e, p0);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(library_size(&e, p0), p0_lib);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);

    // Pay {1} and tap 2 Elves -> draw 1 card; with 1 Bear left, cannot activate again.
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
    activate(&mut e, p0, symbiotic_deployment(), 1);
    e.apply(p0, PlayerAction::ChooseObjects { objects: my_elves })
        .unwrap();
    lk::settle(&mut e);
    assert_eq!(library_size(&e, p0), p0_lib - 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);
    assert!(!lk::offered(&e, symbiotic_deployment(), p0, 1));
}
