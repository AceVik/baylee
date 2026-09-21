//! Power Depot — (no cost) — Artifact Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast artifact spells or activate abilities of artifacts.
//! Oracle: Modular 1
//! Set: MH2 #251 — Modern Horizons 2 | Scryfall ID: 032eba04-0a63-4823-85e1-64861330a8d7 | Oracle ID: 64687880-03f9-4f38-985b-1027c797e33f
// IMPLEMENTED — enters tapped wearing its modular +1/+1 counter; {T}: {C};
// {T}: one mana of any color restricted to artifact spells.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Modular 1's dies trigger ("when this dies, you may put its
// +1/+1 counters on target artifact creature") — no `Amount` reads the
// counters of a kind on the source.
// NOT SUPPORTED: "or activate abilities of artifacts" — a `ManaRestriction`
// is read against spells, so it can only say the first half of the sentence.

card!(
    index = index::POWER_DEPOT,
    oracle_id = "64687880-03f9-4f38-985b-1027c797e33f",
    scryfall_id = "032eba04-0a63-4823-85e1-64861330a8d7",
    faces = &[face!(
        name = "Power Depot",
        types = TypeSet::ARTIFACT.union(TypeSet::LAND),
        enter_modifiers = &[
            EnterModifier::Tapped,
            EnterModifier::WithCounters {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
    coverage = Coverage::Partial(
        "Modular 1's dies trigger needs an Amount for the counters on the \
         source, and a ManaRestriction cannot cover \"activate abilities of \
         artifacts\"",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&Filter::ARTIFACT, SpendRider::None),
        ]),
    ],
);
