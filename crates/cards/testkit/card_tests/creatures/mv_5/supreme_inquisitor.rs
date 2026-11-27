//! `cards/creatures/mv_5/supreme_inquisitor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn supreme_inquisitor() -> CardIndex {
    card_index("4460962e-c9cd-4cfd-b227-4278ee636206")
}

/// Supreme Inquisitor requires tapping five untapped Wizards you control, and
/// exiles up to five cards from the targeted player's library.
#[test]
fn supreme_inquisitor_taps_five_wizards_and_exiles_up_to_five_cards() {
    let mut e = lk::game(
        SEED,
        &[
            supreme_inquisitor(),
            apprentice_wizard(),
            prodigal_sorcerer(),
            apprentice_wizard(),
        ],
        &[],
        &[apprentice_wizard()],
        &[],
    );
    assert!(
        !lk::offered(&e, supreme_inquisitor(), lk::P0, 0),
        "four untapped Wizards is not enough for Tap five untapped Wizards"
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, lk::P0, apprentice_wizard());
    lk::settle(&mut e);

    assert!(
        lk::offered(&e, supreme_inquisitor(), lk::P0, 0),
        "five untapped Wizards (including summoning-sick one) unlocks activation"
    );
    let p1_lib_before = library_size(&e, lk::P1);

    activate(&mut e, lk::P0, supreme_inquisitor(), 0);
    lk::aim(&mut e, &[], &[lk::P1]);

    let Pending::ChooseCards {
        min, max, options, ..
    } = e.pending().clone()
    else {
        panic!("expected TapN Wizard selection, got {:?}", e.pending());
    };
    assert_eq!((min, max), (5, 5));
    assert_eq!(options.len(), 5);
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: options.clone(),
        },
    )
    .unwrap();

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        min,
        max,
        options: lib_opts,
        ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (0, 5));
    let picks: Vec<_> = lib_opts.into_iter().take(5).collect();
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: picks.clone(),
        },
    )
    .unwrap();
    lk::settle(&mut e);

    for wiz in options {
        assert!(lk::tapped(&e, wiz));
    }
    assert_eq!(library_size(&e, lk::P1), p1_lib_before - 5);
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(lk::P1)).len(), 5);
}
