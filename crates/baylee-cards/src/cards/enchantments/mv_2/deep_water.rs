//! Deep Water — {U}{U} — Enchantment
//! Oracle: {U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.
//! Set: DRK #23 — The Dark | Scryfall ID: 9dd6a230-6bc0-499c-b7fd-4aaa2569f98f | Oracle ID: cf9ef041-5396-4f18-8233-42d667db0888
// IMPLEMENTED — {U}: until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEEP_WATER,
    oracle_id = "cf9ef041-5396-4f18-8233-42d667db0888",
    scryfall_id = "9dd6a230-6bc0-499c-b7fd-4aaa2569f98f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deep Water",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::YOUR_LAND,
                modifier: Modifier::ReplaceTappedMana {
                    from: ManaColor::White,
                    to: ManaColor::Blue,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::YOUR_LAND,
                modifier: Modifier::ReplaceTappedMana {
                    from: ManaColor::Black,
                    to: ManaColor::Blue,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::YOUR_LAND,
                modifier: Modifier::ReplaceTappedMana {
                    from: ManaColor::Red,
                    to: ManaColor::Blue,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::YOUR_LAND,
                modifier: Modifier::ReplaceTappedMana {
                    from: ManaColor::Green,
                    to: ManaColor::Blue,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::YOUR_LAND,
                modifier: Modifier::ReplaceTappedMana {
                    from: ManaColor::Colorless,
                    to: ManaColor::Blue,
                },
                duration: Duration::UntilEndOfTurn,
            },
        ]
    ),],
);
