//! Library searches: where the finds go, and what gets shown.
//!
//! `Effect::SearchLibrary` used to carry a single `dest` and a single
//! `tapped`, so every card that fetches two lands to two places was
//! inexpressible — Cultivate, Kodama's Reach and Myriad Landscape all failed
//! on it. It now carries one `Find` per card the search may produce, and this
//! is the test that the destinations are actually honoured separately rather
//! than the first one being applied to everything.
//!
//! The shuffle and the reveal are not card data at all: the engine derives
//! both, so the tests below are also the record of what those rules are.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

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

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn mystical_tutor() -> CardIndex {
    card_index("fb81f95c-70f8-4eb7-8d15-15d0ae23ec03")
}
fn demonic_tutor() -> CardIndex {
    card_index("82004860-e589-4e38-8d61-8c0210e4ea39")
}
fn windswept_heath() -> CardIndex {
    card_index("29737a60-3ebd-40d9-b935-c4f54b90d45d")
}

/// Taps everything, casts the one card in hand and passes until the search
/// asks; returns what it offers.
#[track_caller]
fn cast_and_search(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(seat))[0];
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .unwrap();
    settle_to_search(engine)
}

/// Passes priority until the search choice arrives.
#[track_caller]
fn settle_to_search(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..6 {
        if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
            return options;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the search never asked");
}

/// The cards each `Revealed` event in the journal names.
fn revealed(engine: &Engine<RegistryLookup>) -> Vec<Vec<ObjectId>> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            crate::event::GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .collect()
}

/// "Search your library for an instant or sorcery card, reveal it, then
/// shuffle and put that card on top." The reveal is the whole point of the
/// line: the card ends up hidden again, so without it nothing holds the
/// searcher to "instant or sorcery". No card file asks for it — the engine
/// derives it from the filter and the destination.
#[test]
fn a_filtered_search_into_a_hidden_zone_reveals_what_it_found() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9, cultivate())
        .battlefield(0, &[island()])
        .hand(0, &[mystical_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let options = cast_and_search(&mut engine, p0);
    let picked = vec![options[0]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .unwrap();

    assert_eq!(revealed(&engine), vec![picked], "the find was shown");
}

/// "Search your library for a card, put that card into your hand." Nothing
/// narrows it, so there is nothing to hold anyone to — and the printed card
/// says no reveal.
#[test]
fn an_unfiltered_search_reveals_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[demonic_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let options = cast_and_search(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    assert!(
        revealed(&engine).is_empty(),
        "a tutor for \"a card\" shows nothing"
    );
}

/// A fetchland's search is filtered, but it ends on the battlefield, where
/// everyone sees the land anyway. Revealing there would be noise.
#[test]
fn a_search_onto_the_battlefield_reveals_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9, forest())
        .battlefield(0, &[windswept_heath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == windswept_heath()))
        })
        .expect("the fetch is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let options = settle_to_search(&mut engine);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    assert!(
        revealed(&engine).is_empty(),
        "the fetched land is about to be public anyway"
    );
}
