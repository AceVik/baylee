//! Dalkovan Encampment — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Swamp or a Mountain.
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Whenever you attack this turn, create two 1/1 red Warrior creature tokens that are tapped and attacking. Sacrifice them at the beginning of the next end step.
//! Set: TDM #253 — Tarkir: Dragonstorm | Scryfall ID: 98ad5f0c-8775-4e89-8e92-84a6ade93e35 | Oracle ID: 33a90122-7280-4481-9b97-5879194cae40
// IMPLEMENTED — enters tapped unless you control a Swamp or a Mountain,
// {T}: Add {W}, and {2}{W}, {T}: for the rest of the turn, whenever you attack,
// two 1/1 red Warriors enter tapped and attacking and are sacrificed at the
// beginning of the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_OR_MOUNTAIN: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::MOUNTAIN),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::DALKOVAN_ENCAMPMENT,
    oracle_id = "33a90122-7280-4481-9b97-5879194cae40",
    scryfall_id = "98ad5f0c-8775-4e89-8e92-84a6ade93e35",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dalkovan Encampment",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&SWAMP_OR_MOUNTAIN)],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // "{2}{W}, {T}: Whenever you attack this turn, create two 1/1 red
        // Warrior creature tokens that are tapped and attacking. Sacrifice
        // them at the beginning of the next end step." The delayed trigger
        // outlives this land (2025-04-04 ruling) and fires once per
        // declaration of attackers.
        activated!(
            cost!("{2}{W}", TapSelf),
            &[Effect::WheneverYouAttackThisTurn {
                effects: &[Effect::CreateTokenAttacking {
                    token: &crate::generated_tokens::WARRIOR_1_1_RED,
                    amount: Amount::Fixed(2),
                    sacrifice_at_next_end_step: true,
                }],
            }],
        ),
    ],
);
