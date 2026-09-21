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
        "Corrupted — {T}: target 1/1 creature gets +2/+1 is missing: no Condition \
         variant counts an opponent's poison counters, and no Filter names a 1/1 \
         creature (only ToughnessAtMost, which a 0/1 or a 3/1 also matches)"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&PHYREXIAN_CREATURE_SPELL, SpendRider::None)
        ]),
        // NOT SUPPORTED: "Corrupted — {T}: Target 1/1 creature gets +2/+1
        // until end of turn. Activate only if an opponent has three or more
        // poison counters." `Condition` has ControlCount,
        // OpponentGraveyardCountAtLeast, CountersOnSelf, CountersOnSelfExactly
        // and SourceMatches, none of which reads a counter on another player,
        // and `Filter` has no power or exact-size predicate to point at "1/1
        // creature". Written without the gate the ability would pump any
        // creature at any time, so it comes off the card rather than shipping
        // ungated.
    ],
);
