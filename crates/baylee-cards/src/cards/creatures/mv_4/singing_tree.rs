//! Singing Tree — {3}{G} — Creature — Plant
//! Oracle: {T}: Target attacking creature has base power 0 until end of turn.
//! Set: ME1 #130 — Masters Edition | Scryfall ID: 78b11d04-aee6-48c8-b4e2-2949879a30c8 | Oracle ID: 23f9bee4-ac7e-4828-82c0-576fee0d29b7
// IMPLEMENTED — the target attacking creature gets base power 0 until end
// of turn (Modifier::SetPower, layer 7b), its toughness untouched.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SINGING_TREE,
    oracle_id = "23f9bee4-ac7e-4828-82c0-576fee0d29b7",
    scryfall_id = "78b11d04-aee6-48c8-b4e2-2949879a30c8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Singing Tree",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetPower(0),
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE)),
    )],
);
