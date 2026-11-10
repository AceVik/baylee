//! Gempalm Incinerator — {2}{R} — Creature — Goblin
//! Oracle: Cycling {1}{R} ({1}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have it deal X damage to target creature, where X is the number of Goblins on the battlefield.
//! Set: PLST #DMR-122 — The List | Scryfall ID: 0ac7667c-2526-4f4b-8886-2516d4a8fbae | Oracle ID: 467ea3e3-7767-405a-9d1a-8fa5de696376
// IMPLEMENTED — cycling, with optional damage equal to all Goblins on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GOBLIN: Filter = Filter::HasSubtype(subtypes::creature::GOBLIN);

card!(
    index = index::GEMPALM_INCINERATOR,
    oracle_id = "467ea3e3-7767-405a-9d1a-8fa5de696376",
    scryfall_id = "0ac7667c-2526-4f4b-8886-2516d4a8fbae",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gempalm Incinerator",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{1}{R}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand
        ),
        triggered!(
            Trigger::CycledThis,
            &[Effect::MayDo {
                effects: &[Effect::DealDamage {
                    amount: Amount::CountOf {
                        filter: &GOBLIN,
                        zone: ZoneSel::Battlefield,
                    },
                    target: TargetSpec::Object(&Filter::CREATURE),
                }],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ],
);
