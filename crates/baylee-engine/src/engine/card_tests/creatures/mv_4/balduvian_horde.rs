//! Balduvian Horde — `{2}{R}{R}` 5/5 Creature — Human Barbarian (Masters 25)
//!
//! When this creature enters, sacrifice it unless you discard a card at random.

#[allow(clippy::wildcard_imports)]
use super::*;

fn balduvian_horde() -> CardIndex {
    card_index("fd2a94d3-901d-4b46-aa9b-a30ed6dea6bd")
}

/// Casts the Horde with `others` left in P0's hand, answers "may discard" with `discard`, and
/// returns the engine, the Horde and the number of cards that went from the hand to the graveyard.
fn cast_horde(others: &[CardIndex], discard: bool) -> (Engine<RegistryLookup>, ObjectId, usize) {
    let p0 = PlayerId::new(0);
    let mut hand = vec![balduvian_horde()];
    hand.extend_from_slice(others);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horde = in_hand(&engine, p0, balduvian_horde()).unwrap();
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: horde })
        .unwrap();
    // The enters trigger asks the controller; `pass_until` would say yes on its own, so stop here.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(discard)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let discarded = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len() - graveyard_before;
    (engine, horde, discarded)
}

#[test]
fn balduvian_horde_stays_when_a_card_is_discarded_at_random() {
    let p0 = PlayerId::new(0);
    let (engine, horde, graveyard_gain) = cast_horde(&[ironclaw_orcs(), ironclaw_orcs()], true);

    assert_eq!(
        engine.state().object(horde).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(pt(&engine, horde), (5, 5));
    assert_eq!(graveyard_gain, 1, "exactly one card is discarded");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "of the two cards left in hand, one went"
    );
}

#[test]
fn balduvian_horde_is_sacrificed_and_nothing_discarded_when_the_player_declines() {
    let p0 = PlayerId::new(0);
    let (engine, horde, graveyard_gain) = cast_horde(&[ironclaw_orcs(), ironclaw_orcs()], false);

    assert_eq!(engine.state().object(horde).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        graveyard_gain, 1,
        "only the Horde itself reached the graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "no card was discarded"
    );
}

/// "Unless you discard": an empty hand cannot pay, so the Horde goes even when the player says yes.
#[test]
fn balduvian_horde_is_sacrificed_when_there_is_no_card_to_discard() {
    let (engine, horde, graveyard_gain) = cast_horde(&[], true);

    assert_eq!(engine.state().object(horde).unwrap().zone, Zone::Graveyard);
    assert_eq!(graveyard_gain, 1);
}
