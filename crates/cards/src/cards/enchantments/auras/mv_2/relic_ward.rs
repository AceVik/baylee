//! Relic Ward — {1}{W} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant artifact
//! Oracle: Enchanted artifact has shroud. (It can't be the target of spells or abilities.)
//! Set: VIS #16 — Visions | Scryfall ID: f0459667-b7da-43bd-b981-0e515432d147 | Oracle ID: c6321de2-80b0-4679-93cf-97df75344e97
// PARTIAL — an Aura that gives the enchanted artifact shroud, cast at sorcery speed only; the flash and its sacrifice rider are not supported.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The Aura casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RELIC_WARD,
    oracle_id = "c6321de2-80b0-4679-93cf-97df75344e97",
    scryfall_id = "f0459667-b7da-43bd-b981-0e515432d147",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "enchants an artifact and gives it shroud, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)"
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
