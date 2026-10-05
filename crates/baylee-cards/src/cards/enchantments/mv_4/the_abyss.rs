//! The Abyss — {3}{B} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, destroy target nonartifact creature that player controls of their choice. It can't be regenerated.
//! Set: ME3 #77 — Masters Edition III | Scryfall ID: f11db51c-bbbc-4890-960e-d8a3eacca1e5 | Oracle ID: a02a7816-c967-4503-bb08-f8db44915250
// PARTIAL — each player's upkeep destroys one of that player's nonartifact
// creatures of their choice, but as an untargeted choice, and the
// can't-be-regenerated rider is off (see the NOT SUPPORTED lines).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_ABYSS,
    oracle_id = "a02a7816-c967-4503-bb08-f8db44915250",
    scryfall_id = "f11db51c-bbbc-4890-960e-d8a3eacca1e5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no trigger names a chooser other than the ability's controller, so \
         \"that player controls of their choice\" is written as the untargeted \
         `DestroyChosenForPlayers`, and that effect has no \
         can't-be-regenerated rider"
    ),
    faces = &[face!(
        name = "The Abyss",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    // NOT SUPPORTED: "destroy target nonartifact creature that player controls
    // of their choice" — a triggered ability's `TargetReq` has no targeting
    // player (the ability's controller chooses every target), so the
    // destruction is written with `Effect::DestroyChosenForPlayers`, which
    // keeps the affected player's choice but targets nothing (hexproof,
    // shroud and protection no longer keep a creature alive, and the choice
    // happens as the ability resolves rather than as it is put on the stack).
    // NOT SUPPORTED: "It can't be regenerated." — `Effect::Destroy { no_regen }`
    // has the field but needs a target chosen by the ability's controller;
    // `DestroyChosenForPlayers` has no such field, so a regeneration shield
    // still saves the chosen creature.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::DestroyChosenForPlayers {
            who: PlayerRel::ActivePlayer,
            filter: &Filter::And(&[Filter::CREATURE, Filter::LacksType(TypeSet::ARTIFACT)])
        }]
    ),],
);
