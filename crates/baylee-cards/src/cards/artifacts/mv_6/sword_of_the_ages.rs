//! Sword of the Ages — {6} — Artifact
//! Oracle: This artifact enters tapped.
//! Oracle: {T}, Sacrifice this artifact and any number of creatures you control: This artifact deals X damage to any target, where X is the total power of the creatures sacrificed this way, then exile this artifact and those creature cards.
//! Set: ME3 #202 — Masters Edition III | Scryfall ID: aad10ff3-6d8d-4de4-9c30-1ae1aec85e5e | Oracle ID: 201f2434-96b3-408c-a5ce-74d0675920ed
// PARTIAL — "enters tapped" is built; the activated ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SWORD_OF_THE_AGES,
    oracle_id = "201f2434-96b3-408c-a5ce-74d0675920ed",
    scryfall_id = "aad10ff3-6d8d-4de4-9c30-1ae1aec85e5e",
    faces = &[face!(
        name = "Sword of the Ages",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the sacrifice ability is off the card: no cost sacrifices any number \
         of creatures, no amount reads the total power of what was \
         sacrificed, and no effect exiles the cards that paid an activation \
         cost"
    ),
    // NOT SUPPORTED: "{T}, Sacrifice this artifact and any number of
    // creatures you control: This artifact deals X damage to any target,
    // where X is the total power of the creatures sacrificed this way, then
    // exile this artifact and those creature cards." — `CostPart::Sacrifice`
    // is one permanent per part and there is no "any number" part;
    // `Amount::SacrificedManaValue` exists but no amount reads the total
    // power of what was sacrificed; and no `Effect` exiles the cards that
    // paid an activation cost (`Effect::ExileSource` is the source alone).
);
