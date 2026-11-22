use super::*;
use crate::worth::{Aim, Origin};
use baylee_cards_dsl::{
    Amount, Effect, Filter, NumberChoiceKind, PlayerRel, TargetSpec, VariablePayKind,
};
use baylee_core::generated::subtypes::{artifact, enchantment};
use baylee_engine::choice::{ChoicePrompt, NumberPrompt};
use baylee_engine::engine::DecisionContext;

#[test]
fn choose_number_ex_sizes_lethal_energy_and_symmetric_sacrifice_count() {
    static DISCHARGE_CONT: &[Effect] = &[Effect::ChooseNumberEx {
        chooser: PlayerRel::You,
        min: Amount::Fixed(0),
        max: Amount::Fixed(u32::MAX),
        prompt: NumberChoiceKind::EnergyPayment,
        pay_cost: Some(VariablePayKind::Energy),
        then: &[Effect::DealDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::ResolutionChosenNumber,
        }],
    }];
    static INVITATION_CONT: &[Effect] = &[Effect::ChooseNumberEx {
        chooser: PlayerRel::You,
        min: Amount::Fixed(0),
        max: Amount::Fixed(13),
        prompt: NumberChoiceKind::Choice,
        pay_cost: None,
        then: &[Effect::SacrificeAmountForPlayers {
            who: PlayerRel::EachPlayer,
            filter: &Filter::CREATURE,
            amount: Amount::ResolutionChosenNumber,
        }],
    }];

    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let my_dork = permanent(obj(1), me, 1);
    let enemy_three = permanent(obj(2), them, 3);
    let enemy_four = permanent(obj(3), them, 4);

    let mut v = view(0, &[20, 20], vec![my_dork, enemy_three, enemy_four]);
    let mut stack_spell = permanent(obj(90), me, 0);
    stack_spell.targets = vec![baylee_core::ids::TargetRef::Object(
        baylee_core::ids::DamageSourceRef {
            object: obj(2),
            version: 0,
        },
    )];
    v.stack = vec![stack_spell];

    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let energy_pending = Pending::ChooseNumber {
        player: me,
        min: 0,
        max: 5,
        reason: NumberPrompt::VariableEnergyPayment,
    };
    assert_eq!(
        agent.act_with_context(
            &v,
            &energy_pending,
            &DecisionContext {
                effects: DISCHARGE_CONT,
                ..Default::default()
            }
        ),
        PlayerAction::ChooseNumber(3),
        "pays 3 {{E}} for lethal on a 3/3 instead of burning all 5 {{E}}"
    );

    // By Invitation Only: opponent controls 2 creatures while we control 1 -> chooses 2!
    let choice_pending = Pending::ChooseNumber {
        player: me,
        min: 0,
        max: 13,
        reason: NumberPrompt::Choice,
    };
    assert_eq!(
        agent.act_with_context(
            &v,
            &choice_pending,
            &DecisionContext {
                effects: INVITATION_CONT,
                ..Default::default()
            }
        ),
        PlayerAction::ChooseNumber(2)
    );
}

#[test]
fn move_attachment_answers_helpful_vs_harmful_auras_and_declines_bad_optional_host() {
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);

    let mut holy_strength = carded(
        permanent(obj(10), me, 0),
        "Holy Strength",
        TypeSet::ENCHANTMENT,
    );
    holy_strength.subtypes.insert(enchantment::AURA);
    let mut pacifism = carded(permanent(obj(11), me, 0), "Pacifism", TypeSet::ENCHANTMENT);
    pacifism.subtypes.insert(enchantment::AURA);

    let my_small = permanent(obj(1), me, 1);
    let my_big = permanent(obj(2), me, 4);
    let enemy_big = permanent(obj(3), them, 5);
    let v = view(
        0,
        &[20, 20],
        vec![holy_strength, pacifism, my_small, my_big, enemy_big],
    );
    let agent = HeuristicAgent::new(AIProfile::EXPERT);

    // Helpful Aura (Holy Strength): picks our strongest creature over opponent's creature!
    assert_eq!(
        agent.act(
            &v,
            &Pending::ChooseCards {
                player: me,
                options: vec![obj(3), obj(1), obj(2)],
                min: 1,
                max: 1,
                prompt: ChoicePrompt::NewHost { aura: obj(10) },
                total: None,
            }
        ),
        PlayerAction::ChooseObjects {
            objects: vec![obj(2)]
        }
    );

    // Optional MoveAttachment of Helpful Aura when ONLY enemy creature is offered: declines!
    assert_eq!(
        agent.act(
            &v,
            &Pending::ChooseCards {
                player: me,
                options: vec![obj(3)],
                min: 0,
                max: 1,
                prompt: ChoicePrompt::NewHost { aura: obj(10) },
                total: None,
            }
        ),
        PlayerAction::ChooseObjects { objects: vec![] }
    );

    // Harmful Aura (Pacifism): picks opponent's strongest creature over our creatures!
    assert_eq!(
        agent.act(
            &v,
            &Pending::ChooseCards {
                player: me,
                options: vec![obj(1), obj(2), obj(3)],
                min: 1,
                max: 1,
                prompt: ChoicePrompt::NewHost { aura: obj(11) },
                total: None,
            }
        ),
        PlayerAction::ChooseObjects {
            objects: vec![obj(3)]
        }
    );
}

#[test]
fn worth_scores_choose_number_ex_energy_gain_and_equipment_move_attachment() {
    static DISCHARGE_SEQ: &[Effect] = &[
        Effect::Energy {
            who: PlayerRel::You,
            amount: Amount::Fixed(3),
        },
        Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(0),
            max: Amount::Fixed(u32::MAX),
            prompt: NumberChoiceKind::EnergyPayment,
            pay_cost: Some(VariablePayKind::Energy),
            then: &[Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::ResolutionChosenNumber,
            }],
        },
    ];
    static EQUIP_FILTER: Filter = Filter::HasSubtype(artifact::EQUIPMENT);
    static MOVE_EQUIP_SEQ: &[Effect] = &[Effect::MoveAttachment {
        attachment: TargetSpec::Object(&EQUIP_FILTER),
        recipient_filter: &Filter::YOUR_CREATURE,
        must_change_recipient: false,
        optional: true,
        then_if_moved: &[],
    }];

    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let mut shuko = carded(permanent(obj(1), me, 0), "Shuko", TypeSet::ARTIFACT);
    shuko.power = None;
    shuko.toughness = None;
    shuko.subtypes.insert(artifact::EQUIPMENT);

    let my_bear = permanent(obj(2), me, 2);
    let enemy_bear = permanent(obj(3), them, 3);
    let v = view(0, &[20, 20], vec![shuko, my_bear, enemy_bear]);
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let origin = Origin::of(&v, obj(99));

    let vs_enemy = agent
        .effects_worth(&v, origin, DISCHARGE_SEQ, Aim::Object(obj(3)), 0)
        .expect("scored on enemy 3/3 using the 3 {E} gained earlier in the sequence");
    let vs_ally = agent
        .effects_worth(&v, origin, DISCHARGE_SEQ, Aim::Object(obj(2)), 0)
        .expect("scored on friendly 2/2");
    assert!(
        vs_enemy > 200,
        "killing enemy 3/3 via gained {{E}} is strongly positive: {vs_enemy}"
    );
    assert!(
        vs_enemy > vs_ally,
        "aiming Galvanic Discharge at enemy beats aiming at ally ({vs_enemy} vs {vs_ally})"
    );

    let equip_val = agent
        .effects_worth(&v, origin, MOVE_EQUIP_SEQ, Aim::Object(obj(1)), 0)
        .expect("scored Equipment MoveAttachment");
    assert!(
        equip_val > 0,
        "attaching an unattached Equipment to our creature has positive worth: {equip_val}"
    );
}
