//! `cards/creatures/mv_1/birchlore_rangers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn birchlore_rangers() -> CardIndex {
    card_index("667c5676-7a18-4fa9-b965-b26797538e23")
}

/// Birchlore Rangers ({G} 1/1 Elf Druid Ranger):
/// "Tap two untapped Elves you control: Add one mana of any color."
#[test]
fn birchlore_rangers_taps_two_untapped_elves_you_control_for_any_color() {
    let mut e = lk::game(
        SEED,
        &[birchlore_rangers(), grizzly_bears()],
        &[llanowar_elves()],
        &[fyndhorn_elves()],
        &[],
    );
    assert!(
        !lk::offered(&e, birchlore_rangers(), lk::P0, 0),
        "needs two untapped Elves you control; opponent's Elf and your Bear do not count"
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, lk::P0, fyndhorn_elves());
    lk::settle(&mut e);

    let rangers = lk::obj(&e, lk::P0, birchlore_rangers());
    let fyndhorn = lk::obj(&e, lk::P0, fyndhorn_elves());
    assert!(
        lk::offered(&e, birchlore_rangers(), lk::P0, 0),
        "a summoning-sick Elf may still be tapped for a non-{{T}} TapN cost"
    );

    activate(&mut e, lk::P0, birchlore_rangers(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected TapN Elf selection, got {:?}", e.pending());
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&rangers) && options.contains(&fyndhorn));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![rangers, fyndhorn],
        },
    )
    .unwrap();
    e.apply(lk::P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert!(lk::tapped(&e, rangers) && lk::tapped(&e, fyndhorn));
    assert_eq!(lk::pool_of(&e, lk::P0, ManaColor::Blue), 1);
}

/// "Morph {G}"
#[test]
fn birchlore_rangers_casts_face_down_and_turns_face_up_for_green() {
    let mut e = lk::game(SEED, &[forest(); 4], &[], &[birchlore_rangers()], &[]);
    cast_from_hand(&mut e, lk::P0, birchlore_rangers());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, birchlore_rangers());
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
    .expect("remaining {G} pays Morph {G}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 1));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
