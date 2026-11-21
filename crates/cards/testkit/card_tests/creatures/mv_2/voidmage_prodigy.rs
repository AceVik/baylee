//! `cards/creatures/mv_2/voidmage_prodigy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn voidmage_prodigy() -> CardIndex {
    card_index("7e2cacae-2934-4ff5-81c6-dfce7a2820ef")
}

/// Voidmage Prodigy ({U}{U} 2/1 Human Wizard):
/// "{U}{U}, Sacrifice a Wizard: Counter target spell. Morph {U}"
#[test]
fn voidmage_prodigy_sacrifices_your_wizard_to_counter_target_spell() {
    let mut e = lk::game(
        SEED,
        &[voidmage_prodigy(), apprentice_wizard(), grizzly_bears()],
        &[prodigal_sorcerer(), forest(), forest()],
        &[],
        &[pearled_unicorn()],
    );
    let prodigy = lk::obj(&e, lk::P0, voidmage_prodigy());
    let my_wiz = lk::obj(&e, lk::P0, apprentice_wizard());
    let my_bear = lk::obj(&e, lk::P0, grizzly_bears());
    let opp_wiz = lk::obj(&e, lk::P1, prodigal_sorcerer());

    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::White, 3)]);
    let spell = in_hand(&e, lk::P1, pearled_unicorn()).unwrap();
    cast_with_floating(&mut e, lk::P1, pearled_unicorn());
    lk::pass_once(&mut e);

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 2)]);
    activate(&mut e, lk::P0, voidmage_prodigy(), 0);
    lk::aim(&mut e, &[spell], &[]);

    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected Wizard sacrifice choice, got {:?}", e.pending());
    };
    assert!(options.contains(&prodigy) && options.contains(&my_wiz));
    assert!(!options.contains(&my_bear) && !options.contains(&opp_wiz));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![my_wiz],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, apprentice_wizard()).is_none());
    assert!(on_battlefield(&e, lk::P0, voidmage_prodigy()).is_some());
    assert!(on_battlefield(&e, lk::P1, pearled_unicorn()).is_none());
    assert!(in_graveyard(&e, lk::P1, pearled_unicorn()).is_some());
}

#[test]
fn voidmage_prodigy_casts_face_down_and_turns_face_up_for_blue() {
    let mut e = lk::game(SEED, &[island(); 4], &[], &[voidmage_prodigy()], &[]);
    cast_from_hand(&mut e, lk::P0, voidmage_prodigy());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, voidmage_prodigy());
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
    .expect("remaining {U} pays Morph {U}");
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
