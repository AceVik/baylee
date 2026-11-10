//! Prismatic Ward — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a color.
//! Oracle: Prevent all damage that would be dealt to enchanted creature by sources of the chosen color.
//! Set: 5ED #53 — Fifth Edition | Scryfall ID: 24b8ce55-2fb1-4330-9db4-146a120aed81 | Oracle ID: ad5e2cb0-00d7-4dad-b20e-23bd41ada398
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRISMATIC_WARD,
    oracle_id = "ad5e2cb0-00d7-4dad-b20e-23bd41ada398",
    scryfall_id = "24b8ce55-2fb1-4330-9db4-146a120aed81",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Prismatic Ward",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::ChooseColor],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::PreventDamageFrom(&Filter::OfSourceChosenColor)
        ),
    ],
);
