//! `cards/creatures/mv_1/elvish_pioneer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn elvish_pioneer() -> CardIndex {
    card_index("5f4138e9-5471-467f-a15e-89e9e7929f0c")
}

/// Elvish Pioneer ({G} 1/1 Elf Druid):
/// "When this creature enters, you may put a basic land card from your hand
/// onto the battlefield tapped."
#[test]
fn elvish_pioneer_puts_only_a_basic_land_from_hand_onto_battlefield_tapped() {
    let mut e = lk::game(
        SEED,
        &[forest()],
        &[],
        &[elvish_pioneer(), island(), badlands(), grizzly_bears()],
        &[],
    );
    let isl_hand = in_hand(&e, lk::P0, island()).unwrap();

    cast_from_hand(&mut e, lk::P0, elvish_pioneer());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);

    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!(
            "expected basic land choice from hand, got {:?}",
            e.pending()
        );
    };
    assert_eq!(player, lk::P0);
    assert_eq!(
        options,
        vec![isl_hand],
        "only basic land cards in hand are offered (not Badlands or Grizzly Bears)"
    );
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![isl_hand],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    let isl_bf = lk::obj(&e, lk::P0, island());
    assert!(lk::tapped(&e, isl_bf), "basic land enters tapped");
    assert!(in_hand(&e, lk::P0, badlands()).is_some());
}

#[test]
fn elvish_pioneer_etb_may_be_declined_leaving_basic_land_in_hand() {
    let mut e = lk::game(SEED, &[forest()], &[], &[elvish_pioneer(), plains()], &[]);
    cast_from_hand(&mut e, lk::P0, elvish_pioneer());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, plains()).is_none());
    assert!(in_hand(&e, lk::P0, plains()).is_some());
}
