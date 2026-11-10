//! Meddling Mage — {W}{U} — Creature — Human Wizard
//! Oracle: As this creature enters, choose a nonland card name.
//! Oracle: Spells with the chosen name can't be cast.
//! Set: 2XM #210 — Double Masters | Scryfall ID: de734753-e102-489b-9160-b3f3d10be4f1 | Oracle ID: a5d02339-9f6d-486e-87d8-5ec2274cf680
// IMPLEMENTED — the entry choice remembers a nonland card name; the static
// ability prohibits every player from casting spells with that name.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MEDDLING_MAGE,
    oracle_id = "a5d02339-9f6d-486e-87d8-5ec2274cf680",
    scryfall_id = "de734753-e102-489b-9160-b3f3d10be4f1",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Meddling Mage",
        mana_cost = mana!("{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
        enter_modifiers = &[EnterModifier::ChooseCardNameExcept(TypeSet::LAND)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::PlayersCantCast {
            who: PlayerRel::EachPlayer,
            filter: &Filter::NamedBySource,
        }
    )],
);
