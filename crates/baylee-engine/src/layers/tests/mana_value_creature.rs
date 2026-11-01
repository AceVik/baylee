//! "Is a creature in addition to its other types and has base power and base
//! toughness each equal to its mana value": one effect in layers 4 and 7b.
use super::*;

fn with_cost(state: &mut GameState, label: &str, types: TypeSet, cost: &str) -> ObjectId {
    let id = permanent(state, label, types, None);
    state.object_mut(id).expect("in play").base_mut().mana_cost =
        baylee_core::mana::ManaCost::parse(cost);
    id
}

fn types(state: &GameState, id: ObjectId) -> TypeSet {
    let obj = state.object(id).expect("in play");
    recompute(state, obj).characteristics.types
}

/// CR 205.1b: "in addition to its other types" keeps the enchantment type
/// and adds creature alone, never artifact, and CR 613.4b sets the base
/// values to the mana value in 7b. There is no noncreature question: a
/// printed 1/1 enchantment creature is set too, which is where this differs
/// from Animate Artifact's modifier.
#[test]
fn it_adds_only_creature_and_sets_every_affected_body_to_its_mana_value() {
    let mut state = fresh();
    let enchantment = with_cost(&mut state, "Enchantment", TypeSet::ENCHANTMENT, "{2}{W}");
    let creature = with_cost(
        &mut state,
        "Enchantment creature",
        TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        "{4}",
    );
    {
        let base = state.object_mut(creature).expect("in play").base_mut();
        base.power = Some(1);
        base.toughness = Some(1);
    }
    register(&mut state, 1, Modifier::CreatureWithManaValuePT);

    assert_eq!(
        types(&state, enchantment),
        TypeSet::ENCHANTMENT.union(TypeSet::CREATURE)
    );
    assert_eq!(body(&state, enchantment), (Some(3), Some(3)));
    assert_eq!(body(&state, creature), (Some(4), Some(4)));
}

/// CR 613.6: what layer 4 took is set in 7b even where the filter no longer
/// matches there. Here the effect's own creature type is what falsifies
/// "noncreature" between the two layers.
#[test]
fn the_objects_it_took_in_layer_four_are_the_ones_it_sets_in_seven_b() {
    let mut state = fresh();
    let enchantment = with_cost(&mut state, "Enchantment", TypeSet::ENCHANTMENT, "{5}");
    state.effects.register(fx(
        1,
        1,
        &Filter::NONCREATURE,
        Modifier::CreatureWithManaValuePT,
    ));
    let plan = LayerPlan::build(&state.effects);
    assert_eq!(plan.layer(Layer::Type), plan.layer(Layer::PtSet));
    assert_eq!(body(&state, enchantment), (Some(5), Some(5)));
}

/// CR 613.8a: an older "all creatures are artifacts" depends on the effect
/// that makes the enchantment a creature, so it applies after it and the
/// enchantment is an artifact too. In timestamp order alone it would not be.
#[test]
fn a_type_effect_that_reads_creature_waits_for_it() {
    let mut state = fresh();
    let enchantment = with_cost(&mut state, "Enchantment", TypeSet::ENCHANTMENT, "{1}");
    state.effects.register(fx(
        1,
        1,
        &Filter::CREATURE,
        Modifier::AddType(TypeSet::ARTIFACT),
    ));
    state
        .effects
        .register(fx(2, 2, &ANY_F, Modifier::CreatureWithManaValuePT));
    assert_eq!(
        types(&state, enchantment),
        TypeSet::ENCHANTMENT
            .union(TypeSet::CREATURE)
            .union(TypeSet::ARTIFACT)
    );
}

/// Its 7b part competes with other settings by its own timestamp (CR 613.7),
/// and what 7c adds stays on top.
#[test]
fn its_base_values_compete_in_seven_b_by_timestamp() {
    for (own, set, expected) in [(1, 2, (2, 4)), (2, 1, (6, 6))] {
        let mut state = fresh();
        let enchantment = with_cost(&mut state, "Enchantment", TypeSet::ENCHANTMENT, "{5}");
        register(&mut state, own, Modifier::CreatureWithManaValuePT);
        register(&mut state, set, Modifier::SetPT(1, 3));
        register(&mut state, 0, Modifier::ModifyPT(1, 1));
        assert_eq!(
            body(&state, enchantment),
            (Some(expected.0), Some(expected.1))
        );
    }
}
