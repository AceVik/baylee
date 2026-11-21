//! `cards/artifacts/mv_4/slate_of_ancestry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn slate_of_ancestry() -> CardIndex {
    card_index("a07483b4-c04f-42a4-b979-8b77c11fa8f5")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

/// Slate of Ancestry (a07483b4-c04f-42a4-b979-8b77c11fa8f5) — {4} Artifact:
/// "{4}, {T}, Discard your hand: Draw a card for each creature you control."
///
/// Discards your entire hand as part of the activation cost and draws equal to
/// the creatures YOU control (ignoring the opponent's creatures).
#[test]
fn slate_of_ancestry_discards_hand_and_draws_for_own_creatures_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                slate_of_ancestry(),
                grizzly_bears(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[grizzly_bears(), steel_wall()])
        .battlefield(1, &[grizzly_bears(), llanowar_elves(), steel_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let slate = on_battlefield(&engine, p0, slate_of_ancestry()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(hand_before >= 2);
    let lib_before = library_size(&engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    activate(&mut engine, p0, slate_of_ancestry(), 0);

    // Hand was discarded and Slate tapped as activation costs.
    assert!(is_tapped(&engine, slate));
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "hand discarded as cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        hand_before,
        "all cards from hand moved to graveyard"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "drew 2 cards for the 2 creatures you control (not opponent's 3)"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 2);
}

/// Discarding an empty hand is legal (CR 701.8), so Slate of Ancestry still
/// activates and draws for your creatures when your hand is empty.
#[test]
fn slate_of_ancestry_activates_with_empty_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                slate_of_ancestry(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_cards = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let state = engine.dev_state_mut(p0).unwrap();
    for id in hand_cards {
        state
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    activate(&mut engine, p0, slate_of_ancestry(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "empty hand can still be discarded to draw 1 for 1 creature"
    );
}
