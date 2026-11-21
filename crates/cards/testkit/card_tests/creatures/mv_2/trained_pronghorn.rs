//! `cards/creatures/mv_2/trained_pronghorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn trained_pronghorn() -> CardIndex {
    card_index("22528380-4f47-4bd9-a1d4-79f700cba4f5")
}

/// Trained Pronghorn ({1}{W} 1/1 Antelope):
/// "Discard a card: Prevent all damage that would be dealt to this creature this turn."
#[test]
fn trained_pronghorn_discards_a_card_to_prevent_all_damage_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[trained_pronghorn()],
        &[grizzly_bears()],
        &[plains(), lightning_bolt()],
        &[lightning_bolt()],
    );
    let horn = lk::obj(&e, lk::P0, trained_pronghorn());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    let pln = in_hand(&e, lk::P0, plains()).unwrap();

    activate(&mut e, lk::P0, trained_pronghorn(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected discard cost choice, got {:?}", e.pending());
    };
    assert!(options.contains(&pln));
    e.apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![pln] })
        .unwrap();
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, plains()).is_some());

    // Spell damage AND combat damage in the same turn are both prevented.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[horn], &[]);
    assert!(on_battlefield(&e, lk::P0, trained_pronghorn()).is_some());
    assert_eq!(lk::damage_on(&e, horn), 0);

    let _ = lk::attack(&mut e, &[horn]);
    lk::block(&mut e, &[(bear, horn)]);
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, trained_pronghorn()).is_some());
    assert_eq!(lk::damage_on(&e, horn), 0);
    assert!(
        !lk::offered(&e, trained_pronghorn(), lk::P0, 0),
        "empty hand cannot pay Discard a card"
    );

    // Next turn, shield has expired: opponent's Bolt destroys Trained Pronghorn.
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P1, lightning_bolt(), &[horn], &[]);
    assert!(on_battlefield(&e, lk::P0, trained_pronghorn()).is_none());
}
