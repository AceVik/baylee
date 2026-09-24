//! Argoth, Sanctum of Nature — (no cost) — Land
//! Oracle: This land enters tapped unless you control a legendary green creature.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}{G}, {T}: Create a 2/2 green Bear creature token, then mill three cards. Activate only as a sorcery.
//! Oracle: (Melds with Titania, Voice of Gaea.)
//! Set: BRO #256 — The Brothers' War | Scryfall ID: b29c9e4f-7b98-4610-a681-ae6297e8fc72 | Oracle ID: 62648946-1708-48f4-ba40-c057563ab11b
// PARTIAL — the enters-tapped clause (`EnterModifier::TappedUnless`),
// "{T}: Add {G}" and the Bear ability are built; meld is not modelled.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARGOTH_SANCTUM_OF_NATURE,
    oracle_id = "62648946-1708-48f4-ba40-c057563ab11b",
    scryfall_id = "b29c9e4f-7b98-4610-a681-ae6297e8fc72",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argoth, Sanctum of Nature",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::And(&[
            Filter::LEGENDARY_CREATURE,
            Filter::ControlledByYou,
            Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
        ]))],
    ),],
    coverage = Coverage::Partial(
        "meld is not modelled: no AbilityDef or engine rule melds this land with Titania, Voice of Gaea (which is not in the pool)",
    ),
    // NOT SUPPORTED: "(Melds with Titania, Voice of Gaea.)" — no `AbilityDef`
    // says meld.
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{2}{G}{G}", TapSelf),
            &[
                Effect::CreateToken {
                    token: &generated_tokens::BEAR_2_2_GREEN,
                },
                Effect::Mill {
                    amount: Amount::Fixed(3),
                    target: PlayerRel::You,
                },
            ],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
