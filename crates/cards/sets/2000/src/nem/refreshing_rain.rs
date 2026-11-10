//! Refreshing Rain — {3}{G} — Instant
//! Oracle: If an opponent controls a Swamp and you control a Forest, you may cast this spell without paying its mana cost.
//! Oracle: Target player gains 6 life.
//! Set: NEM #110 — Nemesis | Scryfall ID: c5e24850-bd9b-40ab-878f-b8a554da1956 | Oracle ID: c6f69168-71c2-41a2-ab95-cfd7baf0ba68
// NOT SUPPORTED: conditional free alternative cost when an opponent controls a Swamp and you control a Forest is not supported
// PARTIAL — base {3}{G} target player gains 6 life implemented; conditional free alternative cost when an opponent controls a Swamp and you control a Forest is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REFRESHING_RAIN,
    oracle_id = "c6f69168-71c2-41a2-ab95-cfd7baf0ba68",
    scryfall_id = "c5e24850-bd9b-40ab-878f-b8a554da1956",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "conditional free alternative cost when an opponent controls a Swamp and you control a Forest is not supported",
    ),
    faces = &[face!(
        name = "Refreshing Rain",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GainLifeFor {
            amount: Amount::Fixed(6),
            who: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    ),],
);
