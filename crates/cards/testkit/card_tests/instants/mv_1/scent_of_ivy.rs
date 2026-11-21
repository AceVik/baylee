//! `cards/instants/mv_1/scent_of_ivy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b6db81e5-b7ca-45dc-a72d-1ff12935a3e4"
fn scent_of_ivy() -> CardIndex {
    card_index("b6db81e5-b7ca-45dc-a72d-1ff12935a3e4")
}

/// Scent of Ivy ({G}): offers only green cards in hand (excluding non-green
/// spells and colorless lands), pumps target creature by +X/+X until EOT, and
/// wears off on the next turn.
#[test]
fn scent_of_ivy_reveals_only_green_cards_and_pumps_by_count_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[],
        &[
            scent_of_ivy(),
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            lightning_bolt(),
            forest(),
        ],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let green_bear = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let green_elf = in_hand(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    let red_bolt = in_hand(&engine, P0, lightning_bolt()).unwrap();
    let land = in_hand(&engine, P0, forest()).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, P0, scent_of_ivy());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected reveal choice, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(min, 0, "any number allows zero");
    assert!(options.contains(&green_bear) && options.contains(&green_elf));
    assert!(
        !options.contains(&red_bolt) && !options.contains(&land),
        "only green cards are offered: {options:?}"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![green_bear, green_elf],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        pt(&engine, bears),
        (4, 4),
        "+2/+2 for two revealed green cards"
    );
    assert_eq!(pt(&engine, lions), (2, 1), "bystander untouched");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "pump wears off at end of turn");
}

/// Revealing zero green cards gives +0/+0.
#[test]
fn scent_of_ivy_revealing_zero_gives_zero_bonus() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[scent_of_ivy(), index::LLANOWAR_ELVES],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, P0, scent_of_ivy());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, bears), (2, 2), "zero revealed means +0/+0");
}
