//! Hall of Storm Giants — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {5}{U}: Until end of turn, this land becomes a 7/7 blue Giant creature with ward {3}. It's still a land. (Whenever it becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)
//! Set: AFR #257 — Adventures in the Forgotten Realms | Scryfall ID: bf8f052d-8840-4905-a807-9a305f4fd8f7 | Oracle ID: 087c8c0e-a91c-4e3c-8387-9312db01f343
// PARTIAL — the fastland entry condition, {T}: Add {U}, and the {5}{U}
// animation (creature, Giant, blue, 7/7, still a land); the animation's
// ward {3} has no Modifier to grant it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::HALL_OF_STORM_GIANTS,
    oracle_id = "087c8c0e-a91c-4e3c-8387-9312db01f343",
    scryfall_id = "bf8f052d-8840-4905-a807-9a305f4fd8f7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hall of Storm Giants",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessAtMost {
            filter: &Filter::YOUR_LAND,
            at_most: 1,
        }],
    )],
    coverage = Coverage::Partial(
        "the {5}{U} animation's ward {3}: no Modifier grants a Ward ability to a permanent"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "…becomes a 7/7 blue Giant creature with ward {3}" —
        // the animation is written without ward {3}, because nothing in the
        // DSL attaches an AbilityDef::Ward to a permanent a continuous effect
        // animates.
        activated!(
            cost!("{5}{U}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::GIANT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Blue])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(7, 7),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
