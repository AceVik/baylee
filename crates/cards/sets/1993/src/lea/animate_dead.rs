//! Animate Dead — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature card in a graveyard
//! Oracle: When this Aura enters, if it's on the battlefield, it loses "enchant creature card in a graveyard" and gains "enchant creature put onto the battlefield with this Aura." Return enchanted creature card to the battlefield under your control and attach this Aura to it. When this Aura leaves the battlefield, that creature's controller sacrifices it.
//! Oracle: Enchanted creature gets -1/-0.
//! Set: SOC #207 — Secrets of Strixhaven Commander | Scryfall ID: cc0c2bab-4392-4e7b-9d14-2901f4ffbae8 | Oracle ID: c0d8fef4-65f4-4769-982d-b397d2b7e977

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANIMATE_DEAD,
    oracle_id = "c0d8fef4-65f4-4769-982d-b397d2b7e977",
    scryfall_id = "cc0c2bab-4392-4e7b-9d14-2901f4ffbae8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Animate Dead",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: GRAVEYARD_CREATURE
            }],
            targets = Some(TargetReq::one(GRAVEYARD_CREATURE))
        ),
        // "If it's on the battlefield" is read of this Aura incarnation when
        // the ability resolves; `ReanimateEnchanted` does nothing otherwise.
        triggered!(Trigger::ETB, &[Effect::ReanimateEnchanted]),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(-1, 0)
        ),
    ],
);

const GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);
