//! `cards/instants/mv_3/jolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jolt() -> CardIndex {
    card_index("13f69c70-dc93-4983-b649-d91b22633eb9")
}

/// Jolt (13f69c70-dc93-4983-b649-d91b22633eb9) — {2}{U} Instant:
/// "You may tap or untap target artifact, creature, or land. Draw a card at the
/// beginning of the next turn's upkeep."
#[test]
fn jolt_taps_target_permanent_and_draws_a_card_at_next_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[jolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, jolt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Advance to P1's main phase so the delayed upkeep draw resolves.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before_hand - 1 + 1
    );
}

/// P0 casts Jolt at P1's Grizzly Bears (tapped first when `tapped`); the game
/// stops on the "you may" question.
fn jolt_at_bears(tapped: bool) -> (Engine<RegistryLookup>, ObjectId, usize) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[jolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    if tapped {
        engine.dev_state_mut(P0).unwrap().set_tapped(bears, true);
        engine.refresh_offer();
    }
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, jolt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, bears, hand)
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

/// "You may tap or untap": a tapped creature is untapped on a yes.
#[test]
fn jolt_untaps_a_tapped_creature() {
    let (mut engine, bears, _) = jolt_at_bears(true);
    assert!(is_tapped(&engine, bears));

    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears));
}

/// "You may": a no leaves the creature as it was, and the card is still drawn
/// at the next upkeep.
#[test]
fn jolt_declined_leaves_the_creature_alone_and_still_draws() {
    let (mut engine, bears, hand) = jolt_at_bears(false);

    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bears));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand - 1 + 1,
        "Jolt left the hand, the upkeep draw came in"
    );
}
