//! Hot Springs — {1}{G} — Enchantment — Aura
//! Oracle: Enchant land you control
//! Oracle: Enchanted land has "{T}: Prevent the next 1 damage that would be dealt to any target this turn."
//! Set: ICE #248 — Ice Age | Scryfall ID: 1d4fe072-81a7-424e-8d21-aaca010d5b1d | Oracle ID: 123880ad-7b54-4ab0-82eb-c78b12ab89a7
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GRANTED: AbilityDef = activated!(
    cost!(TapSelf),
    &[Effect::PreventNextDamage {
        target: TargetSpec::AnyTarget,
        amount: Amount::Fixed(1),
    }],
    target = Some(TargetSpec::AnyTarget)
);

card!(
    index = index::HOT_SPRINGS,
    oracle_id = "123880ad-7b54-4ab0-82eb-c78b12ab89a7",
    scryfall_id = "1d4fe072-81a7-424e-8d21-aaca010d5b1d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hot Springs",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND)))
        ),
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&GRANTED)
        ),
    ],
);
