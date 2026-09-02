//! Flawless Maneuver — {2}{W} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Creatures you control gain indestructible until end of turn.
//! Set: CMM #24 — Commander Masters | Scryfall ID: ab12f69e-1491-47a8-8c46-d85bbf637ff6 | Oracle ID: 4e183439-17d2-47ff-9d99-5e22821d91e3
// IMPLEMENTED — commander-conditional free cast + team indestructible.

static YOUR_CREATURES: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 51,
    oracle_id: "4e183439-17d2-47ff-9d99-5e22821d91e3",
    scryfall_id: "ab12f69e-1491-47a8-8c46-d85bbf637ff6",
    faces: &[face! {
        name: "Flawless Maneuver",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::INSTANT,
        alternative_costs: &[AlternativeCost {
            cost: Cost::FREE,
            condition: AltCondition::CommanderControlled,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CreateContinuousEffect {
            layer: Layer::Ability,
            filter: &YOUR_CREATURES,
            modifier: Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
            duration: Duration::UntilEndOfTurn,
        }])],
}
