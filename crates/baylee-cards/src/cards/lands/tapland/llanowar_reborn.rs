//! Llanowar Reborn — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Graft 1 (This land enters with a +1/+1 counter on it. Whenever a creature enters, you may move a +1/+1 counter from this land onto it.)
//! Set: LCC #341 — The Lost Caverns of Ixalan Commander | Scryfall ID: 284e165e-9ab6-47fc-9685-0fb03de237df | Oracle ID: 92acb789-0e42-465c-ac16-40fefec48805
// PARTIAL — enters tapped, taps for {G}, and arrives with its +1/+1 counter;
// graft's move is missing.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LLANOWAR_REBORN,
    oracle_id = "92acb789-0e42-465c-ac16-40fefec48805",
    scryfall_id = "284e165e-9ab6-47fc-9685-0fb03de237df",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Llanowar Reborn",
        types = TypeSet::LAND,
        enter_modifiers = &[
            EnterModifier::Tapped,
            EnterModifier::WithCounters {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
    coverage = Coverage::Partial(
        "graft's second sentence: no effect takes a +1/+1 counter off this land and puts it \
         onto the entering creature"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);

// NOT SUPPORTED: "Whenever a creature enters, you may move a +1/+1 counter
// from this land onto it." The cost half of the move exists as
// `CostPart::RemoveCounterSelf`, but that is a cost paid when an ability is
// activated, not an effect that resolves; the effect half has no vocabulary
// at all — `Effect::AddCounter` only adds, `Effect::DrainAllCountersIntoSelf`
// drains into the source, and nothing removes a counter from the source on
// resolution to put it on a target. The `MayDo` wrapper that the printed "you
// may" asks for is therefore not writable either, since the clause it would
// wrap cannot be said.
//
// Graft is also not one of the keyword bits the engine reads (flying, first
// strike, double strike, deathtouch, haste, hexproof, shroud, indestructible,
// lifelink, menace, reach, trample, vigilance, defender, flash, prowess,
// changeling, unblockable, uncounterable, rebound, daybound, nightbound), so
// it is deliberately absent from `keywords` rather than a dead bit.
