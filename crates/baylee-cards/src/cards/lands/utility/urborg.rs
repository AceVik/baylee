//! Urborg — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Target creature loses first strike or swampwalk until end of turn.
//! Set: ME3 #214 — Masters Edition III | Scryfall ID: 5319f782-2713-43b1-9b28-0d1ec7a39203 | Oracle ID: b6114962-035e-4e7f-9009-4739bf83a05a
// PARTIAL — `{T}: Add {B}` is built. The second ability is a choose-one
// between two keywords, and the DSL has no modal *activated* ability (only
// ModalSpell and ModalTriggered); swampwalk is additionally not a keyword the
// engine reads, so neither arm could be said on its own.
// NOT SUPPORTED: {T}: Target creature loses first strike or swampwalk until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBORG,
    oracle_id = "b6114962-035e-4e7f-9009-4739bf83a05a",
    scryfall_id = "5319f782-2713-43b1-9b28-0d1ec7a39203",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no modal activated ability for a choose-one between first strike and swampwalk; swampwalk is not a keyword the engine reads"
    ),
    faces = &[face!(
        name = "Urborg",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
