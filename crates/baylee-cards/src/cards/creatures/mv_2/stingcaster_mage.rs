//! Stingcaster Mage — {1}{R} — Creature — Human Wizard
//! Oracle: Haste
//! Oracle: When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
//! Set: FRA #93 — Reality Fracture | Scryfall ID: 2d8e8e5f-3bf5-490d-aa9c-9df2b26f1460 | Oracle ID: 056b651e-e0e2-4333-9235-d1ffe8fcca29
// IMPLEMENTED — haste, plus Snapcaster Mage's ETB: a targeted instant or
// sorcery in your own graveyard gains flashback for its mana cost until end
// of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STINGCASTER_MAGE,
    oracle_id = "056b651e-e0e2-4333-9235-d1ffe8fcca29",
    scryfall_id = "2d8e8e5f-3bf5-490d-aa9c-9df2b26f1460",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Stingcaster Mage",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(1),
    ),],
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::GrantFlashback],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        )))
    )],
);

// Engine-level test belongs in baylee-engine's card_tests: no test plays
// `Effect::GrantFlashback` yet — Snapcaster Mage appears in combo_tests only
// as a printed Wizard body — so the ETB grant, the cast from the graveyard
// for its mana cost and the exile afterwards want one game each.
