//! Spawning Pool — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}: This land becomes a 1/1 black Skeleton creature with "{B}: Regenerate this creature" until end of turn. It's still a land. (If it regenerates, the next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: 10E #358 — Tenth Edition | Scryfall ID: 907b49ff-2020-4203-b93e-4b3306afc337 | Oracle ID: f3bf22cf-0a6f-4fb6-ba82-63ce290308d6
// IMPLEMENTED — enters tapped, {T}: Add {B}, and the {1}{B} animation to a
// 1/1 black Skeleton that is still a land and can regenerate itself.

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
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
                // The quoted ability the animation hands over, and the
                // reason it is a fifth continuous effect rather than a
                // printed one: the land has it only while it is a
                // Skeleton. `TargetSpec::ThisObject` inside a grant is the
                // *granted* ability's source, which is this land — so
                // "regenerate this creature" regenerates whatever is
                // carrying the ability, exactly as the quotation marks
                // say.
                Effect::continuous(
                    &Filter::This,
                    Modifier::GrantActivated {
                        cost: cost!("{B}"),
                        effects: &[Effect::regenerate(TargetSpec::ThisObject)],
                        mana_ability: false,
                    },
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);
