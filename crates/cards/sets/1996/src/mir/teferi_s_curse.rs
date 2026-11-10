//! Teferi's Curse — {1}{U} — Enchantment — Aura
//! Oracle: Enchant artifact or creature
//! Oracle: Enchanted permanent has phasing. (It phases in or out before its controller untaps during each of their untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #96 — Mirage | Scryfall ID: fb813ef5-8441-4024-a585-1ea24145e1bd | Oracle ID: 6e422d5e-d9f5-492d-b82a-31f7274d4153
// IMPLEMENTED — Aura enchanting artifact or creature and granting phasing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_OR_CREATURE: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]);
static ENCHANTED_PERMANENT: Filter =
    Filter::And(&[ARTIFACT_OR_CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::TEFERI_S_CURSE,
    oracle_id = "6e422d5e-d9f5-492d-b82a-31f7274d4153",
    scryfall_id = "fb813ef5-8441-4024-a585-1ea24145e1bd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Teferi's Curse",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&ARTIFACT_OR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ARTIFACT_OR_CREATURE))),
        ),
        static_ability!(
            ENCHANTED_PERMANENT,
            Modifier::AddKeyword(KeywordSet::PHASING)
        ),
    ],
);
