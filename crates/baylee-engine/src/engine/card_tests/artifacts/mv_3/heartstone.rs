//! `cards/artifacts/mv_3/heartstone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn heartstone() -> CardIndex {
    card_index("8f8ebf49-37c5-4da0-9cee-84ad22ab5d37")
}

fn mindwarper() -> CardIndex {
    card_index("efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa")
}

/// Heartstone (`8f8ebf49-37c5-4da0-9cee-84ad22ab5d37`) — {3} Artifact (Implemented):
/// "Activated abilities of creatures cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana."
#[test]
fn heartstone_reduces_generic_activated_ability_cost_to_minimum_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[heartstone(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[mindwarper()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast Mindwarper ({2}{B}{B}) using the 4 Swamps; then untap 2 Swamps so P0 has {B}{B}
    // available, which pays Mindwarper's {2}{B} ability reduced by {1} from Heartstone.
    cast_from_hand(&mut engine, P0, mindwarper());
    pass_until(&mut engine, stack_is_empty);

    for id in all_on_battlefield(&engine, P0, swamp()).into_iter().take(2) {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .remove(Status::TAPPED);
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, mindwarper(), 0);
    assert!(matches!(engine.pending(), Pending::ChooseTargets { player, .. } if *player == P0));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![lk::P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, lk::P1);
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::pool_total(&engine, P0), 0);
    assert!(in_graveyard(&engine, lk::P1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn global_creature_discount_never_makes_one_mana_free_and_leaves_artifacts_unchanged() {
    let ants = card_index("87bc6be7-4734-4a11-806e-e9e4f5616fe0");
    let mut e = lk::game(
        SEED,
        &[heartstone(), heartstone(), ants, index::BULLWHIP],
        &[ants],
        &[],
        &[],
    );
    assert!(!lk::offered(&e, ants, P0, 0), "minimum one mana");
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    assert!(lk::offered(&e, ants, P0, 0));
    assert!(
        !lk::offered(&e, index::BULLWHIP, P0, 0),
        "artifact still costs two"
    );
    lk::use_ability(&mut e, P0, ants, 0, &[], &[]);
    let mine = lk::obj(&e, P0, ants);
    assert_eq!(pt(&e, mine), (1, 2));
    assert_eq!(lk::pool_total(&e, P0), 0);
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Colorless, 1)]);
    lk::use_ability(&mut e, lk::P1, ants, 0, &[], &[]);
    let theirs = lk::obj(&e, lk::P1, ants);
    assert_eq!(pt(&e, theirs), (1, 2));
    assert_eq!(lk::pool_total(&e, lk::P1), 0);
}
