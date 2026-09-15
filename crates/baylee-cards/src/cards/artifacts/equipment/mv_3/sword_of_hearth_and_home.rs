//! Sword of Hearth and Home — {3} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2 and has protection from green and from white.
//! Oracle: Whenever equipped creature deals combat damage to a player, exile up to one target creature you own, then search your library for a basic land card. Put both cards onto the battlefield under your control, then shuffle.
//! Oracle: Equip {2}
//! Set: MH2 #238 — Modern Horizons 2 | Scryfall ID: a16fabbe-4557-4067-b882-f2e5dbd8b458 | Oracle ID: 913e6182-706a-4872-8c8a-e146b0ae0738
// IMPLEMENTED — +2/+2, protection from green/white, the blink+ramp
// damage trigger, and equip.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

/// Named because the trigger says it twice: what is exiled, and what may be
/// chosen as the target. Every other filter on this card is written where it
/// is read.
static CREATURE_YOU_OWN: Filter = f!(owned CREATURE);

card!(
    index = index::SWORD_OF_HEARTH_AND_HOME,
    oracle_id = "913e6182-706a-4872-8c8a-e146b0ae0738",
    scryfall_id = "a16fabbe-4557-4067-b882-f2e5dbd8b458",
    faces = &[face!(
        name = "Sword of Hearth and Home",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[artifact::EQUIPMENT],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Green])))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::White])))
        ),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[
                Effect::blink(TargetSpec::Object(&CREATURE_YOU_OWN)),
                Effect::SearchLibrary {
                    filter: &Filter::BASIC_LAND,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
            ],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&CREATURE_YOU_OWN)))
        ),
        equip!("{2}"),
    ],
);
