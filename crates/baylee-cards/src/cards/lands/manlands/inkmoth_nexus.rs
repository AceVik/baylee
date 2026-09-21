//! Inkmoth Nexus — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}: This land becomes a 1/1 Phyrexian Blinkmoth artifact creature with flying and infect until end of turn. It's still a land. (It deals damage to creatures in the form of -1/-1 counters and to players in the form of poison counters.)
//! Set: MBS #145 — Mirrodin Besieged | Scryfall ID: ec50c1c3-885e-47d3-ada7-cc0edbf09df1 | Oracle ID: 675281ff-b81f-4e5e-9f85-9f8cd202b50b
// PARTIAL — {T} for {C}, and {1} animates this land itself (artifact
// creature, both subtypes, 1/1 and flying, all until end of turn); infect has
// no bit the engine reads.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::INKMOTH_NEXUS,
    oracle_id = "675281ff-b81f-4e5e-9f85-9f8cd202b50b",
    scryfall_id = "ec50c1c3-885e-47d3-ada7-cc0edbf09df1",
    faces = &[face!(name = "Inkmoth Nexus", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "infect: the engine reads no infect bit, so the animated land's damage is ordinary damage"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "… and infect" (with its reminder text) — infect is not one of the keyword bits the engine reads.
        activated!(
            cost!("{1}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::PHYREXIAN),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::BLINKMOTH),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(1, 1),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
