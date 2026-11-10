//! Headstone — `{1}{B}` Instant (Homelands)
//!
//! Exile target card from a graveyard.
//! Draw a card at the beginning of the next turn's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn headstone() -> CardIndex {
    card_index("cbf767d4-9be4-417a-bbf7-0c72ede727b4")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn headstone_exiles_target_card_from_graveyard_and_draws_at_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[headstone(), lightning_bolt()])
        .battlefield(0, &[swamp(), swamp(), mountain()])
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Destroy Grizzly Bears with Lightning Bolt so it's in p1's graveyard.
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let bear_bf = on_battlefield(&engine, p1, bears()).unwrap();
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == mtn);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_bf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hs = in_hand(&engine, p0, headstone()).unwrap();
    let gy_card = in_graveyard(&engine, p1, bears()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: hs })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_card],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_card).unwrap().zone,
        Zone::Exile,
        "targeted card was exiled from graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the card is drawn at the next upkeep, not now"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "only the targeted card is exiled"
    );

    // Advance to p1's FirstMain: p0 drew a card at p1's upkeep from the delayed trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Headstone drew a card at the beginning of the next turn's upkeep"
    );
}
