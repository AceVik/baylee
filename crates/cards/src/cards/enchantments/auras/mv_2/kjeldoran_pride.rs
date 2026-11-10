//! Kjeldoran Pride — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+2.
//! Oracle: {2}{U}: Attach this Aura to target creature other than enchanted creature.
//! Set: ALL #9a — Alliances | Scryfall ID: d0acdf4d-6fdd-4430-9204-9c80dc8fb387 | Oracle ID: 515fb257-ec07-4978-af16-47a10a22ba3c
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::AttachedToBySource)]);

card!(
    index = index::KJELDORAN_PRIDE,
    oracle_id = "515fb257-ec07-4978-af16-47a10a22ba3c",
    scryfall_id = "d0acdf4d-6fdd-4430-9204-9c80dc8fb387",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Pride",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(1, 2)
        ),
        activated!(
            cost!("{2}{U}"),
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&OTHER_CREATURE),
            }],
            target = Some(TargetSpec::Object(&OTHER_CREATURE)),
        ),
    ],
);
