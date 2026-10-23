//! Fertile Ground — {1}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color.
//! Set: USG #252 — Urza's Saga | Scryfall ID: 091dda35-59e5-456d-8804-61513a610aed | Oracle ID: cf14d4e5-5965-45ad-97f7-26facf2884b5
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FERTILE_GROUND,
    oracle_id = "cf14d4e5-5965-45ad-97f7-26facf2884b5",
    scryfall_id = "091dda35-59e5-456d-8804-61513a610aed",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "adding one mana of any color to the enchanted land's controller's pool when that player is not the Aura's controller is not in Effect; adds one mana of any color to the Aura controller's pool when the enchanted land is tapped for mana",
    ),
    faces = &[face!(
        name = "Fertile Ground",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
        ),
        triggered!(
            Trigger::TappedForMana {
                by: PlayerRel::EachPlayer,
                filter: &Filter::AttachedToBySource,
            },
            &[Effect::mana_of_any_color()],
        ),
    ],
);
