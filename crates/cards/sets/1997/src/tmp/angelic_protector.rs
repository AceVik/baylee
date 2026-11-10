//! Angelic Protector — {3}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: Whenever this creature becomes the target of a spell or ability, this creature gets +0/+3 until end of turn.
//! Set: TPR #2 — Tempest Remastered | Scryfall ID: 21c85b48-9603-4db3-95d2-b5ce794e4634 | Oracle ID: cff434e1-e567-4a80-8dc5-593d70b0f348
// IMPLEMENTED — Flying and +0/+3 until end of turn whenever this creature becomes the target of a spell or ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANGELIC_PROTECTOR,
    oracle_id = "cff434e1-e567-4a80-8dc5-593d70b0f348",
    scryfall_id = "21c85b48-9603-4db3-95d2-b5ce794e4634",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angelic Protector",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(3),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
);
