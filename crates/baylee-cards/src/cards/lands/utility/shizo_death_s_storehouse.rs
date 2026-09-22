//! Shizo, Death's Storehouse — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}: Target legendary creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: DMC #233 — Dominaria United Commander | Scryfall ID: 099352e2-38c8-4fb4-a25f-6d928aa20f9e | Oracle ID: 008f2698-1721-45a3-8353-10f2f400dc8f
// IMPLEMENTED — {T}: Add {B}. The {B}, {T} ability is dropped: fear is a
// keyword no rule in the engine reads.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIZO_DEATH_S_STOREHOUSE,
    oracle_id = "008f2698-1721-45a3-8353-10f2f400dc8f",
    scryfall_id = "099352e2-38c8-4fb4-a25f-6d928aa20f9e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shizo, Death's Storehouse",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the {B}, {T} ability grants fear, and fear is a dead keyword bit — \
         no rule reads it and no Effect can say the evasion it stands for"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "{B}, {T}: Target legendary creature gains fear until end
// of turn." The engine reads exactly these keyword bits: flying, first
// strike, double strike, deathtouch, haste, hexproof, shroud,
// indestructible, lifelink, menace, reach, trample, vigilance, defender,
// flash, prowess, changeling, unblockable, uncounterable, rebound,
// daybound, nightbound. Fear is not one of them, and it is not a
// `Modifier` or a `PumpTarget` keyword either — the clause is "can't be
// blocked except by artifact creatures and/or black creatures", which is a
// blocking restriction with no variant in this vocabulary. Writing it as a
// `PumpTarget` with an empty `KeywordSet` would compile, resolve and
// change nothing, so the ability comes off the card instead.
