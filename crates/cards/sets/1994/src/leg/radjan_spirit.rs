//! Radjan Spirit — {3}{G} — Creature — Spirit
//! Oracle: {T}: Target creature loses flying until end of turn.
//! Set: ME4 #162 — Masters Edition IV | Scryfall ID: 03701cfd-4343-4ba0-a0b9-32db69acde6c | Oracle ID: c9ffce6c-a113-4c9d-9148-5ace68f68793
// IMPLEMENTED — {T}: target creature loses flying until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RADJAN_SPIRIT,
    oracle_id = "c9ffce6c-a113-4c9d-9148-5ace68f68793",
    scryfall_id = "03701cfd-4343-4ba0-a0b9-32db69acde6c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Radjan Spirit",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::continuous(
            &Filter::This,
            Modifier::RemoveKeyword(KeywordSet::FLYING),
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
