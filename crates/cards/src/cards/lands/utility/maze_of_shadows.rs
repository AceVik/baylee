//! Maze of Shadows — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Untap target attacking creature with shadow. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Set: TPR #238 — Tempest Remastered | Scryfall ID: db3ea047-d18c-4f2a-913e-17469d3bd14c | Oracle ID: b7b51ab1-403e-4640-8827-b04965aa6760

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAZE_OF_SHADOWS,
    oracle_id = "b7b51ab1-403e-4640-8827-b04965aa6760",
    scryfall_id = "db3ea047-d18c-4f2a-913e-17469d3bd14c",
    faces = &[face!(name = "Maze of Shadows", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[
                Effect::UntapTarget,
                Effect::continuous(
                    &Filter::This,
                    Modifier::PreventDamageToIt,
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::PreventDamageFromIt,
                    Duration::UntilEndOfTurn
                )
            ],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::ATTACKING_CREATURE,
                Filter::HasKeyword(KeywordSet::SHADOW)
            ])))
        ),
    ],
);
