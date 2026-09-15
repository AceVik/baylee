//! Kor Haven — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{W}, {T}: Prevent all combat damage that would be dealt by target attacking creature this turn.
//! Set: NEM #141 — Nemesis | Scryfall ID: 3d5529ca-5c20-4dfd-8595-96d6dfa6debe | Oracle ID: 276cece9-f9f2-46e6-ae76-daddaa2fb9ab
// IMPLEMENTED — {C} mana + attacking-creature damage prevention.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOR_HAVEN,
    oracle_id = "276cece9-f9f2-46e6-ae76-daddaa2fb9ab",
    scryfall_id = "3d5529ca-5c20-4dfd-8595-96d6dfa6debe",
    faces = &[face!(
        name = "Kor Haven",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            )],
            target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE))
        ),
    ],
);
