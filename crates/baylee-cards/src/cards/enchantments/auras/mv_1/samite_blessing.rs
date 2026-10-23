//! Samite Blessing — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: The next time a source of your choice would deal damage to target creature this turn, prevent that damage."
//! Set: STH #13 — Stronghold | Scryfall ID: 3b21aa2e-df36-4101-89b6-515858f2ab88 | Oracle ID: a524ae69-2594-467d-af0a-27b9984d4300
// PARTIAL: choosing a damage source to prevent all damage from that source to target creature this turn is not supported; enchants a creature and grants "{T}: Prevent the next 1 damage that would be dealt to target creature this turn."

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLESSING_ABILITY: AbilityDef = activated!(
    cost!(TapSelf),
    &[Effect::PreventNextDamage {
        target: TargetSpec::Object(&Filter::CREATURE),
        amount: Amount::Fixed(1),
    }],
    target = Some(TargetSpec::Object(&Filter::CREATURE)),
);

card!(
    index = index::SAMITE_BLESSING,
    oracle_id = "a524ae69-2594-467d-af0a-27b9984d4300",
    scryfall_id = "3b21aa2e-df36-4101-89b6-515858f2ab88",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "choosing a damage source to prevent all damage from that source to target creature this turn is not supported; enchants a creature and grants \"{T}: Prevent the next 1 damage that would be dealt to target creature this turn.\"",
    ),
    faces = &[face!(
        name = "Samite Blessing",
        mana_cost = mana!("{W}"),
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
            Modifier::GrantActivatedDefinition(&BLESSING_ABILITY),
        ),
    ],
);
