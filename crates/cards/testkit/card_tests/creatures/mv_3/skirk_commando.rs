//! `cards/creatures/mv_3/skirk_commando.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn skirk_commando() -> CardIndex {
    card_index("c92f1d40-0441-4ede-9363-78821ff15f88")
}

/// Skirk Commando ({1}{R}{R} 2/1 Goblin):
/// "Whenever this creature deals combat damage to a player, you may have it
/// deal 2 damage to target creature that player controls. Morph {2}{R}"
#[test]
fn skirk_commando_may_deal_two_damage_to_creature_damaged_player_controls_only() {
    let mut e = lk::game(
        SEED,
        &[skirk_commando(), pearled_unicorn()],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let commando = lk::obj(&e, lk::P0, skirk_commando());
    let my_unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let opp_bear = lk::obj(&e, lk::P1, grizzly_bears());

    let _ = lk::attack(&mut e, &[commando]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!();
    };
    assert_eq!(
        options,
        vec![opp_bear],
        "only damaged player's creature is legal"
    );
    assert!(!options.contains(&my_unicorn));
    lk::aim(&mut e, &[opp_bear], &[]);

    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::through_combat(&mut e);

    assert_eq!(lk::life(&e, lk::P1), 18);
    assert!(on_battlefield(&e, lk::P1, grizzly_bears()).is_none());
}

#[test]
fn skirk_commando_casts_face_down_and_turns_face_up_for_two_and_red() {
    let mut e = lk::game(SEED, &[mountain(); 6], &[], &[skirk_commando()], &[]);
    cast_from_hand(&mut e, lk::P0, skirk_commando());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, skirk_commando());
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
    .expect("remaining {2}{R} pays Morph {2}{R}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 1));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
