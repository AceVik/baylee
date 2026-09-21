//! Fanatic of Rhonas — {1}{G} — Creature — Snake Druid
//! Oracle: {T}: Add {G}.
//! Oracle: Ferocious — {T}: Add {G}{G}{G}{G}. Activate only if you control a creature with power 4 or greater.
//! Oracle: Eternalize {2}{G}{G} ({2}{G}{G}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Snake Druid with no mana cost. Eternalize only as a sorcery.)
//! Set: MH3 #152 — Modern Horizons 3 | Scryfall ID: 1f9fb33a-3b39-4aff-93b8-aedafe0ea694 | Oracle ID: 7973820b-fdaf-46ec-9e3e-d4c0e77b5067
// PARTIAL — the plain "{T}: Add {G}" is built; the ferocious {T} and
// Eternalize have no spelling in the DSL, see the NOT SUPPORTED lines below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FANATIC_OF_RHONAS,
    oracle_id = "7973820b-fdaf-46ec-9e3e-d4c0e77b5067",
    scryfall_id = "1f9fb33a-3b39-4aff-93b8-aedafe0ea694",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fanatic of Rhonas",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "the ferocious gate needs a power predicate no Filter carries, and Eternalize needs an activation from a graveyard that creates a modified copy",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "Ferocious — {T}: Add {G}{G}{G}{G}. Activate only if
        // you control a creature with power 4 or greater." A `Condition` can
        // count permanents (`Condition::ControlCount`), but `Filter` has no
        // predicate over power — it carries `CmcAtMost`/`CmcAtLeast` and
        // `ToughnessAtMost` and nothing for "power 4 or greater" — so the gate
        // cannot be said. The ability comes off the card rather than being
        // offered ungated, which would hand out four mana with a 1/1 Snake on
        // the battlefield.
        // NOT SUPPORTED: "Eternalize {2}{G}{G} ({2}{G}{G}, Exile this card
        // from your graveyard: Create a token that's a copy of it, except it's
        // a 4/4 black Zombie Snake Druid with no mana cost. Eternalize only as
        // a sorcery.)" `ActivationZone` has `Battlefield` and `Hand` and no
        // graveyard, and no `CopyMod` sets power and toughness to 4/4.
    ],
);
