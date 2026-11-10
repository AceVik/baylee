//! Chocobo Camp — (no cost) — Land
//! Oracle: This land enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {G}. When you next cast a Bird creature spell this turn, it enters with an additional +1/+1 counter on it.
//! Oracle: {2}{G}{G}, {T}: Create a 2/2 green Bird creature token with "Whenever a land you control enters, this token gets +1/+0 until end of turn."
//! Set: FIC #462 — Final Fantasy Commander | Scryfall ID: 4fcd08ad-4dac-4236-9030-f59e473b3ec7 | Oracle ID: ed77fdf2-59c0-4310-9b12-80d28beeaeef
// IMPLEMENTED — conditional entry, green mana with the delayed next-Bird
// entry-counter trigger, and a Bird token with its own landfall ability.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::CHOCOBO_CAMP,
    oracle_id = "ed77fdf2-59c0-4310-9b12-80d28beeaeef",
    scryfall_id = "4fcd08ad-4dac-4236-9030-f59e473b3ec7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Chocobo Camp",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(your LEGENDARY_CREATURE))],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Green, 1),
            Effect::WhenNextSpellCastThisTurn {
                filter: &Filter::And(&[Filter::CREATURE, Filter::HasSubtype(creature::BIRD)]),
                effects: &[Effect::EventSpellEntersWithP1P1 { n: 1 }],
            },
        ]),
        activated!(
            cost!("{2}{G}{G}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::BIRD_2_2_GREEN,
            }],
        ),
    ],
);
