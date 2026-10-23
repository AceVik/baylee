//! Somnophore — {2}{U}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage to a player, tap target creature that player controls. That creature doesn't untap during its controller's untap step for as long as this creature remains on the battlefield.
//! Set: USG #97 — Urza's Saga | Scryfall ID: c35aa7a8-5579-46b7-83ef-f5ecc2b31847 | Oracle ID: 86541bba-7db8-4893-addf-8a4a42168ce4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOMNOPHORE,
    oracle_id = "86541bba-7db8-4893-addf-8a4a42168ce4",
    scryfall_id = "c35aa7a8-5579-46b7-83ef-f5ecc2b31847",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Somnophore",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[
            Effect::TapTarget,
            Effect::ModifyTargets {
                modifier: Modifier::DoesNotUntap,
                duration: Duration::WhileSourceOnBattlefield,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::ObjectOfEventPlayer(
            &Filter::CREATURE,
        ))),
    ),],
);
