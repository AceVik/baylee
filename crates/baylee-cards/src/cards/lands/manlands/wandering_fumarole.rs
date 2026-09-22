//! Wandering Fumarole — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Oracle: {2}{U}{R}: Until end of turn, this land becomes a 1/4 blue and red Elemental creature with "{0}: Switch this creature's power and toughness until end of turn." It's still a land.
//! Set: CLB #928 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: ce3a6a8e-a01e-4d14-a6e5-c02c8205c749 | Oracle ID: 741c51f1-cfbe-4c29-ac8f-ca6bcd2652f9
// IMPLEMENTED — enters tapped, {T}: Add {U} or {R}, and the {2}{U}{R}
// animation into a 1/4 blue and red Elemental that is still a land and
// carries the granted "{0}: Switch this creature's power and toughness
// until end of turn".

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::WANDERING_FUMAROLE,
    oracle_id = "741c51f1-cfbe-4c29-ac8f-ca6bcd2652f9",
    scryfall_id = "ce3a6a8e-a01e-4d14-a6e5-c02c8205c749",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Wandering Fumarole",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Red])]),
        activated!(
            cost!("{2}{U}{R}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(1, 4),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Blue, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                // The granted ability rides in the same animation, the shape
                // Urza's Saga chapter II is written in: one
                // `Effect::continuous` carrying `GrantActivated`, so the
                // switch ability appears exactly while the land is a creature
                // and leaves with it. `Cost::FREE` is the printed "{0}".
                Effect::continuous(
                    &Filter::This,
                    Modifier::GrantActivated {
                        cost: Cost::FREE,
                        effects: &[Effect::continuous(
                            &Filter::This,
                            Modifier::SwitchPT,
                            Duration::UntilEndOfTurn,
                        )],
                        mana_ability: false,
                    },
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
