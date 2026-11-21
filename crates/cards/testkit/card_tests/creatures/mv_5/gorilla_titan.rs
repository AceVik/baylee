//! `cards/creatures/mv_5/gorilla_titan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gorilla_titan() -> CardIndex {
    card_index("9ce80c55-a995-4226-8506-901f6cf06efc")
}

/// Gorilla Titan (9ce80c55-a995-4226-8506-901f6cf06efc) — `{3}{G}{G}` 4/4 Creature — Ape:
/// - Trample
/// - This creature gets +4/+4 as long as there are no cards in your graveyard.
#[test]
fn gorilla_titan_is_eight_eight_only_while_your_graveyard_is_empty() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), mountain()],
        )
        .hand(0, &[gorilla_titan(), lightning_bolt()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Put a card in P1's graveyard -> opponent's graveyard does NOT turn off Gorilla Titan.
    let _ = hand_to_graveyard(&mut engine, P1, island());

    tap_all_mana_but(&mut engine, P0, Some(mountain()));
    cast_with_floating(&mut engine, P0, gorilla_titan());
    pass_until(&mut engine, stack_is_empty);

    let titan = on_battlefield(&engine, P0, gorilla_titan()).expect("Gorilla Titan entered");
    assert!(keywords(&engine, titan).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, titan),
        (8, 8),
        "empty P0 graveyard grants +4/+4 even when opponent's graveyard is non-empty"
    );

    // Casting Lightning Bolt puts a card into P0's graveyard -> drops immediately to 4/4.
    cast_from_hand(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, titan), (4, 4));
}
