//! Invigorate — {2}{G} — Instant
//! Oracle: If you control a Forest, rather than pay this spell's mana cost, you may have an opponent gain 3 life.
//! Oracle: Target creature gets +4/+4 until end of turn.
//! Set: 2XM #172 — Double Masters | Scryfall ID: e0899da3-7beb-4161-81a3-e2d694e5b8a5 | Oracle ID: 9264c198-76c7-4ac0-9334-37c691de1b54
// NOT SUPPORTED: optional alternative cost (having an opponent gain 3 life if you control a Forest) not supported
// PARTIAL — omits optional alternative cost (having an opponent gain 3 life if you control a Forest).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INVIGORATE,
    oracle_id = "9264c198-76c7-4ac0-9334-37c691de1b54",
    scryfall_id = "e0899da3-7beb-4161-81a3-e2d694e5b8a5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "optional alternative cost (having an opponent gain 3 life if you control a Forest) not supported"
    ),
    faces = &[face!(
        name = "Invigorate",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(4),
            toughness: Amount::Fixed(4),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
