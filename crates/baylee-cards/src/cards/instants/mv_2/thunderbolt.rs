//! Thunderbolt — {1}{R} — Instant
//! Oracle: Choose one —
//! Oracle: • Thunderbolt deals 3 damage to target player or planeswalker.
//! Oracle: • Thunderbolt deals 4 damage to target creature with flying.
//! Set: AVR #159 — Avacyn Restored | Scryfall ID: 5845a5bc-6b7d-4bbb-80b3-a0f877b95553 | Oracle ID: 6ffae2fa-511a-45bf-93c4-5d53c85324f5
// IMPLEMENTED — Modal instant dealing 3 damage to target player or planeswalker or 4 damage to target creature with flying.

use baylee_cards_dsl::prelude::*;

static FLYING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::THUNDERBOLT,
    oracle_id = "6ffae2fa-511a-45bf-93c4-5d53c85324f5",
    scryfall_id = "5845a5bc-6b7d-4bbb-80b3-a0f877b95553",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thunderbolt",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
                }],
                targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
                    &Filter::PLANESWALKER,
                ))),
            ),
            mode!(
                &[Effect::DealDamage {
                    amount: Amount::Fixed(4),
                    target: TargetSpec::Object(&FLYING_CREATURE),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&FLYING_CREATURE))),
            ),
        ],
    }],
);
