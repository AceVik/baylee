//! `cards/creatures/mv_2/whipcorder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn whipcorder() -> CardIndex {
    card_index("4a4c187e-e10b-4fd2-b70b-6401bb0595c5")
}

/// Whipcorder ({W}{W} 2/2 Human Soldier Rebel):
/// "{W}, {T}: Tap target creature. Morph {W}"
#[test]
fn whipcorder_taps_target_creature_and_ignores_noncreatures() {
    let mut e = lk::game(
        SEED,
        &[whipcorder()],
        &[grizzly_bears(), plains()],
        &[],
        &[],
    );
    let corder = lk::obj(&e, lk::P0, whipcorder());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    let pln = lk::obj(&e, lk::P1, plains());

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 1)]);
    activate(&mut e, lk::P0, whipcorder(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected creature target choice, got {:?}", e.pending());
    };
    assert!(options.contains(&bear) && !options.contains(&pln));
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);

    assert!(lk::tapped(&e, corder));
    assert!(lk::tapped(&e, bear));
}

#[test]
fn whipcorder_casts_face_down_and_turns_face_up_for_white() {
    let mut e = lk::game(SEED, &[plains(); 4], &[], &[whipcorder()], &[]);
    cast_from_hand(&mut e, lk::P0, whipcorder());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, whipcorder());
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
    .expect("remaining {W} pays Morph {W}");
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
