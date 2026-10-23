//! `cards/instants/mv_3/dazzling_beauty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, WILD_ELEPHANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dazzling_beauty() -> CardIndex {
    card_index("de96ef93-a2e0-409d-9d25-fcb2405039b5")
}

/// Dazzling Beauty (de96ef93-a2e0-409d-9d25-fcb2405039b5) — {2}{W} Instant:
/// "Cast this spell only during the declare blockers step. Target unblocked
/// attacking creature becomes blocked. Draw a card at the beginning of the
/// next turn's upkeep."
///
/// P1 attacks P0 with a Hill Giant and a Wild Elephant (trample) and nobody
/// blocks. P0 holds the Beauty with three Plains.
fn their_attack_unblocked() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[HILL_GIANT, WILD_ELEPHANT])
        .hand(0, &[dazzling_beauty()])
        .start();
    keep_mulligans(&mut engine);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();
    let elephant = on_battlefield(&engine, P1, WILD_ELEPHANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (giant, Defender::Player(P0)),
                    (elephant, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    (engine, giant, elephant)
}

/// The Giant's 3 damage to P0 is stopped, the card is drawn at the next
/// upkeep (P0's turn 3, on top of the draw step), the Elephant is not on the
/// menu (a blocked trampler would still hit P0), and in the declare attackers
/// step, before blockers, the spell is not castable.
#[test]
fn dazzling_beauty_stops_an_unblocked_attackers_damage_and_draws_next_upkeep() {
    let (mut engine, giant, elephant) = their_attack_unblocked();

    // Declare attackers step, P1 first: pass to P0, who still may not cast it
    // (with the Plains untapped, `payable` is where a legal cast would be).
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    let card = in_hand(&engine, P0, dazzling_beauty()).unwrap();
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "only during the declare blockers step: {legal:?}"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // P1 gets priority in DeclareBlockers first; pass to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_with_floating(&mut engine, P0, dazzling_beauty());
    let menu = aim_at(&mut engine, P0, giant);
    assert!(menu.contains(&giant), "{menu:?}");
    assert!(
        !menu.contains(&elephant),
        "a creature with trample is not a target: {menu:?}"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // The Giant (3) assigned no combat damage; the Elephant (3) did.
    assert_eq!(engine.state().players[0].life, 17);

    // P0's turn 3: the Beauty's draw at the upkeep and the draw step.
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1 + 2,
        "the Beauty left the hand, the upkeep draw and the draw step came in"
    );
}
