//! Adagia, Windswept Bastion — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {3}{W}, {T}: Create a token that's a copy of target artifact or enchantment you control, except it's legendary. Activate only as a sorcery.
//! Set: EOE #250 — Edge of Eternities | Scryfall ID: c634273a-94b0-4104-9d10-ae522ece1fc7 | Oracle ID: 70d35dbd-1d91-4a2a-a643-6870d168f4f5
// PARTIAL — entering tapped and "{T}: Add {W}" are built; the Station
// ability and the 12+ copy ability have no DSL shape (see the two
// NOT SUPPORTED lines below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ADAGIA_WINDSWEPT_BASTION,
    oracle_id = "70d35dbd-1d91-4a2a-a643-6870d168f4f5",
    scryfall_id = "c634273a-94b0-4104-9d10-ae522ece1fc7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Adagia, Windswept Bastion",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "no DSL shape for Station's counters (\"equal to its power\") or for the 12+ ability's \"except it's legendary\"",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "Station (Tap another creature you control: Put
        // charge counters equal to its power on this Planet. Station only as
        // a sorcery.)" — `cost!(TapOther(…))` is payable and
        // `ActivationTiming::SorcerySpeed` says the sorcery clause, but no
        // `Amount` reads the power of the creature that paid the cost, and
        // nothing hands a cost-paid object to the effect, so the counters
        // cannot be counted.
        // NOT SUPPORTED: "12+ | {3}{W}, {T}: Create a token that's a copy of
        // target artifact or enchantment you control, except it's legendary.
        // Activate only as a sorcery." — `Condition::CountersOnSelf(Charge,
        // 12)` and `cost!("{3}{W}", TapSelf)` would carry the gate and the
        // cost, but `Effect::CreateTokenCopyOf` carries no `mods`, so
        // "except it's legendary" has nowhere to go (`CopyMod::
        // RemoveSupertype` is reachable only through
        // `CreateTokenCopyOfEquipped`, which copies the equipped creature
        // instead).
    ],
);
