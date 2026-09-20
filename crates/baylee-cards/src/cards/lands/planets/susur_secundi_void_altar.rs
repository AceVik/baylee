//! Susur Secundi, Void Altar — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {1}{B}, {T}, Pay 2 life, Sacrifice a creature: Draw cards equal to the sacrificed creature's power. Activate only as a sorcery.
//! Set: EOE #259 — Edge of Eternities | Scryfall ID: aefb8c0d-2bc6-4bec-851e-0137b4abfb22 | Oracle ID: 50d6cadc-07e4-479e-90f4-e3a20f769bab
// PARTIAL — the arrival replacement and the mana ability only; the Station
// ability and its 12+ payoff are NOT SUPPORTED (see below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUSUR_SECUNDI_VOID_ALTAR,
    oracle_id = "50d6cadc-07e4-479e-90f4-e3a20f769bab",
    scryfall_id = "aefb8c0d-2bc6-4bec-851e-0137b4abfb22",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Station's charge counters and the 12+ ability's draw count both read the power of the creature tapped or sacrificed to pay a cost, which no Amount names"
    ),
    faces = &[face!(
        name = "Susur Secundi, Void Altar",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "Station (Tap another creature you control: Put charge
// counters equal to its power on this Planet. Station only as a sorcery.)" —
// `CostPart::TapOther(filter)` is sayable and `CounterKind::Charge` exists,
// but the count is the power of the permanent *named to pay the cost*, and no
// `Amount` reads a cost's chosen object: `Amount::TargetPower` reads the
// ability's first target, and Station targets nothing (CR 115.1c). Nothing
// else in the vocabulary answers it either — `Amount::CountOf` counts a
// filter, `Amount::SourcePower` reads this land.
// NOT SUPPORTED: "12+ | {1}{B}, {T}, Pay 2 life, Sacrifice a creature: Draw
// cards equal to the sacrificed creature's power. Activate only as a
// sorcery." — the gate and the price are sayable
// (`Condition::CountersOnSelf(CounterKind::Charge, 12)` on an
// `ActivatedConditional`, with `cost!("{1}{B}", TapSelf, PayLife(2),
// Sacrifice(..))` at `ActivationTiming::SorcerySpeed`), but the draw count is
// the power of the creature sacrificed to pay that cost, which is the same
// missing `Amount` as above and cannot be approximated by a fixed number.
