//! Relic Ward — {1}{W} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant artifact
//! Oracle: Enchanted artifact has shroud. (It can't be the target of spells or abilities.)
//! Set: VIS #16 — Visions | Scryfall ID: f0459667-b7da-43bd-b981-0e515432d147 | Oracle ID: c6321de2-80b0-4679-93cf-97df75344e97
// PARTIAL: has flash, enchants artifact, and grants shroud; sacrificing Relic Ward at the beginning of the next cleanup step if cast at instant speed is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
card!(
    index = index::RELIC_WARD,
    oracle_id = "c6321de2-80b0-4679-93cf-97df75344e97",
    scryfall_id = "f0459667-b7da-43bd-b981-0e515432d147",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial(
        "has flash, enchants artifact, and grants shroud; sacrificing Relic Ward at the beginning of the next cleanup step if cast at instant speed is not yet supported"
    ),
    faces = &[face!(
        name = "Relic Ward",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ARTIFACT),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
        ),
        static_ability!(
            Filter::And(&[Filter::ARTIFACT, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::SHROUD),
        ),
    ],
);
