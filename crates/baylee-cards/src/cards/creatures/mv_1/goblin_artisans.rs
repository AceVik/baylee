//! Goblin Artisans — {R} — Creature — Goblin Artificer
//! Oracle: {T}: Flip a coin. If you win the flip, draw a card. If you lose the flip, counter target artifact spell you control that isn't the target of an ability from another creature named Goblin Artisans.
//! Set: CHR #48 — Chronicles | Scryfall ID: b4e8d779-ef6f-4b09-870e-f1fdeac83e32 | Oracle ID: ec85a375-fe38-4b11-af0e-f2b466181dd7
// PARTIAL — the coin-flip ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_ARTISANS,
    oracle_id = "ec85a375-fe38-4b11-af0e-f2b466181dd7",
    scryfall_id = "b4e8d779-ef6f-4b09-870e-f1fdeac83e32",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Artisans",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "the ability's whole resolution is a coin flip and the DSL has no \
         coin-flip effect, so the win and lose branches have nothing that \
         chooses between them",
    ),
    // NOT SUPPORTED: "{T}: Flip a coin. If you win the flip, draw a card. If
    // you lose the flip, counter target artifact spell you control that isn't
    // the target of an ability from another creature named Goblin Artisans." —
    // both branches are ordinary effects (Effect::draw(1) and
    // Effect::CounterTargetSpell over TargetSpec::Spell), but no Effect or
    // ReplacementRule flips a coin (the engine's GameRng has no vocabulary, as
    // Bottle of Suleiman and Ydwen Efreet also record), and the "isn't the
    // target of an ability from another creature named Goblin Artisans"
    // restriction has no Filter that asks what a spell is already targeted by.
    // So the ability comes off the card rather than resolving one branch for
    // free.
);
