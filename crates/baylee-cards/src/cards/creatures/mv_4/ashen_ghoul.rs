//! Ashen Ghoul — {3}{B} — Creature — Zombie
//! Oracle: Haste
//! Oracle: {B}: Return this card from your graveyard to the battlefield. Activate only during your upkeep and only if three or more creature cards are above this card.
//! Set: ME2 #78 — Masters Edition II | Scryfall ID: 65373ddc-7ca0-4284-be3c-ebdd09c1ccb3 | Oracle ID: 46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASHEN_GHOUL,
    oracle_id = "46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22",
    scryfall_id = "65373ddc-7ca0-4284-be3c-ebdd09c1ccb3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::HASTE,
    faces = &[face!(
        name = "Ashen Ghoul",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}"),
        &[Effect::reanimate(TargetSpec::ThisObject)],
        zone = ActivationZone::Graveyard,
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
            Condition::GraveyardCardsAbove(&Filter::CREATURE, 3),
        ]))
    )],
);
