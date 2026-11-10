//! Serra's Emissary — {4}{W}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: As this creature enters, choose a card type.
//! Oracle: You and creatures you control have protection from the chosen card type.
//! Set: FRC #32 — Reality Fracture Commander | Scryfall ID: 0a69b668-7b7c-4da9-af49-e8cbc13797cc | Oracle ID: 7a56c6e1-0509-4783-9b29-cf3163977166
// IMPLEMENTED — the chosen card type is retained on this permanent; both
// protection abilities read that same choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERRA_S_EMISSARY,
    oracle_id = "7a56c6e1-0509-4783-9b29-cf3163977166",
    scryfall_id = "0a69b668-7b7c-4da9-af49-e8cbc13797cc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serra's Emissary",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(7),
        toughness = Some(7),
        enter_modifiers = &[EnterModifier::ChooseCardType],
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::PlayerProtectionFrom {
                who: PlayerRel::You,
                from: &Filter::OfSourceChosenCardType,
            }
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
            Modifier::ProtectionFrom(&Filter::OfSourceChosenCardType)
        ),
    ],
);
