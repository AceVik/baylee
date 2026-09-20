//! Archway of Innovation — (no cost) — Land
//! Oracle: This land enters tapped unless you control an Island.
//! Oracle: {T}: Add {U}.
//! Oracle: {U}, {T}: The next spell you cast this turn has improvise. (Your artifacts can help cast that spell. Each artifact you tap after you're done activating mana abilities pays for {1}.)
//! Set: MH3 #214 — Modern Horizons 3 | Scryfall ID: 6a90f9e6-9251-4203-9599-cc4032a5e6e1 | Oracle ID: bfa20bc7-4626-4a52-87f4-6e2763cb8ed5
// PARTIAL — the enters-tapped clause (unless you control an Island) and
// {T}: Add {U} are built; the improvise grant is dropped, see NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::ARCHWAY_OF_INNOVATION,
    oracle_id = "bfa20bc7-4626-4a52-87f4-6e2763cb8ed5",
    scryfall_id = "6a90f9e6-9251-4203-9599-cc4032a5e6e1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Archway of Innovation",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(
            &f!(your Filter::HasSubtype(land::ISLAND))
        )],
    ),],
    coverage =
        Coverage::Partial("the {U}, {T} ability grants improvise, which the DSL cannot say",),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);

// NOT SUPPORTED: "{U}, {T}: The next spell you cast this turn has improvise."
// No `Modifier` grants improvise and no keyword bit is read for it, so the
// ability comes off the card rather than resolving as a {U}, {T} that does
// nothing.
