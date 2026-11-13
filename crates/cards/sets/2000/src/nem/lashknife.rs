//! Lashknife — {1}{W} — Enchantment — Aura
//! Oracle: If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has first strike.
//! Set: NEM #9 — Nemesis | Scryfall ID: fd7451cc-4126-4518-a103-2558fa81323f | Oracle ID: 98d790c0-985f-44a4-b247-0951beea7637
// PARTIAL — Aura enchant creature granting first strike.
// NOT SUPPORTED: conditional alternative cost of tapping an untapped creature you control if you control a Plains (`cast_wizard::paid_as_an_alternative_cost` only supports `PayLife` and `ExileFromHand`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LASHKNIFE,
    oracle_id = "98d790c0-985f-44a4-b247-0951beea7637",
    scryfall_id = "fd7451cc-4126-4518-a103-2558fa81323f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "cast_wizard::paid_as_an_alternative_cost only supports PayLife and ExileFromHand, not tapping an untapped creature when controlling a Plains",
    ),
    faces = &[face!(
        name = "Lashknife",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE)
        ),
    ],
);
