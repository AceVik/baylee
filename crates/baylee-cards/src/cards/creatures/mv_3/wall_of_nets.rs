//! Wall of Nets — {1}{W}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: At end of combat, exile all creatures blocked by this creature.
//! Oracle: When this creature leaves the battlefield, return all cards exiled with it to the battlefield under their owners' control.
//! Set: EXO #24 — Exodus | Scryfall ID: c1da8e79-365d-4a36-87c5-648085828f9f | Oracle ID: cd112962-b297-403c-bede-c8b587f379b3

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_NETS,
    oracle_id = "cd112962-b297-403c-bede-c8b587f379b3",
    scryfall_id = "c1da8e79-365d-4a36-87c5-648085828f9f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Batch-exiling blocked creatures as linked exiled cards is not supported; exiles all creatures blocked by it at end of combat via ExileAll and returns linked exiled cards when it leaves the battlefield",
    ),
    faces = &[face!(
        name = "Wall of Nets",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(7),
        keywords = KeywordSet::DEFENDER,
        abilities = &[
            triggered!(
                Trigger::StepBegin {
                    step: StepKind::CombatEnd,
                    whose: PlayerRel::EachPlayer,
                },
                &[Effect::ExileAll {
                    filter: &Filter::BlockedBySource,
                }],
            ),
            triggered!(
                Trigger::LeavesBattlefield(&Filter::This),
                &[Effect::ReturnLinkedToBattlefield],
            ),
        ],
    ),],
);
