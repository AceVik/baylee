//! Anarchist — {4}{R} — Creature — Human Wizard
//! Oracle: When this creature enters, you may return target sorcery card from your graveyard to your hand.
//! Set: TPR #125 — Tempest Remastered | Scryfall ID: 966d56fd-4df8-4532-860c-371641630a70 | Oracle ID: 82b9ad93-df75-4417-9746-798db8fe6ba0
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SORCERY_CARD: Filter = Filter::HasType(TypeSet::SORCERY);

card!(
    index = index::ANARCHIST,
    oracle_id = "82b9ad93-df75-4417-9746-798db8fe6ba0",
    scryfall_id = "966d56fd-4df8-4532-860c-371641630a70",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Anarchist",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::MayDo {
            effects: &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&SORCERY_CARD, PlayerRel::You),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &SORCERY_CARD,
            PlayerRel::You,
        ))),
    )],
);
