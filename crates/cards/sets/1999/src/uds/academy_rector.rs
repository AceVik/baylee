//! Academy Rector — {3}{W} — Creature — Human Cleric
//! Oracle: When this creature dies, you may exile it. If you do, search your library for an enchantment card, put that card onto the battlefield, then shuffle.
//! Set: UDS #1 — Urza's Destiny | Scryfall ID: 4367bc78-0912-4abd-8edd-bc792558d01a | Oracle ID: e3c85068-b4b6-40b9-a16c-5c3b2d059ec4
// IMPLEMENTED — a dies trigger: on a yes it exiles the Rector out of the
// graveyard and tutors an enchantment straight onto the battlefield. One
// question gates both halves, which is what the printed "if you do" says on
// every board a player can reach; the shuffle is the search's own.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ACADEMY_RECTOR,
    oracle_id = "e3c85068-b4b6-40b9-a16c-5c3b2d059ec4",
    scryfall_id = "4367bc78-0912-4abd-8edd-bc792558d01a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Academy Rector",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::ExileSource,
                Effect::SearchLibrary {
                    filter: &Filter::ENCHANTMENT,
                    finds: &[Find::BATTLEFIELD],
                    optional: false,
                },
            ],
        }]
    )],
);

// Engine-level test belongs in baylee-engine (card_tests). Three firsts meet
// on this card: the source an `ExileSource` reaches is in a graveyard rather
// than on the stack, a search puts a nonland onto the battlefield, and an
// optional clause's body itself asks a question — so a yes must suspend on
// the search and resume, while a no leaves the Rector lying in the graveyard
// with the library untouched.
