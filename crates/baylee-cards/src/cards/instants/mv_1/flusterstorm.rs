//! Flusterstorm — {U} — Instant
//! Oracle: Counter target instant or sorcery spell unless its controller pays {1}.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: IMA #55 — Iconic Masters | Scryfall ID: f900eeb7-7c45-44bc-ad3a-0bbe594ecf50 | Oracle ID: 86bf58f2-7f25-4e10-b797-25e0e8e67769
// PARTIAL — a one-mana blue instant that points at an instant or a sorcery on
// the stack and offers its controller a {1} tax; they decline and it is
// countered, they pay and it stays. The printed storm never copies it.
// NOT SUPPORTED: `Storm` — "copy it for each spell cast before it this turn".
// Three separate pieces are missing and no combination of what exists says it.
// Storm is not on `keyword_tests::ENFORCED`, so it can be no `KeywordSet` bit.
// No `Amount` reads how many spells were cast this turn — `Amount::CountOf`
// counts objects in a `ZoneSel` and none of them is the stack, and
// `Trigger::NthSpellCast` reads that count only as a threshold to fire on, not
// as a number an effect may use. And no `Effect` copies the *source* spell:
// `Effect::CopyTargetSpell` copies the first target, once, and this spell's
// target is the one it is countering. It plays as though the line were not
// printed — a {U} instant that taxes one spell, once.

use baylee_cards_dsl::prelude::*;

/// What happens when the tax goes unpaid. A named `static` because
/// `Effect::PlayerMayPayOr` holds its fallback branch by reference, which
/// is how ward spells the same sentence in `baylee-engine`'s `trigger.rs`.
static COUNTER_IT: Effect = Effect::CounterTargetSpell;

card!(
    index = index::FLUSTERSTORM,
    oracle_id = "86bf58f2-7f25-4e10-b797-25e0e8e67769",
    scryfall_id = "f900eeb7-7c45-44bc-ad3a-0bbe594ecf50",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Flusterstorm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial("storm is not written: the spell resolves once"),
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &COUNTER_IT,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY
        )))
    )],
);
