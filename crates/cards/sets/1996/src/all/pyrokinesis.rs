//! Pyrokinesis — {4}{R}{R} — Instant
//! Oracle: You may exile a red card from your hand rather than pay this spell's mana cost.
//! Oracle: Pyrokinesis deals 4 damage divided as you choose among any number of target creatures.
//! Set: EMA #143 — Eternal Masters | Scryfall ID: 25f98f02-9ada-4561-93fd-9cfcb0f48b5d | Oracle ID: d001febc-d511-4e65-a631-91dc9415056b
// IMPLEMENTED — pitch alternative cost exiling a red card from your hand, and 4 damage divided as you choose among up to four target creatures.

use baylee_cards_dsl::prelude::*;

static RED_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::PYROKINESIS,
    oracle_id = "d001febc-d511-4e65-a631-91dc9415056b",
    scryfall_id = "25f98f02-9ada-4561-93fd-9cfcb0f48b5d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pyrokinesis",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&RED_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::Fixed(4),
        }],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 4)),
    )],
);
