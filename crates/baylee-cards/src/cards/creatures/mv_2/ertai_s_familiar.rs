//! Ertai's Familiar — {1}{U} — Creature — Illusion
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: When this creature phases out or leaves the battlefield, mill three cards.
//! Oracle: {U}: Until your next upkeep, this creature can't phase out.
//! Set: WTH #38 — Weatherlight | Scryfall ID: 354c9de7-0cdf-4302-9d1a-ae17eca13053 | Oracle ID: 711cd80d-c58d-4c2a-87db-d2e4c75f60d3
// NOT SUPPORTED: triggering when Ertai's Familiar phases out and preventing all phase-out effects (not only the phasing keyword) require a PhasesOut trigger and a CantPhaseOut modifier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERTAI_S_FAMILIAR,
    oracle_id = "711cd80d-c58d-4c2a-87db-d2e4c75f60d3",
    scryfall_id = "354c9de7-0cdf-4302-9d1a-ae17eca13053",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::PHASING,
    coverage = Coverage::Partial(
        "has phasing, mills three cards when it leaves the battlefield, and {U} removes phasing until your next upkeep; triggering when it phases out and preventing all phase-out effects (not only the phasing keyword) require a PhasesOut trigger and a CantPhaseOut modifier",
    ),
    faces = &[face!(
        name = "Ertai's Familiar",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::PHASING,
    ),],
    abilities = &[
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }],
        ),
        activated!(
            cost!("{U}"),
            &[Effect::continuous(
                &Filter::This,
                Modifier::RemoveKeyword(KeywordSet::PHASING),
                Duration::UntilYourNextUpkeep,
            )],
        ),
    ],
);
