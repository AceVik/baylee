//! `cards/instants/mv_2/false_cure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "fc4c969d-408c-46d1-8dba-fabecc2c30ce"
fn false_cure() -> CardIndex {
    card_index("fc4c969d-408c-46d1-8dba-fabecc2c30ce")
}

/// False Cure ({B}{B}): until end of turn, whenever ANY player (you or an
/// opponent) gains life, that player loses twice that much life; expires at
/// end of turn.
#[test]
fn false_cure_makes_any_player_lose_double_life_gained_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[false_cure(), nourish(), nourish()],
        &[nourish()],
    );

    // Cast False Cure.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, false_cure());
    lk::settle(&mut engine);

    // P0 gains 6 via Nourish (20 -> 26), then loses 12 from False Cure (26 -> 14).
    cast_with_floating(&mut engine, P0, nourish());
    lk::settle(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        14,
        "net -6 life for P0 (gained 6, lost 12)"
    );

    // On the same turn, P1 gains 6 via Nourish (20 -> 26), then loses 12 (26 -> 14).
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P1, nourish());
    lk::settle(&mut engine);
    assert_eq!(
        lk::life(&engine, P1),
        14,
        "net -6 life for P1 (gained 6, lost 12)"
    );

    // Next turn False Cure has worn off: gaining 6 gives +6 with no life loss.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P0, nourish());
    lk::settle(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "False Cure expired at end of turn"
    );
}
