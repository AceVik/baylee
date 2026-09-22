//! Lair of the Hydra — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {X}{G}: Until end of turn, this land becomes an X/X green Hydra creature. It's still a land. X can't be 0.
//! Set: AFR #259 — Adventures in the Forgotten Realms | Scryfall ID: b670bb0f-680f-4036-bdb6-ac73e866a398 | Oracle ID: 126e9140-2c05-4c00-8b01-5653456c736a
// IMPLEMENTED — the tapped entry, {T}: Add {G}, and the animation
// (+creature, +Hydra, +green, X/X until end of turn); "X can't be 0" is a
// Coverage::Partial reason.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::LAIR_OF_THE_HYDRA,
    oracle_id = "126e9140-2c05-4c00-8b01-5653456c736a",
    scryfall_id = "b670bb0f-680f-4036-bdb6-ac73e866a398",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "X is never announced at all: the engine asks for a number on an \
         activation only for a counter cost (CR 602.2b is unimplemented for \
         mana), so {X}{G} is paid as {G} and this land becomes a 0/0 that \
         dies. \"X can't be 0\" is the smaller half and is also not \
         expressible, because a Cost says no minimum for its X"
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
        // NOT SUPPORTED: the announced X. `abilities.rs` reaches
        // `Pending::ChooseNumber` for an activation only through
        // `counter_x_part`, which reads a counter cost and nothing about
        // mana — so nobody is asked, X is 0, and the land becomes a 0/0
        // that a state-based action buries. "X can't be 0" is the second
        // half and needs a minimum a `Cost` cannot say. Three more cards in
        // this pool price an activation with a mana {X}: Treasure Vault,
        // Kessig Wolf Run and Blast Zone.
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
