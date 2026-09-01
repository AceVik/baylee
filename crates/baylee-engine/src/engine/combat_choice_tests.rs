//! What combat *offers*, as opposed to what it accepts.
//!
//! Attacker and blocker legality used to live only in `apply`: the choice
//! said "declare attackers" and left the asking side to work out which
//! creatures those could be. The house AI filtered the battlefield with
//! `combat::can_attack`, the Bevy client filtered its board model by
//! "untapped and not summoning sick", and the two disagreed with each other
//! and with the rules — a client cannot re-derive evasion.
//!
//! Both choices now carry the enumeration, and these are the tests that it
//! is the rules' answer and not a plausible-looking subset of it.

use super::testkit::{Duel, card_index, keep_mulligans, pass_until, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// 1/1 flier.
fn baleful_strix() -> CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
/// 1/2 ground creature.
fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

/// Runs the game until `seat` is asked to declare attackers.
#[track_caller]
fn reach_attackers(engine: &mut Engine<super::testkit::RegistryLookup>) -> Pending {
    for _ in 0..80 {
        if matches!(engine.pending(), Pending::ChooseAttackers { .. }) {
            return engine.pending().clone();
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("never reached the declare-attackers step");
}

/// A creature that entered this turn cannot attack (CR 302.6), so it must
/// not be offered — the client would otherwise draw an affordance the
/// engine rejects. A turn later the same creature is offered, so the
/// enumeration tracks the rules rather than being cautious by construction.
#[test]
fn a_creature_cast_this_turn_is_not_offered_until_the_next_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(21, island())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert!(
        attackers.is_empty(),
        "the Strix entered this turn and may not attack"
    );

    // Round the table once: the next declare-attackers step that belongs to
    // seat 0 is a turn later, and by then the Strix has settled in.
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { player, .. } if *player == p0
        )
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        attackers.len(),
        1,
        "one turn later the same creature may attack"
    );
}

/// Evasion is a pairing question. A ground creature is a perfectly legal
/// blocker and still may not block a flier (CR 702.9b), so the offer is
/// per attacker and not one flat list of "creatures that may block".
#[test]
fn a_ground_creature_is_not_offered_against_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(22, island())
        .battlefield(0, &[baleful_strix()])
        .battlefield(1, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);

    // Seat 0 passes its sick first combat; seat 1 does the same; on seat 0's
    // second turn the Strix attacks.
    let attackers = loop {
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = reach_attackers(&mut engine)
        else {
            unreachable!()
        };
        if player == p0 && !attackers.is_empty() {
            break attackers;
        }
        engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .unwrap();
    };
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    };
    assert!(
        blockers.is_empty(),
        "a 1/2 without flying or reach was offered against a flier: {blockers:?}"
    );
}
