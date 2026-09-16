//! U.S.S. Enterprise-D, Galaxy-Class — {3} — Legendary Artifact — Spacecraft
//! Oracle: Whenever one or more charge counters are put on U.S.S. Enterprise-D for the first time each turn, exile the top card of your library. You may play that card this turn.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying, vigilance
//! Set: TRK #273 — Star Trek | Scryfall ID: 057a4413-6a17-491e-bfd7-6cd427b1a442 | Oracle ID: d95af032-3efd-40c7-8229-ade9d974934f
// PARTIAL — station (tap another creature you control, put its power in
// charge counters on this Spacecraft, only as a sorcery), the artifact
// creature at 7+, and the 7+ flying and vigilance: the mode the card is
// played for, written the way Inspirit, Flagship Vessel writes the same
// keyword one file over.
// NOT SUPPORTED: "Whenever one or more charge counters are put on
// U.S.S. Enterprise-D for the first time each turn, exile the top card of
// your library. You may play that card this turn." Two gaps, either of
// which alone would hold it. No `Trigger` variant fires on counters being
// put on an object — the enum reaches `EntersBattlefield`, `Dies`,
// `BecomesTapped`, `StepBegin` and eleven more, and not one of them is
// about a counter; `once_per_turn = true` would carry "for the first time
// each turn" the day such a trigger exists. And no `Effect` grants
// permission to play a card from exile: the nearest are `Effect::Exile`,
// which needs a target and grants nothing, and `Effect::LookAtTopPick`,
// which puts the pick into hand instead. Written with the exile half
// alone the ability would bury the top of your library and never hand it
// back — a card the printing does not describe rather than a weaker one —
// so the whole trigger is left off and the Spacecraft plays as though the
// line were not printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_ENTERPRISE_D_GALAXY_CLASS,
    oracle_id = "d95af032-3efd-40c7-8229-ade9d974934f",
    scryfall_id = "057a4413-6a17-491e-bfd7-6cd427b1a442",
    faces = &[face!(
        name = "U.S.S. Enterprise-D, Galaxy-Class",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(5),
    ),],
    coverage = Coverage::Partial("the charge-counter trigger is not written"),
    abilities = &[
        // Station: tap another creature you control, put charge counters
        // equal to its power on this Spacecraft, only as a sorcery.
        activated!(
            Cost::FREE,
            &[
                Effect::TapTarget,
                Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::Charge,
                    amount: Amount::TargetPower,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            timing = ActivationTiming::SorcerySpeed
        ),
        // "It's an artifact creature at 7+."
        static_ability!(
            Filter::This,
            Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 7,
                types: TypeSet::CREATURE,
            }
        ),
        // 7+ | Flying, vigilance
        static_ability!(
            Filter::This,
            Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 7,
                keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
            }
        ),
    ],
);
