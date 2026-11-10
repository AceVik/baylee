//! Urza, Lord High Artificer — {2}{U}{U} — Legendary Creature — Human Artificer
//! Oracle: When Urza enters, create a 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control."
//! Oracle: Tap an untapped artifact you control: Add {U}.
//! Oracle: {5}: Shuffle your library, then exile the top card. Until end of turn, you may play that card without paying its mana cost.
//! Set: CMM #130 — Commander Masters | Scryfall ID: 7b7a348a-51f7-4dc5-8fe7-1c70fea5e050 | Oracle ID: e87906d2-db1a-4e19-b910-adb4eb339945

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

use crate::tokens::CONSTRUCT_ARTIFACT_0_0 as CONSTRUCT;

card!(
    index = index::URZA_LORD_HIGH_ARTIFICER,
    oracle_id = "e87906d2-db1a-4e19-b910-adb4eb339945",
    scryfall_id = "7b7a348a-51f7-4dc5-8fe7-1c70fea5e050",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urza, Lord High Artificer",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokenPtPerCount {
                token: &CONSTRUCT,
                filter: &Filter::ARTIFACT,
                p: 1,
                t: 1,
            }]
        ),
        mana_ability!(
            cost!(TapOther(&Filter::YOUR_ARTIFACT)),
            &[Effect::mana(ManaColor::Blue, 1)]
        ),
        activated!(
            cost!("{5}"),
            &[
                Effect::ShuffleLibrary {
                    who: PlayerRel::You
                },
                Effect::ExileTopToPlay {
                    who: PlayerRel::You,
                    play: ExilePlay {
                        count: 1,
                        window: PlayWindow::ThisTurn,
                        free: true,
                        mana_any_type: false,
                        spell_types: None,
                    }
                },
            ]
        ),
    ],
);
