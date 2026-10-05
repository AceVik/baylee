//! Wall of Tombstones — {1}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: At the beginning of your upkeep, this creature's base toughness becomes equal to 1 plus the number of creature cards in your graveyard. (This effect lasts indefinitely.)
//! Set: LEG #129 — Legends | Scryfall ID: 55da1e86-fe18-486a-b510-f941e6f6e378 | Oracle ID: 2412a8a2-d028-44f1-8979-add9ec953759
// PARTIAL — defender only; the base-toughness trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_TOMBSTONES,
    oracle_id = "2412a8a2-d028-44f1-8979-add9ec953759",
    scryfall_id = "55da1e86-fe18-486a-b510-f941e6f6e378",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "every base-power/toughness setter writes both halves together, and \
         the printed sentence changes base toughness alone"
    ),
    faces = &[face!(
        name = "Wall of Tombstones",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "At the beginning of your upkeep, this creature's base
// toughness becomes equal to 1 plus the number of creature cards in your
// graveyard. (This effect lasts indefinitely.)" — the trigger is
// `Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }`,
// the count is `Amount::Plus` over `Amount::CountOf` in
// `ZoneSel::GraveyardYou`, and the duration is `Duration::Indefinitely`,
// but `Effect::SetPTFilter` (like every base-P/T setter) sets base power and
// toughness together. Writing `power: Amount::Fixed(0)` would overwrite a
// base power the printed sentence leaves alone, so a later animation or
// base-P/T setter would be erased — the same gap Singing Tree and Sentinel
// report. The ability comes off the card rather than changing a
// characteristic the card does not touch.
