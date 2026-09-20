//! Hammerheim — (no cost) — Legendary Land
//! Oracle: {T}: Add {R}.
//! Oracle: {T}: Target creature loses all landwalk abilities until end of turn.
//! Set: ME3 #207 — Masters Edition III | Scryfall ID: 773acde9-504c-4065-95e0-f96428f05a7d | Oracle ID: c7476beb-7923-4994-8476-bc69187ecb72
// IMPLEMENTED — {T}: Add {R}; the second ability has no DSL variant and is
// dropped (see NOT SUPPORTED below and the Coverage::Partial reason).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAMMERHEIM,
    oracle_id = "c7476beb-7923-4994-8476-bc69187ecb72",
    scryfall_id = "773acde9-504c-4065-95e0-f96428f05a7d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Hammerheim",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the second printed ability — \"{T}: Target creature loses all landwalk \
         abilities until end of turn\" — is not expressible: landwalk is not one \
         of the keyword bits the engine reads, so a filter for it could not be \
         matched, and Modifier::LoseKeywords strips every keyword rather than the \
         landwalk family, which is a different (and much larger) sentence"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: "{T}: Target creature loses all landwalk abilities
        // until end of turn." There is no Modifier that removes a named family
        // of keywords — Modifier::LoseKeywords removes all of them, and
        // landwalk carries no keyword bit for a filter to reach in the first
        // place. The ability is left off rather than approximated.
    ],
);
