//! Equinox — {W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Counter target spell if it would destroy a land you control."
//! Set: LEG #13 — Legends | Scryfall ID: 840c6586-a7a9-4ae8-96be-a995a0693eb6 | Oracle ID: 0fa2cb01-476e-4e82-94e6-9639e53a7743
// IMPLEMENTED — enchant land; the enchanted land has "{T}: Counter target
// spell if it would destroy a land you control."

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// "{T}: Counter target spell if it would destroy a land you control." The
// ability can target any spell, even one that would destroy nothing; the
// check is made on resolution (2013-09-20 and 2004-10-04 rulings).
static EQUINOX_ABILITY: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::IfCondition {
        condition: Condition::TargetSpellWouldDestroy(&Filter::YOUR_LAND),
        then: &[Effect::CounterTargetSpell],
        otherwise: &[],
    }],
    targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
);

card!(
    index = index::EQUINOX,
    oracle_id = "0fa2cb01-476e-4e82-94e6-9639e53a7743",
    scryfall_id = "840c6586-a7a9-4ae8-96be-a995a0693eb6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Equinox",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        // "Enchanted land has ..."
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&EQUINOX_ABILITY)
        ),
    ],
);
