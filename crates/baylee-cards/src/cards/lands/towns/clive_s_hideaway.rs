//! Clive's Hideaway — (no cost) — Land — Town
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You may play the exiled card without paying its mana cost if you control four or more legendary creatures.
//! Set: FIN #275 — Final Fantasy | Scryfall ID: 5e43c36f-b8a2-4b2b-b2ea-57e6fa97521c | Oracle ID: 283f743f-6e79-49de-b7ed-08e6ffb64cc6
// PARTIAL — only {T}: Add {C} is built; Hideaway 4 and the exiled-card ability are outside the DSL (see the NOT SUPPORTED lines).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLIVE_S_HIDEAWAY,
    oracle_id = "283f743f-6e79-49de-b7ed-08e6ffb64cc6",
    scryfall_id = "5e43c36f-b8a2-4b2b-b2ea-57e6fa97521c",
    faces = &[face!(
        name = "Clive's Hideaway",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::TOWN],
    ),],
    coverage = Coverage::Partial(
        "Hideaway 4 needs a face-down linked exile from the library that something remembers for \
         later, and no effect or permission plays a card from exile without paying its cost"
    ),
    abilities = &[
        // NOT SUPPORTED: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.) — Effect::LookAtTopPick puts the kept card into hand rather than exiling it face down, so the mechanic is not expressible.
        // NOT SUPPORTED: {2}, {T}: You may play the exiled card without paying its mana cost if you control four or more legendary creatures. — no Effect plays a card from exile, and there is no play-from-exile permission.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
