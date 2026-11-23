use super::*;
use baylee_cards_dsl::{Amount, Effect, Filter, TargetSpec};
use baylee_core::generated::subtypes::artifact;
use baylee_engine::choice::ChoicePrompt;
use baylee_engine::engine::DecisionContext;

#[test]
fn attach_chosen_ai_attaches_equipment_to_own_creature_and_declines_for_hostile_target() {
    static ATTACH_EQ: &[Effect] = &[Effect::AttachChosen {
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::Object(&Filter::CREATURE),
        max: Amount::Fixed(1),
        optional: true,
    }];

    let me = PlayerId::new(0);
    let them = PlayerId::new(1);

    let mut eq = permanent(obj(10), me, 0);
    eq.types = TypeSet::ARTIFACT;
    eq.subtypes = SubtypeSet::from_slice(&[artifact::EQUIPMENT]);
    let my_creature = permanent(obj(1), me, 3);
    let opp_creature = permanent(obj(2), them, 3);

    // Friendly target on stack -> AI attaches the Equipment!
    let mut v_friendly = view(
        0,
        &[20, 20],
        vec![eq.clone(), my_creature.clone(), opp_creature.clone()],
    );
    let mut stack_friendly = permanent(obj(90), me, 0);
    stack_friendly.targets = vec![baylee_core::ids::TargetRef::Object(
        baylee_core::ids::DamageSourceRef {
            object: obj(1),
            version: 0,
        },
    )];
    v_friendly.stack = vec![stack_friendly];

    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let pending = Pending::ChooseCards {
        player: me,
        options: vec![obj(10)],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    };
    let ctx = DecisionContext {
        effects: ATTACH_EQ,
        source: Some(obj(90)),
        ..Default::default()
    };
    assert_eq!(
        agent.act_with_context(&v_friendly, &pending, &ctx),
        PlayerAction::ChooseObjects {
            objects: vec![obj(10)],
        },
        "AI should attach beneficial Equipment to its own creature"
    );

    // Hostile target on stack -> AI declines optional Equipment attach!
    let mut v_hostile = view(0, &[20, 20], vec![eq, my_creature, opp_creature]);
    let mut stack_hostile = permanent(obj(91), me, 0);
    stack_hostile.targets = vec![baylee_core::ids::TargetRef::Object(
        baylee_core::ids::DamageSourceRef {
            object: obj(2),
            version: 0,
        },
    )];
    v_hostile.stack = vec![stack_hostile];
    let ctx_hostile = DecisionContext {
        effects: ATTACH_EQ,
        source: Some(obj(91)),
        ..Default::default()
    };
    assert_eq!(
        agent.act_with_context(&v_hostile, &pending, &ctx_hostile),
        PlayerAction::ChooseObjects { objects: vec![] },
        "AI should decline optionally attaching its Equipment to an opponent's creature"
    );
}
