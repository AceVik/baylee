//! Sustainer of the Realm — {2}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: Whenever this creature blocks, it gets +0/+2 until end of turn.
//! Set: IMA #35 — Iconic Masters | Scryfall ID: e9f7e21e-d4d8-4d6c-9530-b17d5204e9e7 | Oracle ID: 2c99d52d-f909-4494-aeee-937ae779fd4a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUSTAINER_OF_THE_REALM,
    oracle_id = "2c99d52d-f909-4494-aeee-937ae779fd4a",
    scryfall_id = "e9f7e21e-d4d8-4d6c-9530-b17d5204e9e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sustainer of the Realm",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(0, 2),
            Duration::UntilEndOfTurn,
        )],
    )],
);
