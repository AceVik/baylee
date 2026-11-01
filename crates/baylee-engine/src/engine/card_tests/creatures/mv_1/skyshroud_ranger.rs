//! `cards/creatures/mv_1/skyshroud_ranger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn skyshroud_ranger() -> CardIndex {
    card_index("8f47a230-7500-4afb-97cd-dbd444c307c8")
}

/// `Skyshroud Ranger` ({G}, 1/1 Creature — Elf Ranger, `Coverage::Implemented`):
/// `{T}: You may put a land card from your hand onto the battlefield. Activate
/// only as a sorcery.`
#[test]
fn skyshroud_ranger_taps_at_sorcery_speed_to_put_a_land_from_hand_onto_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skyshroud_ranger()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let land_in_hand = in_hand(&engine, p0, forest()).unwrap();
    activate(&mut engine, p0, skyshroud_ranger(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, forest()).is_some());
}

#[test]
fn ranger_is_sorcery_only_and_may_decline_but_cannot_put_a_creature() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[skyshroud_ranger()],
        &[],
        &[forest(), index::GRIZZLY_BEARS],
        &[],
    );
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, skyshroud_ranger(), p0, 0));
    reach_their_main_phase(&mut e, p0);
    activate(&mut e, p0, skyshroud_ranger(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let creature = in_hand(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature]
            }
        )
        .is_err()
    );
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut e);
    assert!(in_hand(&e, p0, forest()).is_some());
}
