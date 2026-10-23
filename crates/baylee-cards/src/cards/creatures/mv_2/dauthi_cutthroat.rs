//! Dauthi Cutthroat — {1}{B} — Creature — Dauthi Minion
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: {1}{B}, {T}: Destroy target creature with shadow.
//! Set: EXO #57 — Exodus | Scryfall ID: 127b8994-fff8-4500-8ab4-244eeb3ed110 | Oracle ID: 5fdde593-54cb-4186-974b-df4383f7efe2
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITH_SHADOW: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);

card!(
    index = index::DAUTHI_CUTTHROAT,
    oracle_id = "5fdde593-54cb-4186-974b-df4383f7efe2",
    scryfall_id = "127b8994-fff8-4500-8ab4-244eeb3ed110",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dauthi Cutthroat",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::MINION],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{B}", TapSelf),
        &[Effect::destroy(TargetSpec::Object(&CREATURE_WITH_SHADOW))],
        targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_SHADOW))),
    )],
);
