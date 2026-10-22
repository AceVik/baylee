//! Ivory Gargoyle — {4}{W} — Creature — Gargoyle
//! Oracle: Flying
//! Oracle: When this creature dies, return it to the battlefield under its owner's control at the beginning of the next end step and you skip your next draw step.
//! Oracle: {4}{W}: Exile this creature.
//! Set: ME2 #19 — Masters Edition II | Scryfall ID: 07d79058-2a78-4237-83b0-4715053e8267 | Oracle ID: e8cc9f83-4e4c-4820-9a7a-f82f728e9767
// NOT SUPPORTED: waiting until the beginning of the next end step to return and skipping your next draw step when this creature dies are not supported; has flying, returns from the graveyard to the battlefield under its owner's control when it dies, and exiles itself for {4}{W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IVORY_GARGOYLE,
    oracle_id = "e8cc9f83-4e4c-4820-9a7a-f82f728e9767",
    scryfall_id = "07d79058-2a78-4237-83b0-4715053e8267",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "waiting until the beginning of the next end step to return and skipping your next draw step when this creature dies are not supported; has flying, returns from the graveyard to the battlefield under its owner's control when it dies, and exiles itself for {4}{W}",
    ),
    faces = &[face!(
        name = "Ivory Gargoyle",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GARGOYLE],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnSourceFromGraveyard {
                face: None,
                tapped: false,
                owner_control: true,
                counters: None,
            }],
        ),
        activated!(
            cost!("{4}{W}"),
            &[Effect::ExileSource],
        ),
    ],
);
