//! Library searches that produce more than one card.
//!
//! `Effect::SearchLibrary` used to carry a single `dest` and a single
//! `tapped`, so every card that fetches two lands to two places was
//! inexpressible — Cultivate, Kodama's Reach and Myriad Landscape all failed
//! on it. It now carries one `Find` per card the search may produce, and this
//! is the test that the destinations are actually honoured separately rather
//! than the first one being applied to everything.

use super::testkit::{Duel, card_index, keep_mulligans, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn cultivate() -> CardIndex {
    card_index("8b755881-a72d-4e21-a369-d2924eb4585a")
}

/// "…put one onto the battlefield tapped and the other into your hand."
/// Two finds, two different destinations, one search.
#[test]
fn cultivate_puts_one_land_onto_the_battlefield_and_the_other_in_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[cultivate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Both players pass, the sorcery resolves, the search asks.
    for _ in 0..4 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the search choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        (min, max),
        (0, 2),
        "\"up to two\" means two may be found and none is legal"
    );
    assert!(options.len() >= 2, "the library holds basic lands");

    let picked = vec![options[0], options[1]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .unwrap();

    // The first find is the battlefield, tapped; the second is the hand.
    let first = engine.state().object(picked[0]).expect("first find");
    assert_eq!(
        first.zone,
        crate::zone::Zone::Battlefield,
        "the first card named goes onto the battlefield"
    );
    assert!(
        first.status.contains(crate::object::Status::TAPPED),
        "and it enters tapped"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&picked[1]),
        "the second card named goes to hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "exactly one card reached the hand"
    );
}

/// Finding fewer cards than the search allows fills the finds from the front:
/// one basic land found by Cultivate is put onto the battlefield tapped, not
/// into hand. That is the order the printed text names, and the order the
/// forge reference resolves its two sub-abilities in.
#[test]
fn one_land_found_takes_the_first_destination() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[cultivate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    for _ in 0..4 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the search choice");
    };

    let one = options[0];
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![one] })
        .unwrap();

    let obj = engine.state().object(one).expect("the single find");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert!(obj.status.contains(crate::object::Status::TAPPED));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "nothing reached the hand"
    );
}
