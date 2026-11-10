//! Goblin Flotilla — {2}{R} — Creature — Goblin
//! Oracle: Islandwalk (This creature can't be blocked as long as defending player controls an Island.)
//! Oracle: At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn.
//! Set: FEM #55 — Fallen Empires | Scryfall ID: 87024efe-4a74-49fe-a43a-480bed0a650a | Oracle ID: 014ca38f-d4ac-4417-bfa8-785a7d6fce43
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_FLOTILLA,
    oracle_id = "014ca38f-d4ac-4417-bfa8-785a7d6fce43",
    scryfall_id = "87024efe-4a74-49fe-a43a-480bed0a650a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::ISLANDWALK,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Flotilla",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{R}"),
            effect: &Effect::ModifySource {
                modifier: Modifier::GrantTriggered {
                    trigger: Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
                    effects: &[Effect::ModifyEvent {
                        modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                        duration: Duration::UntilEndOfTurn,
                    }],
                    target: None,
                },
                duration: Duration::UntilEndOfCombat,
            },
        }],
    ),],
);
