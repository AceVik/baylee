//! Wind Shear — {2}{G} — Instant
//! Oracle: Attacking creatures with flying get -2/-2 and lose flying until end of turn.
//! Set: VIS #125 — Visions | Scryfall ID: b8324f44-c7f5-41ee-bc8d-16822bd8942f | Oracle ID: fa683059-b9de-4063-ae71-597ed4e2df30
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ATTACKING_WITH_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::HasKeyword(KeywordSet::FLYING),
]);

card!(
    index = index::WIND_SHEAR,
    oracle_id = "fa683059-b9de-4063-ae71-597ed4e2df30",
    scryfall_id = "b8324f44-c7f5-41ee-bc8d-16822bd8942f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wind Shear",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::continuous(
            &ATTACKING_WITH_FLYING,
            Modifier::ModifyPT(-2, -2),
            Duration::UntilEndOfTurn,
        ),
        Effect::continuous(
            &ATTACKING_WITH_FLYING,
            Modifier::RemoveKeyword(KeywordSet::FLYING),
            Duration::UntilEndOfTurn,
        ),
    ])],
);
