//! Mutated Cultist — {2}{B} — Creature — Eldrazi Horror
//! Oracle: Devoid (This card has no color.)
//! Oracle: When you cast this spell, remove all counters from up to one target permanent or opponent. The next spell you cast this turn costs {1} less to cast for each counter removed this way.
//! Oracle: Deathtouch
//! Set: M3C #53 — Modern Horizons 3 Commander | Scryfall ID: cc4bb871-cde9-43a9-8089-5987fa0dd4ef | Oracle ID: 5168be9b-ffca-41c5-a944-92a1a2e30779
// IMPLEMENTED — devoid/deathtouch, optional permanent-or-opponent cleansing,
// and the actual removed count reducing only the caster's next spell this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MUTATED_CULTIST,
    oracle_id = "5168be9b-ffca-41c5-a944-92a1a2e30779",
    scryfall_id = "cc4bb871-cde9-43a9-8089-5987fa0dd4ef",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mutated Cultist",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELDRAZI, subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::DEVOID.union(KeywordSet::DEATHTOUCH),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::This),
        &[
            Effect::RemoveAllCountersTarget,
            Effect::NextSpellThisTurnCostsLess(Amount::ThisWay(
                ThisWay::CountersRemovedFromTargets,
            )),
        ],
        targets = Some(TargetReq::up_to_one(TargetSpec::OpponentOrObject(
            &Filter::Any,
        )))
    )],
);
