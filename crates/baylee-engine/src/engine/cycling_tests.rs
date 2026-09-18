//! Cycling, played from hand, on the eleven cards a reader wrote.
//!
//! CR 702.29a spells the keyword out: "Cycling [cost]" means "[Cost],
//! Discard this card: Draw a card", an activated ability that functions only
//! while the card is in a player's hand. `xtask codegen` now reads
//! `K:Cycling:<cost>` and emits exactly that shape, which turned eleven pool
//! stubs into cards — so what is under test here is one rule's output eleven
//! times over, and not eleven cards that happen to share a keyword.
//!
//! Two claims, and the second is what makes the first worth having:
//!
//! 1. The ability is offered from hand, its cost discards the card, and it
//!    draws one. The card is spent and replaced: net zero.
//! 2. The price is **collected**. On an empty board nothing is offered at
//!    all, and on a board of nothing but Forests the ten three-colour
//!    landscapes stay unoffered while Night Market, whose price is generic,
//!    is offered off the same eight green mana. An ability that is offered
//!    and then not paid for is a free draw, and claim 1 alone cannot tell
//!    the two apart — nor can `xtask validate`, whose price check reads a
//!    `{…}:` prefix at the *start* of a printed line. Cycling prints its
//!    price as `Cycling {3}`, and the `{3}, Discard this card:` that would
//!    match sits inside the reminder text behind it, so the check passes
//!    over these eleven cards without comparing anything.

use super::testkit::{
    Duel, RegistryLookup, SEED, basic_forest, basics, card_index, in_graveyard, in_hand,
    keep_mulligans, pass_until, reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_cards_dsl::{AbilityDef, ActivationZone};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{CardIndex, ObjectId};

/// The eleven cards `K:Cycling:<cost>` wrote, by oracle id and name.
///
/// Named rather than swept off the pool, because the population is the
/// point: these are the cards one reader reached, and a sweep for "every
/// card with a hand ability" would quietly grow a hand-written twelfth into
/// the claim — or, the day a reader regressed, shrink to nothing and pass.
const CYCLERS: &[(&str, &str)] = &[
    ("4cdb9f80-d08f-4986-99c8-573166d66082", "Night Market"),
    (
        "2eb69a8f-9456-4852-b868-85ae609d3441",
        "Bountiful Landscape",
    ),
    (
        "28196fd9-00c9-4cd0-b603-0eec8511ec79",
        "Contaminated Landscape",
    ),
    (
        "1831fe12-dbe0-437f-8fc8-f01bbb701fe1",
        "Deceptive Landscape",
    ),
    (
        "bcfe1653-e602-4d38-abe7-bfcc7f203f9d",
        "Foreboding Landscape",
    ),
    ("e2b472dd-047d-47eb-9ebb-df6aa4b52dd4", "Perilous Landscape"),
    ("2d8635bd-ed96-4bb1-8718-6962a0eee3d5", "Seething Landscape"),
    (
        "7fbad3f2-66f6-4e3a-b9e0-1ddbcd94d42a",
        "Shattered Landscape",
    ),
    (
        "5b932be0-4dac-41b9-9c59-f79e4cecc31a",
        "Sheltering Landscape",
    ),
    ("d4eb65d5-99fd-4daf-b7d3-8ebf99ee9c61", "Tranquil Landscape"),
    ("db659cae-2078-423e-a6ed-63898dbab87f", "Twisted Landscape"),
];

/// The colours the card's hand ability charges for.
///
/// Both activated arms, because a precondition is the only difference
/// between them and reading one of them alone is this repo's own recurring
/// fault.
fn cycling_colors(card: CardIndex) -> ColorSet {
    let def = baylee_cards::by_index(card).expect("the registry has the card");
    def.abilities_for_face(0)
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Activated {
                cost,
                zone: ActivationZone::Hand,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                zone: ActivationZone::Hand,
                ..
            } => Some(cost.mana.colors()),
            _ => None,
        })
        .expect("the card prints an ability activated from hand")
}

/// Taps every mana source the seat has, and answers what its hand ability
/// is offered as — `None` where the offer was not made at all.
///
/// The tap comes first because affordability is read off the **pool**
/// (`Engine::can_pay_mana`), not off what is still untapped: an ability
/// whose price nobody has floated is not offered, which is exactly the
/// distinction claim 2 rests on.
fn offer_after_tapping(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    object: ObjectId,
) -> Option<u32> {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal
        .abilities
        .iter()
        .find(|(id, _)| *id == object)
        .map(|&(_, index)| index)
}

#[test]
fn every_transcoded_cycler_cycles_from_hand() {
    let seat = PlayerId::new(0);
    for (oracle_id, name) in CYCLERS {
        let card = card_index(oracle_id);
        let mut engine = Duel::new(SEED, basic_forest())
            .hand(0, &[card])
            .battlefield(0, &basics())
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, seat);

        let object = in_hand(&engine, seat, card).unwrap_or_else(|| panic!("{name} is in hand"));
        let held = engine.state().zones.list(ZoneLocation::Hand(seat)).len();
        let turn = engine.state().turn.number;
        let ability_index = offer_after_tapping(&mut engine, seat, object)
            .unwrap_or_else(|| panic!("{name} offers no ability from hand"));
        engine
            .apply(
                seat,
                PlayerAction::ActivateAbility {
                    source: object,
                    ability_index,
                },
            )
            .unwrap();

        // The discard is a cost, so it is already paid while the ability is
        // still on the stack (CR 601.2h by way of CR 602.2b).
        assert!(
            in_hand(&engine, seat, card).is_none(),
            "{name}: cycling discards the card as a cost"
        );
        assert!(
            in_graveyard(&engine, seat, card).is_some(),
            "{name}: the discarded card is in its owner's graveyard"
        );

        pass_until(&mut engine, stack_is_empty);
        // Still this turn, which is what makes the count below a reading of
        // the cycling draw: a turn later the draw step would have replaced
        // the discarded card all by itself and the two outcomes would be
        // the same number.
        assert_eq!(
            engine.state().turn.number,
            turn,
            "{name}: the ability resolved in the turn it was activated"
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(seat)).len(),
            held,
            "{name}: cycling spends one card and draws one"
        );
    }
}

#[test]
fn a_cycling_price_is_collected_and_not_merely_printed() {
    let seat = PlayerId::new(0);
    let green = ColorSet::of(Color::Green);
    for (oracle_id, name) in CYCLERS {
        let card = card_index(oracle_id);

        // Nothing to tap, so nothing is floating and nothing is affordable.
        let mut broke = Duel::new(SEED, basic_forest()).hand(0, &[card]).start();
        keep_mulligans(&mut broke);
        reach_main_phase(&mut broke, seat);
        let object = in_hand(&broke, seat, card).unwrap_or_else(|| panic!("{name} is in hand"));
        assert!(
            offer_after_tapping(&mut broke, seat, object).is_none(),
            "{name}: cycling is offered on an empty board, so its price is free"
        );

        // Eight green mana: enough for any of these prices by amount, and
        // enough for exactly one of them by colour.
        let forests = vec![basic_forest(); 8];
        let mut mono = Duel::new(SEED, basic_forest())
            .hand(0, &[card])
            .battlefield(0, &forests)
            .start();
        keep_mulligans(&mut mono);
        reach_main_phase(&mut mono, seat);
        let object = in_hand(&mono, seat, card).unwrap_or_else(|| panic!("{name} is in hand"));
        let payable = cycling_colors(card).difference(green).is_empty();
        assert_eq!(
            offer_after_tapping(&mut mono, seat, object).is_some(),
            payable,
            "{name}: off eight Forests, cycling should be offered iff its price is green or generic"
        );
    }
}
