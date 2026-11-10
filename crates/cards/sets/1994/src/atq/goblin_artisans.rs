//! Goblin Artisans — {R} — Creature — Goblin Artificer
//! Oracle: {T}: Flip a coin. If you win the flip, draw a card. If you lose the flip, counter target artifact spell you control that isn't the target of an ability from another creature named Goblin Artisans.
//! Set: CHR #48 — Chronicles | Scryfall ID: b4e8d779-ef6f-4b09-870e-f1fdeac83e32 | Oracle ID: ec85a375-fe38-4b11-af0e-f2b466181dd7
// IMPLEMENTED — Effect::FlipCoin: a won flip draws, a lost one counters the
// target artifact spell you control that no other Goblin Artisans' ability
// targets (Filter::NotTargetedByAnotherNamed).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_UNCLAIMED_ARTIFACT_SPELL: Filter = Filter::And(&[
    Filter::ARTIFACT,
    Filter::ControlledByYou,
    Filter::NotTargetedByAnotherNamed("Goblin Artisans"),
]);

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
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::FlipCoin {
            won: &[Effect::draw(1)],
            lost: &[Effect::CounterTargetSpell],
        }],
        target = Some(TargetSpec::Spell(&YOUR_UNCLAIMED_ARTIFACT_SPELL))
    )],
);
