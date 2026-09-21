//! Faceless Haven — (no cost) — Snow Land
//! Oracle: {T}: Add {C}.
//! Oracle: {S}{S}{S}: This land becomes a 4/3 creature with vigilance and all creature types until end of turn. It's still a land. ({S} can be paid with one mana from a snow source.)
//! Set: KHM #255 — Kaldheim | Scryfall ID: e3cd82e5-6072-4334-a493-01ca4ad6b4eb | Oracle ID: f74107d5-fb4a-464b-9251-42b84d91775d
// IMPLEMENTED — {T} for {C}, and {S}{S}{S} turning the land into a 4/3
// vigilance creature of every creature type until end of turn: four
// until-end-of-turn continuous effects on the source, no type removed, so
// it is still a land.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FACELESS_HAVEN,
    oracle_id = "f74107d5-fb4a-464b-9251-42b84d91775d",
    scryfall_id = "e3cd82e5-6072-4334-a493-01ca4ad6b4eb",
    faces = &[face!(
        name = "Faceless Haven",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::SNOW,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{S}{S}{S}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AllCreatureTypes,
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(4, 3),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::VIGILANCE),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
