//! Arcum's Sleigh — {1} — Artifact
//! Oracle: {2}, {T}: Target creature gains vigilance until end of turn. Activate only during combat and only if defending player controls a snow land.
//! Set: ICE #309 — Ice Age | Scryfall ID: e9780ce2-756c-48e5-9936-45f6a224f61d | Oracle ID: 40960ec8-e3be-45f2-8d27-316d82c69123

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "and only if defending player controls a snow land" —
// Condition has no variant checking whether the defending player controls a matching permanent.
card!(
    index = index::ARCUM_S_SLEIGH,
    oracle_id = "40960ec8-e3be-45f2-8d27-316d82c69123",
    scryfall_id = "e9780ce2-756c-48e5-9936-45f6a224f61d",
    coverage = Coverage::Partial(
        "missing condition checking whether defending player controls a snow land"
    ),
    faces = &[face!(
        name = "Arcum's Sleigh",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::VIGILANCE,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::DuringCombat)
    )],
);
