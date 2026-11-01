//! Scars of the Veteran — {4}{W} — Instant
//! Oracle: You may exile a white card from your hand rather than pay this spell's mana cost.
//! Oracle: Prevent the next 7 damage that would be dealt to any target this turn. If it's a creature, put a +0/+1 counter on it for each 1 damage prevented this way at the beginning of the next end step.
//! Set: ME2 #34 — Masters Edition II | Scryfall ID: d239cae2-48c4-4ac7-a41a-7e25d852ee22 | Oracle ID: 5f0ad09f-a9ee-42df-b32d-9d842ce5dd00

// PARTIAL — the white-card exile pitch cost, and "prevent the next 7 damage that would be dealt to any target this turn".
// NOT SUPPORTED: "If it's a creature, put a +0/+1 counter on it for each 1 damage prevented this way at the beginning of the next end step." Nothing counts the damage a shield prevented or puts counters at the next end step for a creature, so the creature gets no counters (the card does less than printed).

use baylee_cards_dsl::prelude::*;

static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

card!(
    index = index::SCARS_OF_THE_VETERAN,
    oracle_id = "5f0ad09f-a9ee-42df-b32d-9d842ce5dd00",
    scryfall_id = "d239cae2-48c4-4ac7-a41a-7e25d852ee22",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Scars of the Veteran",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&WHITE_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    coverage = Coverage::Partial(
        "Prevents the next 7 damage that would be dealt to any target this turn (including via the white-card exile pitch cost), without putting +0/+1 counters on a creature at the beginning of the next end step for each 1 damage prevented this way."
    ),
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::Fixed(7),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
