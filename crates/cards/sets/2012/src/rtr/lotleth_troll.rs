//! Lotleth Troll — {B}{G} — Creature — Zombie Troll
//! Oracle: Trample
//! Oracle: Discard a creature card: Put a +1/+1 counter on this creature.
//! Oracle: {B}: Regenerate this creature.
//! Set: 2X2 #245 — Double Masters 2022 | Scryfall ID: cc138550-a797-4a57-91b3-626aac1b1edd | Oracle ID: 61b1d7e5-6155-4204-b110-35a890551ec8
// IMPLEMENTED — trample, the discard for a +1/+1 counter, and the {B}
// regeneration shield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOTLETH_TROLL,
    oracle_id = "61b1d7e5-6155-4204-b110-35a890551ec8",
    scryfall_id = "cc138550-a797-4a57-91b3-626aac1b1edd",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lotleth Troll",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::TROLL],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!(Discard(&Filter::CREATURE)),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        // "Regenerate **this creature**" names no target (CR 115.1), so
        // the shield is put on the source through `TargetSpec::ThisObject`
        // and hexproof, shroud and protection never enter into it.
        activated!(cost!("{B}"), &[Effect::regenerate(TargetSpec::ThisObject)]),
    ],
);
