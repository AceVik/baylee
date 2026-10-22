//! Canopy Dragon — {4}{G}{G} — Creature — Dragon
//! Oracle: Trample
//! Oracle: {1}{G}: This creature gains flying and loses trample until end of turn.
//! Set: MIR #209 — Mirage | Scryfall ID: 20c5e4d6-b716-4994-b226-b1eb799bec25 | Oracle ID: a9e59ee4-02a6-498c-82f0-f4f2ff334d65
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CANOPY_DRAGON,
    oracle_id = "a9e59ee4-02a6-498c-82f0-f4f2ff334d65",
    scryfall_id = "20c5e4d6-b716-4994-b226-b1eb799bec25",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Canopy Dragon",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[activated!(
        cost!("{1}{G}"),
        &[
            Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifySource {
                modifier: Modifier::RemoveKeyword(KeywordSet::TRAMPLE),
                duration: Duration::UntilEndOfTurn,
            },
        ],
    )],
);
