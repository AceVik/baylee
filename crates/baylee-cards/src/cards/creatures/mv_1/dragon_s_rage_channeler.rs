//! Dragon's Rage Channeler — {R} — Creature — Human Shaman
//! Oracle: Whenever you cast a noncreature spell, surveil 1. (Look at the top card of your library. You may put that card into your graveyard.)
//! Oracle: Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able.
//! Set: MH2 #121 — Modern Horizons 2 | Scryfall ID: 4ced112a-e775-4f97-97b3-74877e9dce12 | Oracle ID: 0c016ccc-a341-4b76-87ba-69c639d2746d
// PARTIAL — the noncreature-spell surveil trigger is built; the Delirium
// clause is not expressible at all.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_S_RAGE_CHANNELER,
    oracle_id = "0c016ccc-a341-4b76-87ba-69c639d2746d",
    scryfall_id = "4ced112a-e775-4f97-97b3-74877e9dce12",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dragon's Rage Channeler",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "Delirium: no static ability can be conditioned on the card types in your graveyard, and no Modifier says attacks each combat if able"
    ),
    abilities = &[
        triggered!(
            Trigger::SpellCast(&f!(your NONCREATURE)),
            &[Effect::surveil(1)]
        ),
        // NOT SUPPORTED: "Delirium — As long as there are four or more card
        // types among cards in your graveyard, this creature gets +2/+2, has
        // flying, and attacks each combat if able." — a static ability
        // carries no condition (`Condition` has no graveyard-card-types
        // sentence, and `StaticAbility` has no condition field at all), no
        // `Filter` predicate counts card types in a graveyard, and no
        // `Modifier` says "attacks each combat if able".
    ],
);
