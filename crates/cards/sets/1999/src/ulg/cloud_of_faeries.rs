//! Cloud of Faeries — {1}{U} — Creature — Faerie
//! Oracle: Flying
//! Oracle: When this creature enters, untap up to two lands.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: MOC #219 — March of the Machine Commander | Scryfall ID: 03aff125-8962-4a0a-a2b4-e5d693b75400 | Oracle ID: adc1f3b4-cce7-42c9-a208-dbab1d1924f5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLOUD_OF_FAERIES,
    oracle_id = "adc1f3b4-cce7-42c9-a208-dbab1d1924f5",
    scryfall_id = "03aff125-8962-4a0a-a2b4-e5d693b75400",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cloud of Faeries",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::UntapChosen {
                filter: &Filter::LAND,
                count: 2,
            }],
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
