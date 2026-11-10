//! `cards/creatures/mv_2/people_of_the_woods.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn people_of_the_woods() -> CardIndex {
    card_index("66e7a760-2b2f-4af9-983c-300c657beaaf")
}

/// People of the Woods (66e7a760-2b2f-4af9-983c-300c657beaaf) — "People of
/// the Woods's toughness is equal to the number of Forests you control."
/// (printed power 1)
#[test]
fn people_of_the_woods_toughness_counts_only_your_forests() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                people_of_the_woods(),
                forest(),
                forest(),
                forest(),
                island(),
            ],
        )
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let people = on_battlefield(&engine, P0, people_of_the_woods()).expect("People is out");
    assert_eq!(
        pt(&engine, people),
        (1, 3),
        "three Forests of mine count; the Island and the opponent's Forests do not"
    );
}

/// The count is live: each Forest that leaves lowers the toughness, and with
/// none left the creature has toughness 0 and dies.
#[test]
fn people_of_the_woods_toughness_follows_forests_and_dies_at_zero() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[people_of_the_woods(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let people = on_battlefield(&engine, P0, people_of_the_woods()).expect("People is out");
    assert_eq!(pt(&engine, people), (1, 2));

    let forests = all_on_battlefield(&engine, P0, forest());
    assert_eq!(forests.len(), 2);
    kill(&mut engine, forests[0]);
    assert_eq!(
        pt(&engine, people),
        (1, 1),
        "one Forest left, toughness one"
    );

    kill(&mut engine, forests[1]);
    assert!(
        on_battlefield(&engine, P0, people_of_the_woods()).is_none(),
        "no Forest, toughness 0: state-based actions put it away"
    );
    assert!(in_graveyard(&engine, P0, people_of_the_woods()).is_some());
}

/// A Forest arriving raises it again (the count is not fixed at entry).
#[test]
fn people_of_the_woods_grows_when_a_forest_arrives() {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forest()])
        .battlefield(0, &[people_of_the_woods(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let people = on_battlefield(&engine, P0, people_of_the_woods()).expect("People is out");
    assert_eq!(pt(&engine, people), (1, 1));
    let land = in_hand(&engine, P0, forest()).expect("a Forest in hand");
    engine
        .apply(P0, PlayerAction::PlayLand { card: land })
        .expect("the land drop is available");
    assert_eq!(pt(&engine, people), (1, 2));
}
