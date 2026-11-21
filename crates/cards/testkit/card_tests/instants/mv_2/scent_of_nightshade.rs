//! `cards/instants/mv_2/scent_of_nightshade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "2e88f39d-6177-42d9-b756-144219dc0961"
fn scent_of_nightshade() -> CardIndex {
    card_index("2e88f39d-6177-42d9-b756-144219dc0961")
}

/// Scent of Nightshade ({1}{B}): offers only black cards in hand (excluding
/// non-black spells and colorless Swamps) and gives target creature -X/-X EOT.
#[test]
fn scent_of_nightshade_reveals_only_black_cards_and_kills_at_zero_toughness() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[
            scent_of_nightshade(),
            index::DARK_RITUAL,
            index::DRUDGE_SKELETONS,
            lightning_bolt(),
            swamp(),
        ],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let ritual = in_hand(&engine, P0, index::DARK_RITUAL).unwrap();
    let skel = in_hand(&engine, P0, index::DRUDGE_SKELETONS).unwrap();
    let bolt = in_hand(&engine, P0, lightning_bolt()).unwrap();
    let land = in_hand(&engine, P0, swamp()).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, scent_of_nightshade());
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
    assert_eq!(min, 0);
    assert!(options.contains(&ritual) && options.contains(&skel));
    assert!(
        !options.contains(&bolt) && !options.contains(&land),
        "only black cards may be revealed: {options:?}"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual, skel],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "-2/-2 kills a 2/2 via 0 toughness"
    );
}

/// Revealing 1 black card shrinks a 3/3 to 2/2 until end of turn, and revealing
/// 0 leaves a creature untouched.
#[test]
fn scent_of_nightshade_shrinks_until_eot_and_allows_zero_reveals() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[
            scent_of_nightshade(),
            scent_of_nightshade(),
            index::DARK_RITUAL,
        ],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let ritual = in_hand(&engine, P0, index::DARK_RITUAL).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut engine, P0, scent_of_nightshade());
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, giant), (2, 2), "-1/-1 for one black card");

    cast_with_floating(&mut engine, P0, scent_of_nightshade());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, bears), (2, 2), "zero revealed gives -0/-0");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, giant), (3, 3), "-1/-1 wears off at end of turn");
}
