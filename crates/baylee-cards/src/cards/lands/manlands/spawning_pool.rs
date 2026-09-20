//! Spawning Pool — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}: This land becomes a 1/1 black Skeleton creature with "{B}: Regenerate this creature" until end of turn. It's still a land. (If it regenerates, the next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: 10E #358 — Tenth Edition | Scryfall ID: 907b49ff-2020-4203-b93e-4b3306afc337 | Oracle ID: f3bf22cf-0a6f-4fb6-ba82-63ce290308d6
// PARTIAL — enters tapped, {T}: Add {B}, and the {1}{B} animation to a 1/1
// black Skeleton that is still a land; the ability the animation grants is
// not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::SPAWNING_POOL,
    oracle_id = "f3bf22cf-0a6f-4fb6-ba82-63ce290308d6",
    scryfall_id = "907b49ff-2020-4203-b93e-4b3306afc337",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spawning Pool",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the animation's granted \"{B}: Regenerate this creature\" is not expressible: the DSL has no regenerate effect and no regenerate keyword bit",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // NOT SUPPORTED: "… becomes a 1/1 black Skeleton creature with
        // \"{B}: Regenerate this creature\" …" — no Effect::Regenerate and no
        // regenerate keyword exists, so the land animates without it.
        activated!(
            cost!("{1}{B}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::SKELETON),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Black])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(1, 1),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);
