//! Energy Bolt — {X}{R}{W} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Energy Bolt deals X damage to target player or planeswalker.
//! Oracle: • Target player gains X life.
//! Set: MIR #263 — Mirage | Scryfall ID: 711f4cff-0256-44b2-a2fe-1cae6e9edb2b | Oracle ID: 5c19db44-0eae-4d22-9bf4-7e6cdc6b9d8f
// IMPLEMENTED — modal X spell dealing X damage to target player or planeswalker, or target player gaining X life.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_BOLT,
    oracle_id = "5c19db44-0eae-4d22-9bf4-7e6cdc6b9d8f",
    scryfall_id = "711f4cff-0256-44b2-a2fe-1cae6e9edb2b",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Energy Bolt",
        mana_cost = mana!("{X}{R}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::DealDamage {
                    amount: Amount::X,
                    target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
                }],
                targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
                    &Filter::PLANESWALKER
                )))
            ),
            mode!(
                &[Effect::GainLifeFor {
                    amount: Amount::X,
                    who: PlayerRel::Chosen,
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
            ),
        ],
    }],
);
