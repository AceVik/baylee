//! `cards/creatures/mv_6/muldrotha_the_gravetide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Muldrotha, the Gravetide: a land in the graveyard is a land play.
///
/// The smallest board for it; the test after this one is the whole sentence,
/// one of each permanent type a turn.
#[test]
fn muldrotha_lets_you_play_a_land_out_of_your_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(387, forest())
        .battlefield(0, &[muldrotha_the_gravetide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The library filler is a Forest, so the graveyard is one land.
    seed_graveyard(&mut engine, p0, 1);

    let land = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("Muldrotha permits the land drop out of the graveyard");
    assert_eq!(
        engine
            .state()
            .object(land)
            .expect("the land is still an object")
            .zone,
        Zone::Battlefield,
        "and the land is on the battlefield rather than back in the graveyard"
    );
}

/// The whole of Muldrotha: "During each of your turns, you may play a land and
/// cast a permanent spell of each permanent type from your graveyard."
///
/// One of each type a turn is what separates it from Crucible of Worlds and
/// Wrenn's emblem. With Exploration's second land drop open, the second Forest
/// in the graveyard is not offered once the first was played from there (the
/// Forest in hand still is); with a creature cast from the graveyard, the
/// second Llanowar Elves waits while Sol Ring, an artifact, is still offered.
/// On the opponent's turn nothing is allowed, and on the next turn of the
/// owner's the allowance is whole again.
#[test]
fn muldrotha_allows_one_permanent_of_each_type_from_the_graveyard_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(397, forest())
        .battlefield(
            0,
            &[
                muldrotha_the_gravetide(),
                exploration(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let first_land = hand_to_graveyard(&mut engine, p0, forest());
    let second_land = hand_to_graveyard(&mut engine, p0, forest());
    let first_elf = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    let second_elf = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    let ring = hand_to_graveyard(&mut engine, p0, quiet_artifact());
    let hand_forest = in_hand(&engine, p0, forest()).expect("one Forest stays in hand");

    let legal = priority_offer(&engine);
    assert!(
        legal.lands.contains(&first_land) && legal.lands.contains(&second_land),
        "a land from the graveyard: {:?}",
        legal.lands
    );
    engine
        .apply(p0, PlayerAction::PlayLand { card: first_land })
        .expect("Muldrotha lets the Forest be played from the graveyard");
    let legal = priority_offer(&engine);
    assert!(
        !legal.lands.contains(&second_land),
        "\"a land\": one a turn from the graveyard"
    );
    assert!(
        legal.lands.contains(&hand_forest),
        "while Exploration's second land drop is still open"
    );

    // `legal.castable` reads the pool, so the mana floats first.
    tap_all_mana(&mut engine, p0);
    let legal = priority_offer(&engine);
    for card in [first_elf, second_elf, ring] {
        assert!(
            legal.castable.contains(&card),
            "a creature and an artifact from the graveyard: {:?}",
            legal.castable
        );
    }
    cast_object_and_resolve(&mut engine, p0, first_elf);
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&second_elf),
        "the creature of the turn is spent: {:?}",
        legal.castable
    );
    assert!(
        legal.castable.contains(&ring),
        "and the artifact is not: {:?}",
        legal.castable
    );
    cast_object_and_resolve(&mut engine, p0, ring);

    reach_their_main_phase(&mut engine, p1);
    let elf = engine.state().object(second_elf).expect("the Elves");
    assert!(
        crate::casting::graveyard_cast_permission(engine.state(), p0, elf).is_none()
            && crate::casting::graveyard_land_permission(engine.state(), p0).is_none(),
        "\"during each of your turns\": not the opponent's"
    );

    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let legal = priority_offer(&engine);
    assert!(
        legal.castable.contains(&second_elf) && legal.lands.contains(&second_land),
        "a new turn, a new creature and a new land: {:?} {:?}",
        legal.castable,
        legal.lands
    );
}
