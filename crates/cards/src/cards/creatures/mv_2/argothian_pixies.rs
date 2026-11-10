//! Argothian Pixies — {1}{G} — Creature — Faerie
//! Oracle: This creature can't be blocked by artifact creatures.
//! Oracle: Prevent all damage that would be dealt to this creature by artifact creatures.
//! Set: ME4 #142 — Masters Edition IV | Scryfall ID: 6d672009-a442-495a-b2aa-b25b92db7030 | Oracle ID: bbf183bc-d502-4432-8202-f29f60c08396
// IMPLEMENTED — can't be blocked by artifact creatures, and damage from
// artifact creatures is prevented (Modifier::PreventDamageFrom).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::ARGOTHIAN_PIXIES,
    oracle_id = "bbf183bc-d502-4432-8202-f29f60c08396",
    scryfall_id = "6d672009-a442-495a-b2aa-b25b92db7030",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argothian Pixies",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(2),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeBlockedBy(&ARTIFACT_CREATURE)),
        static_ability!(
            Filter::This,
            Modifier::PreventDamageFrom(&ARTIFACT_CREATURE)
        ),
    ],
);
