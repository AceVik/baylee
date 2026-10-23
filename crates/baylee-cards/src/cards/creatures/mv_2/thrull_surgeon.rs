//! Thrull Surgeon — {1}{B} — Creature — Thrull
//! Oracle: {1}{B}, Sacrifice this creature: Look at target player's hand and choose a card from it. That player discards that card. Activate only as a sorcery.
//! Set: TPR #121 — Tempest Remastered | Scryfall ID: 20fffe54-4c9e-454d-bc2a-cb1fb48a6f49 | Oracle ID: ead5a2bf-f48a-43a2-89bb-df3cc587c843

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRULL_SURGEON,
    oracle_id = "ead5a2bf-f48a-43a2-89bb-df3cc587c843",
    scryfall_id = "20fffe54-4c9e-454d-bc2a-cb1fb48a6f49",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Privately looking at target player's hand to choose a card to discard is not distinguished from revealing the hand; reveals target player's hand and chooses a card from it for that player to discard",
    ),
    faces = &[face!(
        name = "Thrull Surgeon",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(1),
        toughness = Some(1),
        abilities = &[activated!(
            cost!("{1}{B}", SacrificeSelf),
            &[Effect::RevealHandDiscard {
                filter: &Filter::Any,
            }],
            target = Some(TargetSpec::AnyPlayer),
            timing = ActivationTiming::SorcerySpeed,
        )],
    ),],
);
