//! Zanarkand, Ancient Metropolis // Lasting Fayth — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Create a 1/1 colorless Hero creature token. Put a +1/+1 counter on it for each land you control. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #293 — Final Fantasy | Scryfall ID: 881e4c00-3b9a-47a1-bf66-1badda994c88 | Oracle ID: 5f2b3ea8-99ee-47a4-8a1c-4b27478d524c
//! Face: Zanarkand, Ancient Metropolis —  — Land — Town
//! Face: Lasting Fayth — {4}{G}{G} — Sorcery — Adventure
// PARTIAL — the land: it enters tapped and taps for {G}. The adventure half is
// NOT SUPPORTED; the exact reason is on the back face below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZANARKAND_ANCIENT_METROPOLIS,
    oracle_id = "5f2b3ea8-99ee-47a4-8a1c-4b27478d524c",
    scryfall_id = "881e4c00-3b9a-47a1-bf66-1badda994c88",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Zanarkand, Ancient Metropolis",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        // NOT SUPPORTED: "Create a 1/1 colorless Hero creature token. Put a
        // +1/+1 counter on it for each land you control." — two things stop it.
        // The pool's token registry carries no Hero token for
        // `Effect::CreateToken` to name, and a card file may not define one
        // (`tokens::no_card_file_defines_its_own_token`). And no effect puts
        // counters on a token the same resolution created: `Effect::AddCounter`
        // counts onto the first target or onto the source, `AddCounterFilter`
        // onto every object a filter matches, and `CreateTokenPtPerCount`
        // grants +P/+T for a count rather than the counters this card prints.
        face!(
            name = "Lasting Fayth",
            mana_cost = mana!("{4}{G}{G}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
    coverage = Coverage::Partial(
        "Lasting Fayth: no Hero token in the registry, and no effect puts counters on the token the resolution just created",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
