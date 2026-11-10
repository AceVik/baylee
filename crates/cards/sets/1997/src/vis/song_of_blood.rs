//! Song of Blood — {1}{R} — Sorcery
//! Oracle: Mill four cards. Whenever a creature attacks this turn, it gets +1/+0 until end of turn for each creature card put into your graveyard this way.
//! Set: VIS #94 — Visions | Scryfall ID: 4497a1d7-6604-4f2d-9484-1f1d77a6228f | Oracle ID: ab4b6524-3667-4db3-b5cb-1b02ee944906
// PARTIAL: mills four cards on resolution; the delayed attack trigger granting +1/+0 until end of turn per creature card put into your graveyard this way is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONG_OF_BLOOD,
    oracle_id = "ab4b6524-3667-4db3-b5cb-1b02ee944906",
    scryfall_id = "4497a1d7-6604-4f2d-9484-1f1d77a6228f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "mills four cards on resolution; the delayed attack trigger granting +1/+0 until end of turn per creature card put into your graveyard this way is not yet modeled",
    ),
    faces = &[face!(
        name = "Song of Blood",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::Mill {
        amount: Amount::Fixed(4),
        target: PlayerRel::You,
    }])],
);
