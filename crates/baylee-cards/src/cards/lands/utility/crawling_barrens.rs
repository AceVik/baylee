//! Crawling Barrens — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}: Put two +1/+1 counters on this land. Then you may have it become a 0/0 Elemental creature until end of turn. It's still a land.
//! Set: FDN #685 — Foundations | Scryfall ID: ac2aff0e-1319-4d3b-903c-fa1ce3db7602 | Oracle ID: dfe1a112-97aa-4e81-8431-81552ba2cdcf
// IMPLEMENTED — {T}: Add {C} plus {4}: two +1/+1 counters on this land and
// the optional animation into a 0/0 Elemental that is still a land.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "Then you may have it become a 0/0 Elemental creature until end of turn.
/// It's still a land."
///
/// The whole clause is one `MayDo`, because the printed "you may" covers all
/// of it — the two counters above happen whether or not the land animates.
///
/// Nothing here replaces a type: the land gains Creature and Elemental on top
/// of what it already is, which is what "it's still a land" says. The P/T is
/// set at layer 7b and the two +1/+1 counters are applied at 7c, after it, so
/// the animated land is a 2/2 and not a 0/0 wearing counters nothing reads.
const ANIMATE: &[Effect] = &[
    Effect::continuous(
        &Filter::This,
        Modifier::AddType(TypeSet::CREATURE),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddSubtype(creature::ELEMENTAL),
        Duration::UntilEndOfTurn,
    ),
    Effect::SetPTFilter {
        filter: &Filter::This,
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(0),
        duration: Duration::UntilEndOfTurn,
    },
];

card!(
    index = index::CRAWLING_BARRENS,
    oracle_id = "dfe1a112-97aa-4e81-8431-81552ba2cdcf",
    scryfall_id = "ac2aff0e-1319-4d3b-903c-fa1ce3db7602",
    faces = &[face!(name = "Crawling Barrens", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}"),
            &[
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(2),
                },
                Effect::MayDo { effects: ANIMATE },
            ]
        ),
    ],
);
