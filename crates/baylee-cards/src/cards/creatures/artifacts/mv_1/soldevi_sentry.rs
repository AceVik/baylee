//! Soldevi Sentry — {1} — Artifact Creature — Soldier
//! Oracle: {1}: Choose target opponent. Regenerate this creature. When it regenerates this way, that player may draw a card.
//! Set: ALL #132a — Alliances | Scryfall ID: 85976b5c-4eed-4cf9-b2b0-a8421a97ab2a | Oracle ID: d2a61132-d881-4c80-92f1-a151d835832a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDEVI_SENTRY,
    oracle_id = "d2a61132-d881-4c80-92f1-a151d835832a",
    scryfall_id = "85976b5c-4eed-4cf9-b2b0-a8421a97ab2a",
    faces = &[face!(
        name = "Soldevi Sentry",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "The {1} ability regenerates this creature and immediately lets target opponent draw a card on resolution rather than triggering when the regeneration shield is used."
    ),
    abilities = &[activated!(
        cost!("{1}"),
        &[
            Effect::Regenerate {
                target: TargetSpec::ThisObject,
            },
            Effect::MayDoFor {
                player: PlayerRel::Chosen,
                effects: &[Effect::DrawCardsFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::Chosen,
                }],
            },
        ],
        target = Some(TargetSpec::AnyOpponent)
    )],
);
