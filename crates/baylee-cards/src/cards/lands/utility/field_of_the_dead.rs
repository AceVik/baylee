//! Field of the Dead — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: Whenever this land or another land you control enters, if you control seven or more lands with different names, create a 2/2 black Zombie creature token.
//! Set: M20 #247 — Core Set 2020 | Scryfall ID: 470ca3f4-29aa-4c4c-8ff2-8cdd70c69943 | Oracle ID: aa959340-c869-4caa-92c7-572bd8d23eef
// PARTIAL — the land half is built: it enters tapped
// (`EnterModifier::Tapped`) and taps for {C}. The token trigger is not built,
// because its intervening-if cannot be said (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIELD_OF_THE_DEAD,
    oracle_id = "aa959340-c869-4caa-92c7-572bd8d23eef",
    scryfall_id = "470ca3f4-29aa-4c4c-8ff2-8cdd70c69943",
    faces = &[face!(
        name = "Field of the Dead",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the token trigger's printed intervening-if counts lands with \
         different names, and Condition has no distinct-name count \
         (ControlCount counts permanents matching a filter); written \
         without it the trigger would fire on every land, so it is off \
         the card"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Whenever this land or another land you control
        // enters, if you control seven or more lands with different names,
        // create a 2/2 black Zombie creature token."
        //
        // The event filter ("this land or another land you control") and the
        // effect (a 2/2 black Zombie token) are both expressible — it is the
        // intervening-if (CR 603.4) that is not. `Condition::ControlCount`
        // counts permanents matching a filter, and "seven or more lands with
        // *different names*" is an aggregate over names that no `Filter`,
        // `Amount` or `Condition` in this vocabulary carries
        // (`Amount::DistinctColorsAmong` counts colours). Dropping only the
        // `if` would leave a trigger that makes a Zombie on every land drop,
        // which is not the printed card and not a thing `Coverage::Partial`
        // may excuse — so the ability comes off whole.
    ],
);
