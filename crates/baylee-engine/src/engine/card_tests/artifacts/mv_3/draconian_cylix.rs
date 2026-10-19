//! `cards/artifacts/mv_3/draconian_cylix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn draconian_cylix() -> CardIndex {
    card_index("0880b36a-6141-4955-b532-cf88daca0869")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn graveyard_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .len()
}

fn shields(engine: &Engine<RegistryLookup>, id: ObjectId) -> u8 {
    engine.state().object(id).unwrap().regeneration_shields
}

/// p0: Cylix, Bears, two Forests, two Islands in hand. p1: Hill Giant.
fn a_cylix_board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[draconian_cylix(), grizzly_bears(), forest(), forest()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Draconian Cylix (0880b36a-6141-4955-b532-cf88daca0869) — "{2}, {T}, Discard
/// a card at random: Regenerate target creature."
///
/// The price is three things at once: {2}, the Cylix's tap and one card of the
/// hand, gone before the ability resolves; the Bears then hold one shield.
#[test]
fn draconian_cylix_discards_a_card_and_shields_the_target() {
    let p0 = PlayerId::new(0);
    let mut engine = a_cylix_board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
    let cylix = on_battlefield(&engine, p0, draconian_cylix()).expect("Cylix");
    let hand_before = hand_size(&engine, p0);
    let yard_before = graveyard_size(&engine, p0);

    assert_eq!(tap_all_mana(&mut engine, p0), 2);
    activate(&mut engine, p0, draconian_cylix(), 0);
    aim_at(&mut engine, p0, bears);
    assert_eq!(
        hand_size(&engine, p0),
        hand_before - 1,
        "a card was discarded"
    );
    assert_eq!(graveyard_size(&engine, p0), yard_before + 1);
    assert!(is_tapped(&engine, cylix), "{{T}} was paid");
    assert_eq!(shields(&engine, bears), 0, "nothing until it resolves");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(shields(&engine, bears), 1, "one regeneration shield");
}

/// The discard is part of the price: with an empty hand the ability is not on
/// offer, however much mana is floating.
#[test]
fn draconian_cylix_needs_a_card_to_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[draconian_cylix(), grizzly_bears(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cylix = on_battlefield(&engine, p0, draconian_cylix()).expect("Cylix");
    if hand_size(&engine, p0) > 0 {
        // This player drew for the turn; empty the hand so only the missing
        // card is in question.
        let cards: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
        bury_hand(&mut engine, &cards);
    }
    assert_eq!(hand_size(&engine, p0), 0);
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine).abilities.contains(&(cylix, 0)),
        "no card to discard, no activation"
    );
}

/// Moves the cards to the graveyard through the harness.
fn bury_hand(engine: &mut Engine<RegistryLookup>, cards: &[ObjectId]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for &card in cards {
        state
            .move_object(
                card,
                ZoneLocation::Graveyard(PlayerId::new(0)),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    engine.refresh_offer();
}

/// The shield does its job: the Bears attack, the Giant blocks and deals lethal
/// damage, and with the Cylix's shield the Bears are still on the battlefield.
/// Without it, in the same combat, they die.
#[test]
fn draconian_cylix_shield_saves_the_creature_in_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for shielded in [true, false] {
        let mut engine = a_cylix_board();
        let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
        let giant = on_battlefield(&engine, p1, hill_giant()).expect("Giant");
        if shielded {
            tap_all_mana(&mut engine, p0);
            activate(&mut engine, p0, draconian_cylix(), 0);
            aim_at(&mut engine, p0, bears);
            pass_until(&mut engine, stack_is_empty);
        }
        pass_until(&mut engine, |e| {
            e.state().turn.step == crate::turn::Step::DeclareAttackers
        });
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(bears, Defender::Player(p1))],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.step == crate::turn::Step::DeclareBlockers
        });
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bears)],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.phase == crate::turn::Phase::SecondMain
        });
        assert_eq!(
            on_battlefield(&engine, p0, grizzly_bears()).is_some(),
            shielded,
            "shielded: {shielded}"
        );
    }
}
