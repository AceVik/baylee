//! Angel of Condemnation — {2}{W}{W} — Creature — Angel
//! Oracle: Flying, vigilance
//! Oracle: {2}{W}, {T}: Exile another target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.
//! Oracle: {2}{W}, {T}, Exert this creature: Exile another target creature until this creature leaves the battlefield. (An exerted creature won't untap during your next untap step.)
//! Set: HOU #3 — Hour of Devastation | Scryfall ID: 90dad070-e41d-419b-ae27-ace82b9ab2c5 | Oracle ID: 2317a33f-665e-4cbc-bb5c-16b912ac2eef
// IMPLEMENTED — both targeted exile abilities, exert, flying, and vigilance.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_CREATURE: Filter = Filter::And(&[Filter::Another, Filter::CREATURE]);

card!(
    index = index::ANGEL_OF_CONDEMNATION,
    oracle_id = "2317a33f-665e-4cbc-bb5c-16b912ac2eef",
    scryfall_id = "90dad070-e41d-419b-ae27-ace82b9ab2c5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of Condemnation",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        activated!(
            cost!("{2}{W}", TapSelf),
            &[Effect::ExileAndReturnAtEndStep],
            targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_CREATURE))),
        ),
        activated!(
            cost!("{2}{W}", TapSelf, ExertSelf),
            &[Effect::exile_until(
                TargetSpec::Object(&ANOTHER_CREATURE),
                ExileUntil::SourceLeavesBattlefield,
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_CREATURE))),
        ),
    ],
);
