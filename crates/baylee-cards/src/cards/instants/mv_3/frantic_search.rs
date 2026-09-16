//! Frantic Search — {2}{U} — Instant
//! Oracle: Draw two cards, then discard two cards. Untap up to three lands.
//! Set: TLE #159 — Avatar: The Last Airbender Eternal | Scryfall ID: d8c5e52d-57ae-464b-8860-7dd39ebfef64 | Oracle ID: 16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c
// PARTIAL — an instant-speed loot that refunds its own mana: draw two, then
// discard two, then untap up to three lands.
// NOT SUPPORTED: "Untap up to three lands" is written as up to three
// *targets*. The card prints no "target", so the lands are named as the
// spell is cast (CR 601.2c) rather than picked as it resolves, and a land
// with shroud is unreachable. It is the same approximation Nesting Dovehawk
// makes for populate, and for the same reason: a target is the only thing
// the DSL has to say "up to three" with.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FRANTIC_SEARCH,
    oracle_id = "16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c",
    scryfall_id = "d8c5e52d-57ae-464b-8860-7dd39ebfef64",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Frantic Search",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage =
        Coverage::Partial("the lands are written as targets, so a shrouded land is unreachable"),
    abilities = &[spell!(
        &[
            Effect::draw(2),
            Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 2,
            },
            Effect::UntapTarget,
        ],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::LAND), 3))
    )],
);

// Behaviour belongs in `baylee-engine`'s `engine::card_tests`: this is the
// first spell whose effect list suspends mid-list on a `DiscardForPlayers`
// choice and then resumes (`Resolution::pc`) into a later effect, so what
// wants playing is that the three lands are still untapped after the discard
// has been answered.
