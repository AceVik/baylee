//! Selenia, Dark Angel — {3}{W}{B} — Legendary Creature — Phyrexian Angel
//! Oracle: Flying
//! Oracle: Pay 2 life: Return Selenia to its owner's hand.
//! Set: TPR #210 — Tempest Remastered | Scryfall ID: 664b57e3-9476-4d6a-8301-5a2f96bcfbf7 | Oracle ID: 3db1ac09-e35a-4f50-97c4-f65dbc28daef

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SELENIA_DARK_ANGEL,
    oracle_id = "3db1ac09-e35a-4f50-97c4-f65dbc28daef",
    scryfall_id = "664b57e3-9476-4d6a-8301-5a2f96bcfbf7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    keywords = KeywordSet::FLYING,
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Selenia, Dark Angel",
        mana_cost = mana!("{3}{W}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!(PayLife(2)),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
