//! `cards/sorceries/mv_3/pelakka_predation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land face enters tapped and produces black mana after untapping.
#[test]
fn pelakka_caverns_enters_tapped_and_taps_for_black_mana() {
    let (mut engine, caverns) =
        play_land_face(pelakka_predation(), 1).expect("plays as Pelakka Caverns");
    assert!(is_tapped(&engine, caverns), "Pelakka Caverns enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, caverns), "untaps on next turn");
    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);

    activate(&mut engine, p0, pelakka_predation(), 0);

    assert!(
        is_tapped(&engine, caverns),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        black_before + 1,
        "adds one black mana to the pool"
    );
}

#[test]
fn pelakka_predation_reveals_the_hand_and_discards_only_a_card_of_mana_value_at_least_three() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        930,
        &[],
        &[],
        &[pelakka_predation()],
        &[chromatic_lantern(), llanowar_elves()],
    );
    let lantern = in_hand(&engine, kit::P1, chromatic_lantern()).unwrap();
    let elf = in_hand(&engine, kit::P1, llanowar_elves()).unwrap();
    kit::float(&mut engine, kit::P0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut engine, kit::P0, pelakka_predation());
    engine
        .apply(kit::P0, PlayerAction::ChoosePlayer(kit::P1))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, kit::P0);
    assert_eq!((*min, *max), (1, 1));
    assert!(options.contains(&lantern));
    assert!(!options.contains(&elf));
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![lantern],
            },
        )
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(
        engine.state().object(lantern).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(elf).unwrap().zone, Zone::Hand);
}
