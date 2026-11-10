//! Hecatomb — {1}{B}{B} — Enchantment
//! Oracle: When this enchantment enters, sacrifice this enchantment unless you sacrifice four creatures.
//! Oracle: Tap an untapped Swamp you control: This enchantment deals 1 damage to any target.
//! Set: ME1 #71 — Masters Edition | Scryfall ID: d3173377-9a9c-4dd9-a638-89be0cdccaba | Oracle ID: 0ec153ee-2a91-4196-9105-212dd1864aea
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_SWAMP: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::ControlledByYou,
]);

card!(
    index = index::HECATOMB,
    oracle_id = "0ec153ee-2a91-4196-9105-212dd1864aea",
    scryfall_id = "d3173377-9a9c-4dd9-a638-89be0cdccaba",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hecatomb",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PlayerMaySacrificeNOr {
                player: PlayerRel::You,
                filter: &Filter::CREATURE,
                count: 4,
                effect: &Effect::SacrificeSelf,
            }]
        ),
        activated!(
            cost!(TapOther(&YOUR_SWAMP)),
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::AnyTarget)
        ),
    ],
);
