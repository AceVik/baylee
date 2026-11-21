//! `cards/creatures/mv_3/hunting_cheetah.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::zone::ZonePosition;

fn hunting_cheetah() -> CardIndex {
    card_index("057e1cfc-d528-49e1-a386-9fb1edc1d135")
}

/// Hunting Cheetah ({2}{G} 2/3 Cat):
/// "Whenever this creature deals damage to an opponent, you may search your
/// library for a Forest card, reveal that card, put it into your hand, then shuffle."
#[test]
fn hunting_cheetah_triggers_only_on_opponent_damage_and_may_tutor_a_forest_card() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[hunting_cheetah()])
        .battlefield(1, &[pearled_unicorn()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, lk::P0);

    // Put the single Forest on the bottom of P0's otherwise all-Plains library.
    let one_forest = in_hand(&e, lk::P0, forest()).unwrap();
    e.dev_state_mut(lk::P0)
        .unwrap()
        .move_object(
            one_forest,
            ZoneLocation::Library(lk::P0),
            ZonePosition::Bottom,
            baylee_engine::event::Cause::Setup,
        )
        .unwrap();
    e.refresh_offer();

    let cheetah = lk::obj(&e, lk::P0, hunting_cheetah());
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    // Blocked by Unicorn: deals combat damage only to a creature -> no trigger.
    let _ = lk::attack(&mut e, &[cheetah]);
    lk::block(&mut e, &[(unicorn, cheetah)]);
    lk::through_combat(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 0);

    // Unblocked next turn: deals damage to P1 -> accept MayDo and find the Forest.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[cheetah]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![one_forest]);
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![one_forest],
        },
    )
    .unwrap();
    lk::through_combat(&mut e);
    assert!(in_hand(&e, lk::P0, forest()).is_some());
}
