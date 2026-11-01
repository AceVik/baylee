//! Surge of Strength — {R}{G} — Instant
//! Oracle: As an additional cost to cast this spell, discard a red or green card.
//! Oracle: Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value.
//! Set: ALL #109 — Alliances | Scryfall ID: 96fff700-af02-4861-b7ed-be9950e69bf1 | Oracle ID: bd3e55cf-ffca-402c-91ea-2420035a9dd1
// IMPLEMENTED — additional cost: discard a red or green card; the target gains trample and +X/+0 until end of turn, X its mana value.

use baylee_cards_dsl::prelude::*;

static RED_OR_GREEN_CARD: Filter =
    Filter::HasColor(ColorSet::from_slice(&[Color::Red, Color::Green]));

card!(
    index = index::SURGE_OF_STRENGTH,
    oracle_id = "bd3e55cf-ffca-402c-91ea-2420035a9dd1",
    scryfall_id = "96fff700-af02-4861-b7ed-be9950e69bf1",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Surge of Strength",
        mana_cost = mana!("{R}{G}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Discard(&RED_OR_GREEN_CARD)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::TargetCmc,
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::TRAMPLE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
