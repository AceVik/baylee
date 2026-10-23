//! Adding mana does not hide a target belonging to the same activation.

use super::super::{LineShape, ability_shape, line_shape, map};
use baylee_cards_dsl::{
    AbilityDef, Amount, Cost, Effect, Filter, ManaColor, PlayerRel, TargetReq, TargetSpec,
    activated, mana_ability,
};

#[test]
fn a_direct_target_after_adding_mana_keeps_the_activation_on_its_own_line() {
    static MANA: AbilityDef = mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Black, 4)]);
    static ABILITY: AbilityDef = activated!(
        Cost::TAP,
        &[
            Effect::mana(ManaColor::Black, 4),
            Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::TargetPlayerOrController
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    );
    let line = "{T}: Add {B}{B}{B}{B}. Target opponent gains control of this creature. (Activate only as an instant.)";
    assert_eq!(line_shape(line), LineShape::Activated);
    assert_eq!(ability_shape(&ABILITY), LineShape::Activated);
    let oracle = format!("Swampwalk\n{line}");
    assert_eq!(map(&[ABILITY], &oracle).lines, vec![Some(1)]);

    assert_eq!(
        map(&[MANA], line).lines,
        vec![None],
        "the targeted line must not be claimed by a true mana ability"
    );
}

#[test]
fn mana_without_a_direct_target_keeps_its_mana_line() {
    static ABILITY: AbilityDef = mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Black, 4)]);
    let line = "{T}: Add {B}{B}{B}{B}.";
    assert_eq!(line_shape(line), LineShape::Mana);
    assert_eq!(ability_shape(&ABILITY), LineShape::Mana);
    assert_eq!(map(&[ABILITY], line).lines, vec![Some(0)]);
}

#[test]
fn targets_in_a_delayed_trigger_do_not_target_the_mana_activation() {
    static ABILITY: AbilityDef = mana_ability!(
        Cost::TAP,
        &[
            Effect::mana(ManaColor::Red, 1),
            Effect::AtNextEndStep {
                effects: &[Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::AnyTarget
                }],
            },
        ],
    );
    let line = "{T}: Add {R}. At the beginning of the next end step, this creature deals 1 damage to any target.";
    assert_eq!(line_shape(line), LineShape::Mana);
    assert_eq!(ability_shape(&ABILITY), LineShape::Mana);
    assert_eq!(map(&[ABILITY], line).lines, vec![Some(0)]);
    assert_eq!(
        line_shape(
            "{T}: Add one mana of any color. When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose new targets for the copy."
        ),
        LineShape::Mana,
        "the delayed mana-spent rider retains its own targets"
    );
}
