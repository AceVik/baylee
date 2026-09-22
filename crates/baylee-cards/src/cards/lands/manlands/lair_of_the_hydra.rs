//! Lair of the Hydra — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {X}{G}: Until end of turn, this land becomes an X/X green Hydra creature. It's still a land. X can't be 0.
//! Set: AFR #259 — Adventures in the Forgotten Realms | Scryfall ID: b670bb0f-680f-4036-bdb6-ac73e866a398 | Oracle ID: 126e9140-2c05-4c00-8b01-5653456c736a
// PARTIAL — the tapped entry, {T}: Add {G}, and the animation (+creature,
// +Hydra, +green, X/X until end of turn, with X announced as the cost is
// paid); "X can't be 0" is the one clause with no field to hold it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::LAIR_OF_THE_HYDRA,
    oracle_id = "126e9140-2c05-4c00-8b01-5653456c736a",
    scryfall_id = "b670bb0f-680f-4036-bdb6-ac73e866a398",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "\"X can't be 0\" has nowhere to be written: a Cost says no minimum \
         for its X, so the engine offers the announcement from 0 upward and \
         a player may animate this land into a 0/0 that a state-based \
         action buries"
    ),
    faces = &[face!(
        name = "Lair of the Hydra",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessAtMost {
            filter: &Filter::YOUR_LAND,
            at_most: 1,
        }],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "X can't be 0". The number itself is announced
        // (CR 602.2b, through `Engine::activation_x`) and bounded above by
        // what the pool can pay; the lower bound is the half with no field
        // on a `Cost`, so this land can be animated into a 0/0.
        activated!(
            cost!("{X}{G}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::HYDRA),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Green])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::SetPTFilter {
                    filter: &Filter::This,
                    power: Amount::X,
                    toughness: Amount::X,
                    duration: Duration::UntilEndOfTurn,
                },
            ]
        ),
    ],
);
