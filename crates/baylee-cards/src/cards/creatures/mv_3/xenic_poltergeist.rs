//! Xenic Poltergeist — {1}{B}{B} — Creature — Spirit
//! Oracle: {T}: Until your next upkeep, target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.
//! Set: ME4 #104 — Masters Edition IV | Scryfall ID: 37055986-85e0-4568-a301-b8726da23b75 | Oracle ID: fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c
// PARTIAL — the animation's "until your next upkeep" duration has no Duration
// variant, so the ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::XENIC_POLTERGEIST,
    oracle_id = "fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c",
    scryfall_id = "37055986-85e0-4568-a301-b8726da23b75",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Xenic Poltergeist",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "the animation lasts until your next upkeep, and Duration has no \
         until-your-next-upkeep variant: UntilYourNextTurn expires at the \
         start of that turn, before the untap step the animation should \
         survive, and UntilYourNextUntapStep is a different boundary",
    ),
    // NOT SUPPORTED: "{T}: Until your next upkeep, target noncreature artifact
    // becomes an artifact creature with power and toughness each equal to its
    // mana value." — the cost (Cost::TAP), the target (an artifact that is not
    // a creature) and the animation (Effect::continuous with Filter::This and
    // Modifier::AnimateNoncreatureArtifact, whose layer-4 half adds creature
    // and whose layer-7b half sets both values to the mana value) are all
    // sayable, but no Duration ends at the controller's next upkeep. So the
    // ability comes off the card rather than shipping an animation that
    // outlasts its printed window.
);
