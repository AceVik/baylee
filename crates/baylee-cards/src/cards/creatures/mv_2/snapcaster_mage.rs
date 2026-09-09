//! Snapcaster Mage — {1}{U} — Creature — Human Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
//! Set: INR #478 — Innistrad Remastered | Scryfall ID: 22b36ad5-bf4d-436a-9c3c-fa4acd0052fe | Oracle ID: 2bb2eda7-3b38-4c56-870f-c3218a1056f5
// IMPLEMENTED — flash + flashback grant until EOT (cast from graveyard,
// exile on resolution).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 148,
    oracle_id: "2bb2eda7-3b38-4c56-870f-c3218a1056f5",
    scryfall_id: "22b36ad5-bf4d-436a-9c3c-fa4acd0052fe",
    faces: &[face! {
        name: "Snapcaster Mage",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(2),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::GrantFlashback], targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))))],
}
