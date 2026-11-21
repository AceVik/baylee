//! `cards/creatures/mv_3/dawning_purist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn dawning_purist() -> CardIndex {
    card_index("9e55cde0-16e6-4d32-8a37-1ef3dc812971")
}

/// Dawning Purist ({2}{W} 2/2 Human Cleric):
/// "Whenever this creature deals combat damage to a player, you may destroy
/// target enchantment that player controls. Morph {1}{W}"
#[test]
fn dawning_purist_may_destroy_enchantment_damaged_player_controls_only() {
    let mut e = lk::game(
        SEED,
        &[dawning_purist(), knighthood()],
        &[blood_rites()],
        &[],
        &[],
    );
    let purist = lk::obj(&e, lk::P0, dawning_purist());
    let my_ench = lk::obj(&e, lk::P0, knighthood());
    let opp_ench = lk::obj(&e, lk::P1, blood_rites());

    let _ = lk::attack(&mut e, &[purist]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!();
    };
    assert_eq!(
        options,
        vec![opp_ench],
        "only the damaged player's enchantment is legal"
    );
    assert!(!options.contains(&my_ench));
    lk::aim(&mut e, &[opp_ench], &[]);

    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::through_combat(&mut e);

    assert!(on_battlefield(&e, lk::P1, blood_rites()).is_none());
    assert!(in_graveyard(&e, lk::P1, blood_rites()).is_some());
}

#[test]
fn dawning_purist_casts_face_down_and_turns_face_up_for_one_and_white() {
    let mut e = lk::game(SEED, &[plains(); 5], &[], &[dawning_purist()], &[]);
    cast_from_hand(&mut e, lk::P0, dawning_purist());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, dawning_purist());
    assert!(
        e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));

    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: baylee_engine::choice::TURN_FACE_UP,
        },
    )
    .expect("remaining {1}{W} pays Morph {1}{W}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
