//! Contagion — {3}{B}{B} — Instant
//! Oracle: You may pay 1 life and exile a black card from your hand rather than pay this spell's mana cost.
//! Oracle: Distribute two -2/-1 counters among one or two target creatures.
//! Set: ME1 #64 — Masters Edition | Scryfall ID: 90036bbd-bb67-477b-a924-d0a2bf436d34 | Oracle ID: 433e86ce-df04-4182-bb63-ad2607d67dbd
// NOT SUPPORTED: distributing the two -2/-1 counters across two target creatures is not supported.

use baylee_cards_dsl::prelude::*;

static BLACK_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));

card!(
    index = index::CONTAGION,
    oracle_id = "433e86ce-df04-4182-bb63-ad2607d67dbd",
    scryfall_id = "90036bbd-bb67-477b-a924-d0a2bf436d34",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "may be cast by paying 1 life and exiling a black card from your hand rather than paying its mana cost, and puts two -2/-1 counters on target creature; distributing those two counters across two target creatures is not supported",
    ),
    faces = &[face!(
        name = "Contagion",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(PayLife(1), ExileFromHand(&BLACK_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::Minus {
                power: 2,
                toughness: 1,
            },
            amount: Amount::Fixed(2),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
