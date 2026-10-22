//! Avizoa — {3}{U} — Creature — Jellyfish
//! Oracle: Flying
//! Oracle: {0}: This creature gets +2/+2 until end of turn. You skip your next untap step. Activate only once each turn.
//! Set: WTH #35 — Weatherlight | Scryfall ID: a993986c-e8f1-41b1-86e6-c72021c53b87 | Oracle ID: 91520db8-2d50-4c17-9f0e-2f92f63285f7
// NOT SUPPORTED: skipping your next untap step is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVIZOA,
    oracle_id = "91520db8-2d50-4c17-9f0e-2f92f63285f7",
    scryfall_id = "a993986c-e8f1-41b1-86e6-c72021c53b87",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "has flying and activates {0} once each turn to get +2/+2 until end of turn; skipping your next untap step is not yet modeled",
    ),
    faces = &[face!(
        name = "Avizoa",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::JELLYFISH],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!("{0}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        limit = ActivationLimit::PerTurn(1),
    )],
);
