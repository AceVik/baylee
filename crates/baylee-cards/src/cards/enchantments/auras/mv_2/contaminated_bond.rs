//! Contaminated Bond — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: Whenever enchanted creature attacks or blocks, its controller loses 3 life.
//! Set: 10E #132 — Tenth Edition | Scryfall ID: a56b6552-367e-4b0b-9095-42322843b1b2 | Oracle ID: 56aeaee8-e014-485e-b7ad-e26ade663bcf
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACHED: Filter = Filter::AttachedToBySource;

card!(
    index = index::CONTAMINATED_BOND,
    oracle_id = "56aeaee8-e014-485e-b7ad-e26ade663bcf",
    scryfall_id = "a56b6552-367e-4b0b-9095-42322843b1b2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Contaminated Bond",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::Attacks(&ATTACHED),
            &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::ControllerOfAttached,
            }],
        ),
        triggered!(
            Trigger::Blocks(&ATTACHED),
            &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::ControllerOfAttached,
            }],
        ),
    ],
);
