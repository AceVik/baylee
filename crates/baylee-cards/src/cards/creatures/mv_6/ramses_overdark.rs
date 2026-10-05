//! Ramses Overdark — {2}{U}{U}{B}{B} — Legendary Creature — Human Assassin
//! Oracle: {T}: Destroy target enchanted creature.
//! Set: ME3 #169 — Masters Edition III | Scryfall ID: 61bb42b2-327b-47d3-9fe3-76c029d91ee9 | Oracle ID: 6c9a9071-8159-4607-9465-de796f1dc2cc
// PARTIAL — the one ability is off the card: no filter says "enchanted".

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAMSES_OVERDARK,
    oracle_id = "6c9a9071-8159-4607-9465-de796f1dc2cc",
    scryfall_id = "61bb42b2-327b-47d3-9fe3-76c029d91ee9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ramses Overdark",
        mana_cost = mana!("{2}{U}{U}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ASSASSIN],
        power = Some(4),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no filter means \"has an Aura attached\" — `Filter::AttachedToBySource` \
         asks what the source is attached to and `Filter::IsAttached` asks the \
         Aura whether it is attached, so \"target enchanted creature\" cannot \
         be named and the ability is off",
    ),
    // NOT SUPPORTED: "{T}: Destroy target enchanted creature." — the tap cost
    // and `Effect::destroy` are sayable, but no `Filter` matches a permanent
    // by the Auras attached to it, so the target requirement cannot be
    // written.
);
