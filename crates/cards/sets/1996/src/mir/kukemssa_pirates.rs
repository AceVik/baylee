//! Kukemssa Pirates — {3}{U} — Creature — Human Pirate
//! Oracle: Whenever this creature attacks and isn't blocked, you may gain control of target artifact defending player controls. If you do, this creature assigns no combat damage this turn.
//! Set: MIR #71 — Mirage | Scryfall ID: d10b5ce7-16c9-48f3-a1da-8a91092d053a | Oracle ID: 280148c5-3a2f-48f7-a147-4578f2068073
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDERS_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::KUKEMSSA_PIRATES,
    oracle_id = "280148c5-3a2f-48f7-a147-4578f2068073",
    scryfall_id = "d10b5ce7-16c9-48f3-a1da-8a91092d053a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kukemssa Pirates",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::PIRATE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::ModifyTargets {
                    modifier: Modifier::GainControl,
                    duration: Duration::Indefinitely,
                },
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_ARTIFACT))),
    )],
);
