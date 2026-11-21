//! `cards/sorceries/mv_3/rhystic_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rhystic_tutor() -> CardIndex {
    card_index("1c6da9a9-6395-460b-8150-ea46d7299dc2")
}

/// Rhystic Tutor (1c6da9a9-6395-460b-8150-ea46d7299dc2) — {2}{B} Sorcery:
/// "Unless any player pays {2}, search your library for a card, put that card
/// into your hand, then shuffle."
#[test]
fn rhystic_tutor_tutors_into_hand_unless_any_player_pays_two() {
    let mut unpaid = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[island(), island()],
        &[rhystic_tutor(), sol_ring()],
        &[],
    );
    let ring = hand_to_library_top(&mut unpaid, P0, sol_ring());
    let lib_before = library_size(&unpaid, P0);

    tap_all_mana(&mut unpaid, P0);
    cast_with_floating(&mut unpaid, P0, rhystic_tutor());
    lk::pass_once(&mut unpaid);
    tap_all_mana(&mut unpaid, P1);

    pass_until(&mut unpaid, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    while matches!(unpaid.pending(), Pending::YesNo { .. }) {
        lk::yes_no(&mut unpaid, false);
    }
    let Pending::ChooseCards { player, prompt, .. } = unpaid.pending().clone() else {
        panic!("expected SearchLibrary, got {:?}", unpaid.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    unpaid
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    lk::settle(&mut unpaid);

    assert_eq!(lk::zone_of(&unpaid, ring), Zone::Hand);
    assert_eq!(library_size(&unpaid, P0), lib_before - 1);
    assert_eq!(lk::pool_total(&unpaid, P1), 2, "P1 declined to pay {{2}}");

    // When P1 pays {2}, the library search is prevented.
    let mut paid = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[island(), island()],
        &[rhystic_tutor(), sol_ring()],
        &[],
    );
    let ring2 = hand_to_library_top(&mut paid, P0, sol_ring());
    let lib_paid = library_size(&paid, P0);

    tap_all_mana(&mut paid, P0);
    cast_with_floating(&mut paid, P0, rhystic_tutor());
    lk::pass_once(&mut paid);
    tap_all_mana(&mut paid, P1);

    pass_until(&mut paid, |e| matches!(e.pending(), Pending::YesNo { .. }));
    while let Pending::YesNo { player, .. } = paid.pending().clone() {
        lk::yes_no(&mut paid, player == P1);
    }
    lk::settle(&mut paid);

    assert_eq!(lk::pool_total(&paid, P1), 0, "P1 paid {{2}}");
    assert_eq!(lk::zone_of(&paid, ring2), Zone::Library);
    assert_eq!(library_size(&paid, P0), lib_paid);
}
