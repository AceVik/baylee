//! Necratog — {1}{B}{B} — Creature — Atog
//! Oracle: Exile the top creature card of your graveyard: this creature gets +2/+2 until end of turn.
//! Set: WTH #76 — Weatherlight | Scryfall ID: fb19c519-c09a-44a0-8d4b-ab6c15dabdef | Oracle ID: 9f820f1e-3cd1-4a3e-8d00-194c8a80b296
// PARTIAL — Exiles any creature card from your graveyard to pay its activation cost; requiring the exiled card to be the top creature card of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NECRATOG,
    oracle_id = "9f820f1e-3cd1-4a3e-8d00-194c8a80b296",
    scryfall_id = "fb19c519-c09a-44a0-8d4b-ab6c15dabdef",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Exiles any creature card from your graveyard to pay its activation cost; requiring the exiled card to be the top creature card of your graveyard is not yet modeled."
    ),
    faces = &[face!(
        name = "Necratog",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ATOG],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(ExileFromGraveyard(&Filter::CREATURE)),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, 2),
            duration: Duration::UntilEndOfTurn
        }]
    ),],
);
