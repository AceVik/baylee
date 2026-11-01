//! Deep Spawn — {5}{U}{U}{U} — Creature — Homarid
//! Oracle: Trample
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you mill two cards.
//! Oracle: {U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap this creature. (A creature with shroud can't be the target of spells or abilities.)
//! Set: ME2 #45 — Masters Edition II | Scryfall ID: e70373b7-ced5-4d88-9ff3-04b6f59c69c9 | Oracle ID: d1aba343-d85a-4829-bdfe-db31fcb3c320
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEEP_SPAWN,
    oracle_id = "d1aba343-d85a-4829-bdfe-db31fcb3c320",
    scryfall_id = "e70373b7-ced5-4d88-9ff3-04b6f59c69c9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deep Spawn",
        mana_cost = mana!("{5}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HOMARID],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayMillOr {
                player: PlayerRel::You,
                count: 2,
                effect: &Effect::SacrificeSelf,
            }],
        ),
        activated!(
            cost!("{U}"),
            &[
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::SHROUD),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::ModifySource {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::UntilYourNextUntapStep,
                },
                Effect::TapSelf,
            ],
        ),
    ],
);
