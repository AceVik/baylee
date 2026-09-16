//! Strangleroot Geist — {G}{G} — Creature — Spirit
//! Oracle: Haste
//! Oracle: Undying (When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: DKA #127 — Dark Ascension | Scryfall ID: bf1fb137-205c-480f-b6dc-dfa137793ae3 | Oracle ID: af12758f-4a7b-4156-8942-de4716aa0623
// PARTIAL — haste is a keyword bit the engine reads (keywords field only);
// undying is not built, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRANGLEROOT_GEIST,
    oracle_id = "af12758f-4a7b-4156-8942-de4716aa0623",
    scryfall_id = "bf1fb137-205c-480f-b6dc-dfa137793ae3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "undying: no keyword bit to set, and no effect that returns the source from the graveyard with a +1/+1 counter"
    ),
    faces = &[face!(
        name = "Strangleroot Geist",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(1),
        keywords = KeywordSet::HASTE,
    ),],
);

// NOT SUPPORTED: Undying — "When this creature dies, if it had no +1/+1
// counters on it, return it to the battlefield under its owner's control with
// a +1/+1 counter on it." No `KeywordSet` bit carries it, so the keyword
// itself cannot be stated; and the nearest vocabulary that could spell it out
// — `triggered!(Trigger::Dies(&Filter::This), &[…])` carrying
// `Condition::CountersOnSelfExactly(CounterKind::P1P1, 0)` over
// `Effect::GraveyardToBattlefield` plus `Effect::AddCounter` — would ask for
// the counters of a source that has already left the battlefield (where
// `Condition::SourceMatches`' own rule says a departed source no longer
// matches anything), while the keyword reads last known information. Built
// that way the card would be a Geist that returns even when it died wearing
// the +1/+1 counter — worse than the stub it replaces.
