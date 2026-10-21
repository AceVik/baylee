//! Bounty of the Hunt — {3}{G}{G} — Instant
//! Oracle: You may exile a green card from your hand rather than pay this spell's mana cost.
//! Oracle: Distribute three +1/+1 counters among one, two, or three target creatures. For each +1/+1 counter you put on a creature this way, remove a +1/+1 counter from that creature at the beginning of the next cleanup step.
//! Set: ME2 #154 — Masters Edition II | Scryfall ID: 412161bf-52b9-4e43-8866-aafd28dc2189 | Oracle ID: 77a43c24-398f-42c6-a0d3-b28b33a0ce12
// NOT SUPPORTED: distributing the three +1/+1 counters across up to three target creatures and removing them at the beginning of the next cleanup step are not supported.

use baylee_cards_dsl::prelude::*;

static GREEN_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));

card!(
    index = index::BOUNTY_OF_THE_HUNT,
    oracle_id = "77a43c24-398f-42c6-a0d3-b28b33a0ce12",
    scryfall_id = "412161bf-52b9-4e43-8866-aafd28dc2189",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "may be cast by exiling a green card from your hand rather than paying its mana cost, and puts three +1/+1 counters on target creature; distributing those counters among up to three target creatures and removing them at the beginning of the next cleanup step are not supported",
    ),
    faces = &[face!(
        name = "Bounty of the Hunt",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&GREEN_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
