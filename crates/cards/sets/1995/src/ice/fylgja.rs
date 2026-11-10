//! Fylgja — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: This Aura enters with four healing counters on it.
//! Oracle: Remove a healing counter from this Aura: Prevent the next 1 damage that would be dealt to enchanted creature this turn.
//! Oracle: {2}{W}: Put a healing counter on this Aura.
//! Set: ICE #26 — Ice Age | Scryfall ID: 3c6358a1-37f0-4b40-93d4-4f1652c38404 | Oracle ID: df0e5dd8-65e0-474c-ac32-6e0f6f16b8e6
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FYLGJA,
    oracle_id = "df0e5dd8-65e0-474c-ac32-6e0f6f16b8e6",
    scryfall_id = "3c6358a1-37f0-4b40-93d4-4f1652c38404",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fylgja",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::HEALING,
            amount: Amount::Fixed(4),
        }],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            cost!(RemoveCounterSelf {
                kind: counters::HEALING,
                n: 1
            }),
            &[Effect::PreventNextDamageToAttached {
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!("{2}{W}"),
            &[Effect::AddCounter {
                kind: counters::HEALING,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
