//! Nighthawk Scavenger — {1}{B}{B} — Creature — Vampire Rogue
//! Oracle: Flying, deathtouch, lifelink
//! Oracle: Nighthawk Scavenger's power is equal to 1 plus the number of card types among cards in your opponents' graveyards.
//! Set: OTC #144 — Outlaws of Thunder Junction Commander | Scryfall ID: 47433ce9-1ac6-4359-a5e4-4ec68490cb0c | Oracle ID: 379ed4ec-8f2e-448b-9ed2-a3181e2f877e
// IMPLEMENTED — layer-7a power counts each card type once across opposing
// graveyards only; toughness remains the printed three.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static POWER: PtCount = PtCount::Plus {
    base: &PtCount::CardTypesInOpponentsGraveyards,
    offset: 1,
};

card!(
    index = index::NIGHTHAWK_SCAVENGER,
    oracle_id = "379ed4ec-8f2e-448b-9ed2-a3181e2f877e",
    scryfall_id = "47433ce9-1ac6-4359-a5e4-4ec68490cb0c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING
        .union(KeywordSet::DEATHTOUCH)
        .union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nighthawk Scavenger",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: POWER,
            toughness: PtCount::Fixed(3),
        }
    )],
);
