//! Inner Calm, Outer Strength — {2}{G} — Instant — Arcane
//! Oracle: Target creature gets +X/+X until end of turn, where X is the number of cards in your hand.
//! Set: SOK #133 — Saviors of Kamigawa | Scryfall ID: 40f56817-aabf-469c-a82c-37315decc73c | Oracle ID: b7bdbae5-549f-403b-83ca-4f9a1ac93e93
// IMPLEMENTED — one target creature gets +X/+X until end of turn, X being the
// cards in your hand (Amount::CountOf over ZoneSel::HandYou), read at
// resolution.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "…where X is the number of cards in your hand."
///
/// Counted in the *hand* zone rather than on the battlefield, and `Filter::Any`
/// is every object there — a hand holds cards and nothing else.
const CARDS_IN_HAND: Amount = Amount::CountOf {
    filter: &Filter::Any,
    zone: ZoneSel::HandYou,
};

card!(
    index = index::INNER_CALM_OUTER_STRENGTH,
    oracle_id = "b7bdbae5-549f-403b-83ca-4f9a1ac93e93",
    scryfall_id = "40f56817-aabf-469c-a82c-37315decc73c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Inner Calm, Outer Strength",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
        subtypes = &[subtypes::spell::ARCANE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: CARDS_IN_HAND,
            toughness: CARDS_IN_HAND,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
