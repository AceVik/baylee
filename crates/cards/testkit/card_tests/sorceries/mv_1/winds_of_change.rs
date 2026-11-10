//! `cards/sorceries/mv_1/winds_of_change.rs`, played.
//!
//! "Each player shuffles the cards from their hand into their library, then
//! draws that many cards." The ruling (2007-09-16) is the whole card: each
//! player draws a number equal to the number *they* shuffled in, so three
//! players with three different hand sizes end with three different hands.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn winds_of_change() -> CardIndex {
    card_index("f525cf10-e24c-4c46-9a13-6f8579d09d50")
}

/// Every object in `seat`'s hand with the version it has now (CR 400.7: a
/// card that changed zones is a new object even where its handle is kept).
fn hand_identities(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<(ObjectId, u32)> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .map(|&id| identity(engine, id))
        .collect()
}

/// Winds of Change at a three-player table, with hands of five (four dealt
/// and the card the starting player draws on their first turn, CR 103.8c, once
/// the spell has left it), six and none. Each player ends with exactly the number
/// of cards *they* shuffled away, none of the cards in those hands is the
/// card that was in it before (they all went into the library and a fresh
/// draw came out), and no library lost or gained a card on balance.
#[test]
fn winds_of_change_has_each_player_draw_as_many_as_they_shuffled_in() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(0, &[mountain()])
        .hand(
            0,
            &[winds_of_change(), plains(), island(), forest(), mountain()],
        )
        .hand(
            1,
            &[plains(), island(), forest(), mountain(), plains(), island()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, winds_of_change());
    let before = [
        hand_identities(&engine, p0),
        hand_identities(&engine, p1),
        hand_identities(&engine, p2),
    ];
    let libraries_before = [
        library_size(&engine, p0),
        library_size(&engine, p1),
        library_size(&engine, p2),
    ];
    assert_eq!(
        before.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![5, 6, 0],
        "the hands going in: five after the spell is cast, six, none"
    );
    let rng = engine.state().rng.calls();

    pass_until(&mut engine, stack_is_empty);

    let after = [
        hand_identities(&engine, p0),
        hand_identities(&engine, p1),
        hand_identities(&engine, p2),
    ];
    assert_eq!(
        after.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![5, 6, 0],
        "each player draws exactly as many as they shuffled in"
    );
    assert!(
        engine.state().rng.calls() > rng,
        "the libraries were shuffled by the game's generator"
    );
    for (now, then) in after.iter().zip(&before).take(2) {
        for card in now {
            assert!(
                !then.contains(card),
                "{card:?} is still the object it was: it never went into the library"
            );
        }
    }
    for (seat, libraries) in [p0, p1, p2].into_iter().zip(libraries_before) {
        assert_eq!(
            library_size(&engine, seat),
            libraries,
            "{seat:?}: what went into the library came out again as draws"
        );
    }
}
