//! Goblin Sappers — {1}{R} — Creature — Goblin
//! Oracle: {R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it and this creature at end of combat.
//! Oracle: {R}{R}{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it at end of combat.
//! Set: ICE #189 — Ice Age | Scryfall ID: de839540-a7b9-4f91-91df-3fd4f5c0bc4e | Oracle ID: 9e095761-cdae-4ab2-b2d3-099b0b39600a
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_SAPPERS,
    oracle_id = "9e095761-cdae-4ab2-b2d3-099b0b39600a",
    scryfall_id = "de839540-a7b9-4f91-91df-3fd4f5c0bc4e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Sappers",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{R}{R}", TapSelf),
            &[
                Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::AtEndOfCombat {
                    about: TargetSpec::Object(&Filter::YOUR_CREATURE),
                    effects: &[
                        Effect::destroy(TargetSpec::EventObject),
                        Effect::destroy(TargetSpec::ThisObject),
                    ],
                },
            ],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
        ),
        activated!(
            cost!("{R}{R}{R}{R}", TapSelf),
            &[
                Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::AtEndOfCombat {
                    about: TargetSpec::Object(&Filter::YOUR_CREATURE),
                    effects: &[Effect::destroy(TargetSpec::EventObject)],
                },
            ],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
        ),
    ],
);
