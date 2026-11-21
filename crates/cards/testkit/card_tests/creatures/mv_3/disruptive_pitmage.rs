//! `cards/creatures/mv_3/disruptive_pitmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn disruptive_pitmage() -> CardIndex {
    card_index("c7a60f81-f7cb-4fd7-b422-963290964069")
}

/// Disruptive Pitmage ({2}{U} 1/1 Human Wizard):
/// "{T}: Counter target spell unless its controller pays {1}. Morph {U}"
#[test]
fn disruptive_pitmage_counters_spell_unless_controller_pays_one() {
    // Controller declines to pay {1} -> spell is countered.
    let mut e = lk::game(SEED, &[disruptive_pitmage()], &[], &[], &[grizzly_bears()]);
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Green, 3)]);
    let bear_spell = in_hand(&e, lk::P1, grizzly_bears()).unwrap();
    cast_with_floating(&mut e, lk::P1, grizzly_bears());
    lk::pass_once(&mut e);

    activate(&mut e, lk::P0, disruptive_pitmage(), 0);
    lk::aim(&mut e, &[bear_spell], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(lk::P1, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut e);
    assert!(on_battlefield(&e, lk::P1, grizzly_bears()).is_none());
    assert!(in_graveyard(&e, lk::P1, grizzly_bears()).is_some());
    assert_eq!(lk::pool_total(&e, lk::P1), 1);

    // Controller has {1} floating and pays -> spell resolves.
    let mut e2 = lk::game(SEED, &[disruptive_pitmage()], &[], &[], &[grizzly_bears()]);
    reach_their_main_phase(&mut e2, lk::P1);
    lk::float(&mut e2, lk::P1, &[(ManaColor::Green, 3)]);
    let bear_spell2 = in_hand(&e2, lk::P1, grizzly_bears()).unwrap();
    cast_with_floating(&mut e2, lk::P1, grizzly_bears());
    lk::pass_once(&mut e2);

    activate(&mut e2, lk::P0, disruptive_pitmage(), 0);
    lk::aim(&mut e2, &[bear_spell2], &[]);
    pass_until(&mut e2, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e2.apply(lk::P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut e2);
    assert!(on_battlefield(&e2, lk::P1, grizzly_bears()).is_some());
    assert_eq!(lk::pool_total(&e2, lk::P1), 0);
}

#[test]
fn disruptive_pitmage_casts_face_down_and_turns_face_up_for_blue() {
    let mut e = lk::game(SEED, &[island(); 4], &[], &[disruptive_pitmage()], &[]);
    cast_from_hand(&mut e, lk::P0, disruptive_pitmage());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, disruptive_pitmage());
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
    assert_eq!(pt(&e, id), (1, 1));
}
