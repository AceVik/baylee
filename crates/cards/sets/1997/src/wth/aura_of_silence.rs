//! Aura of Silence — {1}{W}{W} — Enchantment
//! Oracle: Artifact and enchantment spells your opponents cast cost {2} more to cast.
//! Oracle: Sacrifice this enchantment: Destroy target artifact or enchantment.
//! Set: C15 #60 — Commander 2015 | Scryfall ID: 8223756e-6e63-455d-9e86-5608863b7643 | Oracle ID: e7faf8eb-e829-4109-8dfe-42865a23ba86
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static OPPONENT_ARTIFACT_OR_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::ARTIFACT_OR_ENCHANTMENT,
]);

card!(
    index = index::AURA_OF_SILENCE,
    oracle_id = "e7faf8eb-e829-4109-8dfe-42865a23ba86",
    scryfall_id = "8223756e-6e63-455d-9e86-5608863b7643",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aura of Silence",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(
            OPPONENT_ARTIFACT_OR_ENCHANTMENT,
            Modifier::SpellsCostMore(2)
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(
                &Filter::ARTIFACT_OR_ENCHANTMENT
            ))],
            target = Some(TargetSpec::Object(&Filter::ARTIFACT_OR_ENCHANTMENT)),
        ),
    ],
);
