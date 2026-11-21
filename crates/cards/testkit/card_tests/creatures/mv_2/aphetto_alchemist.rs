//! `cards/creatures/mv_2/aphetto_alchemist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn aphetto_alchemist() -> CardIndex {
    card_index("867aa0b7-b814-4d33-a677-f6368c8e93c0")
}

/// Aphetto Alchemist ({1}{U} 1/2 Human Wizard):
/// "{T}: Untap target artifact or creature. Morph {U}"
#[test]
fn aphetto_alchemist_untaps_target_artifact_or_creature_but_not_land() {
    let mut e = lk::game(
        SEED,
        &[aphetto_alchemist(), sol_ring(), grizzly_bears(), island()],
        &[],
        &[],
        &[],
    );
    let alch = lk::obj(&e, lk::P0, aphetto_alchemist());
    let ring = lk::obj(&e, lk::P0, sol_ring());
    let bear = lk::obj(&e, lk::P0, grizzly_bears());
    let isl = lk::obj(&e, lk::P0, island());

    tap_mana_where(&mut e, lk::P0, |id| id == ring || id == isl);
    assert!(lk::tapped(&e, ring) && lk::tapped(&e, isl));

    activate(&mut e, lk::P0, aphetto_alchemist(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice, got {:?}", e.pending());
    };
    assert!(options.contains(&ring) && options.contains(&bear) && options.contains(&alch));
    assert!(
        !options.contains(&isl),
        "noncreature land is not a legal target"
    );
    lk::aim(&mut e, &[ring], &[]);
    lk::settle(&mut e);

    assert!(lk::tapped(&e, alch));
    assert!(!lk::tapped(&e, ring), "Sol Ring was untapped");
}

#[test]
fn aphetto_alchemist_casts_face_down_and_turns_face_up_for_blue() {
    let mut e = lk::game(SEED, &[island(); 4], &[], &[aphetto_alchemist()], &[]);
    cast_from_hand(&mut e, lk::P0, aphetto_alchemist());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, aphetto_alchemist());
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
    assert_eq!(pt(&e, id), (1, 2));
}
