//! Spoils of War — {X}{B} — Sorcery
//! Oracle: X is the number of artifact and/or creature cards in an opponent's graveyard as you cast this spell.
//! Oracle: Distribute X +1/+1 counters among any number of target creatures.
//! Set: ICE #164 — Ice Age | Scryfall ID: b38af8bd-d927-46d0-a1b1-fb437ea9ea66 | Oracle ID: f3240a86-28b6-4338-9a59-1fb6f795d713

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPOILS_OF_WAR,
    oracle_id = "f3240a86-28b6-4338-9a59-1fb6f795d713",
    scryfall_id = "b38af8bd-d927-46d0-a1b1-fb437ea9ea66",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "puts X +1/+1 counters on up to one target creature for a freely chosen X; fixing X to the number of artifact and/or creature cards in an opponent's graveyard as cast and distributing counters across multiple target creatures are not supported",
    ),
    faces = &[face!(
        name = "Spoils of War",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        // NOT SUPPORTED: Fixing X to the number of artifact and/or creature cards in an opponent's graveyard as cast, and distributing X counters across multiple target creatures.
        spell!(
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::X,
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ],
);
