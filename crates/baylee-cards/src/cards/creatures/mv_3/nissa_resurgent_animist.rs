//! Nissa, Resurgent Animist — {2}{G} — Legendary Creature — Elf Scout
//! Oracle: Landfall — Whenever a land you control enters, add one mana of any color. Then if this is the second time this ability has resolved this turn, reveal cards from the top of your library until you reveal an Elf or Elemental card. Put that card into your hand and the rest on the bottom of your library in a random order.
//! Set: MAT #22 — March of the Machine: The Aftermath | Scryfall ID: 248c76d3-b5cb-4582-be17-7cd1d0cb0f58 | Oracle ID: c1fc5923-c3cd-448a-98d1-c154661c2812
// PARTIAL — the landfall trigger adds one mana of any color; the reveal
// clause on the trigger's second resolution has no DSL variant (below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NISSA_RESURGENT_ANIMIST,
    oracle_id = "c1fc5923-c3cd-448a-98d1-c154661c2812",
    scryfall_id = "248c76d3-b5cb-4582-be17-7cd1d0cb0f58",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Nissa, Resurgent Animist",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "the reveal-until clause on the landfall trigger's second resolution \
         is not expressible"
    ),
    abilities = &[
        // NOT SUPPORTED: "Then if this is the second time this ability has
        // resolved this turn, reveal cards from the top of your library
        // until you reveal an Elf or Elemental card. Put that card into your
        // hand and the rest on the bottom of your library in a random
        // order." — no `Condition` counts a trigger's resolutions this turn
        // (the four counting sentences are permanents, an opponent's
        // graveyard, counters on the source, and a filter on the source), no
        // `Effect` reveals cards until one matches a filter (`SearchLibrary`
        // is a chosen search that shuffles, `LookAtTopPick` a fixed count),
        // and none sends the rest to the bottom in a random order.
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::mana_of_any_color()],
        ),
    ],
);
