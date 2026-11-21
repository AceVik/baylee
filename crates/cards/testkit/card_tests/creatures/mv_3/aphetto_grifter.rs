//! `cards/creatures/mv_3/aphetto_grifter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn aphetto_grifter() -> CardIndex {
    card_index("d9d7b7a9-5107-4720-9f92-b49e372372b8")
}

/// Aphetto Grifter ({2}{U} 1/1 Human Wizard):
/// "Tap two untapped Wizards you control: Tap target permanent."
#[test]
fn aphetto_grifter_taps_two_untapped_wizards_you_control_to_tap_any_permanent() {
    let mut e = lk::game(
        SEED,
        &[aphetto_grifter(), grizzly_bears()],
        &[prodigal_sorcerer(), island()],
        &[apprentice_wizard()],
        &[],
    );
    assert!(
        !lk::offered(&e, aphetto_grifter(), lk::P0, 0),
        "needs two untapped Wizards you control"
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, lk::P0, apprentice_wizard());
    lk::settle(&mut e);

    let grifter = lk::obj(&e, lk::P0, aphetto_grifter());
    let wiz = lk::obj(&e, lk::P0, apprentice_wizard());
    let opp_land = lk::obj(&e, lk::P1, island());
    assert!(lk::offered(&e, aphetto_grifter(), lk::P0, 0));

    activate(&mut e, lk::P0, aphetto_grifter(), 0);
    lk::aim(&mut e, &[opp_land], &[]);

    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected TapN Wizard choice, got {:?}", e.pending());
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&grifter) && options.contains(&wiz));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![grifter, wiz],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(lk::tapped(&e, grifter) && lk::tapped(&e, wiz));
    assert!(
        lk::tapped(&e, opp_land),
        "any permanent (including a land) may be tapped"
    );
}
