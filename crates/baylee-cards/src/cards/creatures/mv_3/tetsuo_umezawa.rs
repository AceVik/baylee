//! Tetsuo Umezawa — {U}{B}{R} — Legendary Creature — Human Archer
//! Oracle: Tetsuo Umezawa can't be the target of Aura spells.
//! Oracle: {U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature.
//! Set: ME3 #179 — Masters Edition III | Scryfall ID: ec22ddf8-cb5e-4ca0-8f2f-c5d48f2cbd26 | Oracle ID: ff132d04-c3a9-4949-83ba-d80bcbcd7b9b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TAPPED_OR_BLOCKING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Tapped, Filter::Blocking]),
]);

card!(
    index = index::TETSUO_UMEZAWA,
    oracle_id = "ff132d04-c3a9-4949-83ba-d80bcbcd7b9b",
    scryfall_id = "ec22ddf8-cb5e-4ca0-8f2f-c5d48f2cbd26",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Tetsuo Umezawa",
        mana_cost = mana!("{U}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARCHER],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{U}{B}{B}{R}", TapSelf),
            &[Effect::destroy(TargetSpec::Object(&TAPPED_OR_BLOCKING))],
            target = Some(TargetSpec::Object(&TAPPED_OR_BLOCKING))
        ),
        static_ability!(
            Filter::This,
            Modifier::CantBeTargetedBy(&Filter::And(&[
                Filter::InZone(ZoneRef::Stack),
                Filter::HasSubtype(subtypes::enchantment::AURA),
            ]))
        )
    ],
);
