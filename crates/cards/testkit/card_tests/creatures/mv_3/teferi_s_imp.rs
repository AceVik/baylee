//! `cards/creatures/mv_3/teferi_s_imp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn teferi_s_imp() -> CardIndex {
    card_index("e4826967-b2f9-4f32-8411-d243edf41082")
}

/// Teferi's Imp ({2}{U} 1/1 Imp):
/// "Flying; Phasing; Whenever this creature phases out, discard a card.
/// Whenever this creature phases in, draw a card."
#[test]
fn teferis_imp_discards_when_phasing_out_and_draws_when_phasing_in_plus_flying() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island()],
        &[index::GRIZZLY_BEARS],
        &[teferi_s_imp(), plains()],
        &[],
    );
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    cast_from_hand(&mut e, lk::P0, teferi_s_imp());
    lk::settle(&mut e);
    let imp = lk::obj(&e, lk::P0, teferi_s_imp());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 1);

    // Opponent's untap step does not phase it.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(
        !e.state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // P0's next untap phases Imp out -> upkeep trigger discards Plains; draw step draws 1 Forest.
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    e.apply(
        player,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    reach_main_phase(&mut e, lk::P0);
    assert!(
        e.state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(in_graveyard(&e, lk::P0, plains()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 1);

    // Following P0 untap phases Imp in -> draws 1 from trigger + 1 in draw step = 3 in hand.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        !e.state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 3);

    // Flying carries it over P1's ground blocker.
    let offer = lk::attack(&mut e, &[imp]);
    assert!(!lk::may_block(&offer, bear, imp));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
