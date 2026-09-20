//! R&D's Secret Lair — (no cost) — Legendary Land
//! Oracle: Play cards as written. Ignore all errata.
//! Oracle: {T}: Add {C}. (This mana is still added to your mana pool.)
//! Set: UNH #135 — Unhinged | Scryfall ID: 9bfdc6a9-0a44-43ef-b065-02ef5d6110dc | Oracle ID: b6be7abe-cee3-418f-bf52-8b5405e3462f
// PARTIAL — the {T}: Add {C} mana ability; the un-rules sentence has no
// DSL spelling and no engine reader.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::R_D_S_SECRET_LAIR,
    oracle_id = "b6be7abe-cee3-418f-bf52-8b5405e3462f",
    scryfall_id = "9bfdc6a9-0a44-43ef-b065-02ef5d6110dc",
    faces = &[face!(
        name = "R&D's Secret Lair",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial("Play cards as written. Ignore all errata."),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "Play cards as written. Ignore all errata." — errata is not
// a category the engine has: a card is compiled from its printed text and
// there is nothing else for it to be read as. No `Effect`, `Modifier` or
// `StaticAbility` variant states anything about it, and a `Filter::Any`
// static with no modifier would be a second name for a rule that does not
// exist here.
