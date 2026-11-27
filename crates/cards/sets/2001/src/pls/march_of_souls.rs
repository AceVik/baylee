//! March of Souls — {4}{W} — Sorcery
//! Oracle: Destroy all creatures. They can't be regenerated. For each creature destroyed this way, its controller creates a 1/1 white Spirit creature token with flying.
//! Set: PLS #10 — Planeshift | Scryfall ID: f07dd0f1-b80b-4af0-ae76-907ec55ec7d5 | Oracle ID: bca7101b-e74b-42af-ac63-1b3defb4137d
// IMPLEMENTED — non-regenerable creature wrath creating 1/1 white flying Spirit tokens per destroyed creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARCH_OF_SOULS,
    oracle_id = "bca7101b-e74b-42af-ac63-1b3defb4137d",
    scryfall_id = "f07dd0f1-b80b-4af0-ae76-907ec55ec7d5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "March of Souls",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyAllThenForEachDestroyed {
        filter: &Filter::CREATURE,
        no_regen: true,
        effects: &[Effect::CreateTokensFor {
            player: PlayerRel::ControllerOfEvent,
            token: &crate::generated_tokens::SPIRIT_1_1_WHITE_FLYING,
            amount: Amount::Fixed(1),
        }],
    }])],
);
