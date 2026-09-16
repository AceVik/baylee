//! Dwalin, Weaponmaster — {1}{R/W} — Legendary Creature — Dwarf Warrior
//! Oracle: First strike
//! Oracle: Whenever Dwalin enters or attacks, put a hone counter on each Equipment you control. (Each hone counter on an Equipment grants +1/+0 to equipped creature.)
//! Set: HOB #154 — The Hobbit | Scryfall ID: 196d9287-a37d-4b27-a83b-a5489a54f081 | Oracle ID: cee583b7-7cc3-40ea-a227-b760839ec291
// PARTIAL — first strike only; the enter/attack hone-counter trigger is not
// expressible with this vocabulary.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWALIN_WEAPONMASTER,
    oracle_id = "cee583b7-7cc3-40ea-a227-b760839ec291",
    scryfall_id = "196d9287-a37d-4b27-a83b-a5489a54f081",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Dwalin, Weaponmaster",
        mana_cost = mana!("{1}{R/W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(1),
    ),],
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "the hone-counter trigger: `baylee_cards_dsl::counters` assigns no \
         hone id, and no Modifier grants +1/+0 per counter the attached \
         Equipment wears",
    ),
);

// NOT SUPPORTED: "Whenever Dwalin enters or attacks, put a hone counter on
// each Equipment you control." — two gaps, and the first is only the smaller
// one. The counter has to be *named*, and a card may not write a bare
// `CounterKind::Custom(n)`: ids are assigned in `baylee_cards_dsl::counters`
// and there is no `HONE` there, so `Effect::AddCounterFilter` has nothing to
// name. The half that matters is the reminder text: "Each hone counter on an
// Equipment grants +1/+0 to equipped creature" is a bonus granted to the
// *equipped creature* by counters sitting on another permanent, and no
// `Modifier` reads counters off the object the source is attached to —
// `Modifier::ModifyPTPerCount` counts *permanents* its controller controls,
// and `AddTypeIfCountersAtLeast`/`AddKeywordIfCountersAtLeast` grant types and
// keywords, never P/T. ("Enters or attacks" itself is no blocker: it is the
// two triggers `Trigger::ETB` and `Trigger::Attacks(&Filter::This)`.) Dropping
// only the counters would ship Equipment that mints a counter worth nothing,
// so the whole ability comes off the card and the keyword stays.
