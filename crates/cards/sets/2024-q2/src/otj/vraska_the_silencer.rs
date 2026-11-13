//! Vraska, the Silencer — {1}{B}{G} — Legendary Creature — Gorgon Assassin
//! Oracle: Deathtouch
//! Oracle: Whenever a nontoken creature an opponent controls dies, you may pay {1}. If you do, return that card to the battlefield tapped under your control. It's a Treasure artifact with "{T}, Sacrifice this artifact: Add one mana of any color," and it loses all other card types.
//! Set: OTJ #237 — Outlaws of Thunder Junction | Scryfall ID: b042abf2-c40b-4235-a4fa-2e4901c375c3 | Oracle ID: e7c5a568-3855-4a20-9edf-01a8a6a4ba97
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VRASKA_THE_SILENCER,
    oracle_id = "e7c5a568-3855-4a20-9edf-01a8a6a4ba97",
    scryfall_id = "b042abf2-c40b-4235-a4fa-2e4901c375c3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Vraska, the Silencer",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GORGON, subtypes::creature::ASSASSIN],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    keywords = KeywordSet::DEATHTOUCH,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::And(&[
            Filter::NONTOKEN_CREATURE,
            Filter::ControlledByOpponent,
        ])),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{1}"),
            effects: &[Effect::GraveyardToBattlefieldModified {
                target: TargetSpec::EventObject,
                owner_control: false,
                tapped: true,
                counters: None,
                modifiers: &[
                    Modifier::BecomeType {
                        types: TypeSet::ARTIFACT,
                        subtype: subtypes::artifact::TREASURE,
                    },
                    Modifier::GrantActivatedDefinition(&crate::tokens::TREASURE_MANA_ABILITY),
                ],
            }],
        }]
    ),],
);
