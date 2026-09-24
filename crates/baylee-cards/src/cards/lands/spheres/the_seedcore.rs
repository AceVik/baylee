//! The Seedcore — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast Phyrexian creature spells.
//! Oracle: Corrupted — {T}: Target 1/1 creature gets +2/+1 until end of turn. Activate only if an opponent has three or more poison counters.
//! Set: ONE #259 — Phyrexia: All Will Be One | Scryfall ID: 29c91aad-bf33-448e-b122-65940fb2e33b | Oracle ID: 249fdd3e-376c-4ec2-a612-4353e0e61ee2
// PARTIAL — both mana abilities are built, the second one restricted to
// Phyrexian creature spells; the Corrupted ability is not expressible, see
// the NOT SUPPORTED line in the ability list.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Phyrexian creature spells" — what the restricted mana may be spent on.
static PHYREXIAN_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::PHYREXIAN),
]);

card!(
    index = index::THE_SEEDCORE,
    oracle_id = "249fdd3e-376c-4ec2-a612-4353e0e61ee2",
    scryfall_id = "29c91aad-bf33-448e-b122-65940fb2e33b",
    faces = &[face!(
        name = "The Seedcore",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
    ),],
    coverage = Coverage::Partial(
        "Corrupted — {T}: target 1/1 creature gets +2/+1 is left off: its gate \
         \"an opponent has three or more poison counters\" needs a Condition that \
         reads another player's poison counters, which does not exist, and the \
         ability is not shipped ungated"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&PHYREXIAN_CREATURE_SPELL, SpendRider::None)
        ]),
        // NOT SUPPORTED: "Corrupted — {T}: Target 1/1 creature gets +2/+1
        // until end of turn. Activate only if an opponent has three or more
        // poison counters." No `Condition` reads a counter on another player
        // (and nothing in the engine gives a player poison yet). The target is
        // sayable, Power/ToughnessAtLeast and AtMost 1; written without the
        // gate the ability would pump a 1/1 at any time, so it comes off the
        // card rather than shipping ungated.
    ],
);
