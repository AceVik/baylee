//! Chocobo Camp — (no cost) — Land
//! Oracle: This land enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {G}. When you next cast a Bird creature spell this turn, it enters with an additional +1/+1 counter on it.
//! Oracle: {2}{G}{G}, {T}: Create a 2/2 green Bird creature token with "Whenever a land you control enters, this token gets +1/+0 until end of turn."
//! Set: FIC #462 — Final Fantasy Commander | Scryfall ID: 4fcd08ad-4dac-4236-9030-f59e473b3ec7 | Oracle ID: ed77fdf2-59c0-4310-9b12-80d28beeaeef
// PARTIAL — the enters-tapped-unless-legendary clause, {T}: Add {G} and the
// Bird; the delayed +1/+1-counter rider is dropped (see below).

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

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
    coverage = Coverage::Partial(
        "the mana line's delayed rider — \"when you next cast a Bird creature spell this \
         turn, it enters with an additional +1/+1 counter on it\" — has no variant: no \
         effect sets up an entry replacement for a spell cast later in the turn"
    ),
    abilities = &[
        // NOT SUPPORTED: "When you next cast a Bird creature spell this turn,
        // it enters with an additional +1/+1 counter on it." — no Effect
        // creates an entry replacement (counters) pointed at a spell cast
        // later in the turn, so the mana line stands on its own.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{2}{G}{G}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::BIRD_2_2_GREEN,
            }],
        ),
    ],
);
