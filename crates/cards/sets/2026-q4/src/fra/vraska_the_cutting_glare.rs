//! Vraska, the Cutting Glare — {B}{B}{G} — Legendary Creature — Gorgon Assassin
//! Oracle: Deathtouch
//! Oracle: When Vraska enters, if you control six or more lands, destroy target permanent an opponent controls. They create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: FRA #278 — Reality Fracture | Scryfall ID: 5c28b012-5efb-488f-a1c1-09e2dddfd6ee | Oracle ID: 580de511-863c-4b7a-9d2e-fcdf6e12d54b
// IMPLEMENTED — deathtouch, six-land entry condition and victim-controller Treasure.

use crate::tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VRASKA_THE_CUTTING_GLARE,
    oracle_id = "580de511-863c-4b7a-9d2e-fcdf6e12d54b",
    scryfall_id = "5c28b012-5efb-488f-a1c1-09e2dddfd6ee",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Vraska, the Cutting Glare",
        mana_cost = mana!("{B}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GORGON, subtypes::creature::ASSASSIN],
        power = Some(4),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ControlledByOpponent)),
            Effect::CreateTokenForTargetController {
                token: &tokens::TREASURE
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ControlledByOpponent
        ))),
        condition = Some(Condition::ControlCount(&Filter::YOUR_LAND, 6))
    )],
);
