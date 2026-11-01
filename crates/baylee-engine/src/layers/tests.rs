use super::*;
use baylee_cards_dsl::Duration;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::EffectId;
use baylee_core::types::TypeSet;

mod battle_characteristics;
mod characteristic_extensions;
mod indexed;

static ANY_F: Filter = Filter::Any;
static READS_TYPE: Filter = Filter::HasType(TypeSet::ARTIFACT);
static READS_KW: Filter = Filter::HasKeyword(KeywordSet::FLYING);

fn fx(id: u32, timestamp: u64, filter: &'static Filter, modifier: Modifier) -> ContinuousEffect {
    ContinuousEffect {
        id: EffectId::new(id),
        source: None,
        controller: PlayerId::new(0),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: Layer::Type,
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(filter),
        modifier,
    }
}

/// B depends on A (A adds types, B's filter reads types), C depends
/// on B (B adds a keyword, C's filter reads keywords) — while the
/// timestamps (B=1, C=2, A=3) contradict the dependency chain.
/// Only a topological order yields A, B, C; a pairwise comparator
/// sees b<c, c<a, a<b and its output is unspecified (and newer Rust
/// may panic on the broken total order).
#[test]
fn dependency_order_is_topological_not_pairwise() {
    let a = fx(1, 3, &ANY_F, Modifier::AddType(TypeSet::ARTIFACT));
    let b = fx(2, 1, &READS_TYPE, Modifier::AddKeyword(KeywordSet::FLYING));
    let c = fx(
        3,
        2,
        &READS_KW,
        Modifier::AddColor(ColorSet::from_slice(&[Color::Red])),
    );
    let all = [b, c, a];
    let mut fxs = vec![0u32, 1, 2];
    sort_by_dependency(&all, &mut fxs, Layer::Type);
    let ids: Vec<u32> = fxs.iter().map(|i| all[*i as usize].id.get()).collect();
    assert_eq!(ids, [1, 2, 3], "A, then the dependent B, then C");
}

/// Independent effects keep timestamp order, and a dependency cycle
/// falls back to timestamps (CR 613.8).
#[test]
fn timestamps_order_the_independent_and_break_cycles() {
    let x = fx(1, 10, &ANY_F, Modifier::AddType(TypeSet::ARTIFACT));
    let y = fx(2, 5, &ANY_F, Modifier::AddKeyword(KeywordSet::FLYING));
    let all = [x, y];
    let mut fxs = vec![0u32, 1];
    sort_by_dependency(&all, &mut fxs, Layer::Type);
    let ids: Vec<u32> = fxs.iter().map(|i| all[*i as usize].id.get()).collect();
    assert_eq!(ids, [2, 1], "earlier timestamp first");
}

/// A plan buckets effects per layer, and each bucket is ordered on its
/// own — the property that lets the per-object loop just filter.
#[test]
fn plan_buckets_and_orders_each_layer_once() {
    let mut table = EffectTable::default();
    let mut reg = |layer: Layer, timestamp: u64, modifier: Modifier| {
        table.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: PlayerId::new(0),
            origin: crate::effects::EffectOrigin::Resolution,
            layer,
            timestamp,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(&ANY_F),
            modifier,
        });
    };
    reg(Layer::PtModify, 5, Modifier::ModifyPT(1, 1));
    reg(Layer::Type, 9, Modifier::AddType(TypeSet::ARTIFACT));
    reg(Layer::PtModify, 2, Modifier::ModifyPT(2, 2));

    let plan = LayerPlan::build(&table);
    assert!(!plan.is_empty());
    assert_eq!(plan.layer(Layer::Type).len(), 1);
    assert_eq!(plan.layer(Layer::Color).len(), 0);
    let all = table.as_slice();
    let pt: Vec<u64> = plan
        .layer(Layer::PtModify)
        .iter()
        .map(|i| all[*i as usize].timestamp)
        .collect();
    assert_eq!(pt, [2, 5], "each bucket is timestamp-ordered on its own");
}

/// Changeling is a whole-kind mask, not a scan: every creature type is
/// set and nothing of another kind is.
///
/// The count is asked of `kind()` rather than of a range end, because ids
/// are appended now (#43) and a creature type printed tomorrow will sit
/// past every other kind's block. A mask holding one id too many would
/// make a changeling a Saga.
#[test]
fn changeling_sets_exactly_the_creature_types() {
    use baylee_core::generated::subtypes;
    use baylee_core::types::SubtypeKind;
    let all = SubtypeSet::ALL_CREATURE;
    assert!(all.contains(subtypes::creature::WIZARD));
    assert!(all.contains(subtypes::creature::ALLY));
    assert!(!all.contains(subtypes::land::FOREST));
    assert!(!all.contains(subtypes::spell::ADVENTURE));
    let creatures = (0..subtypes::COUNT)
        .map(baylee_core::ids::SubtypeId::new)
        .filter(|id| subtypes::kind(*id) == Some(SubtypeKind::Creature))
        .count();
    assert_eq!(all.len() as usize, creatures);
    assert!(
        (0..subtypes::COUNT)
            .map(baylee_core::ids::SubtypeId::new)
            .all(|id| all.contains(id) == (subtypes::kind(id) == Some(SubtypeKind::Creature))),
        "the mask and kind() are the same list read two ways"
    );
}

// --- Projection fixtures ------------------------------------------

struct RegistryLookup;
impl crate::state::CardLookup for RegistryLookup {
    fn card(
        &self,
        index: baylee_core::ids::CardIndex,
    ) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn fresh() -> GameState {
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("registry contains Forest")
        .index;
    let entry = DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: forest,
        print: baylee_core::ids::PrintRef::new(0),
    };
    let seat = || SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: SeatCapabilities::default(),
        deck: (0..60).map(|_| entry).collect(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 4,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        seats: vec![seat(), seat()],
    };
    GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
}

/// A permanent of exactly these types, with this printed body.
fn permanent(
    state: &mut GameState,
    label: &str,
    types: TypeSet,
    body: Option<(i16, i16)>,
) -> ObjectId {
    let name = state.names.intern(label);
    let id = state.create_bare(
        me(),
        crate::object::ObjectKind::Permanent,
        name,
        crate::zone::ZoneLocation::Battlefield,
    );
    let base = state.object_mut(id).expect("just created").base_mut();
    base.types = types;
    base.power = body.map(|(p, _)| p);
    base.toughness = body.map(|(_, t)| t);
    id
}

fn register(state: &mut GameState, timestamp: u64, modifier: Modifier) {
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: me(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(&ANY_F),
        modifier,
    });
}

fn body(state: &GameState, id: ObjectId) -> (Option<i16>, Option<i16>) {
    let obj = state.object(id).expect("in play");
    let c = recompute(state, obj).characteristics;
    (c.power, c.toughness)
}

fn artifact_with_cost(state: &mut GameState, cost: &str) -> ObjectId {
    let artifact = permanent(state, "Artifact", TypeSet::ARTIFACT, None);
    state
        .object_mut(artifact)
        .expect("in play")
        .base_mut()
        .mana_cost = baylee_core::mana::ManaCost::parse(cost);
    artifact
}

/// CR 613.6: the filter can stop matching after layer 4 without
/// stopping the same effect's layer-7b portion. A fresh projection
/// makes the decision afresh instead of reading the previous result.
#[test]
fn conditional_animation_continues_after_its_filter_stops_matching() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{2}{U}");
    let animation = fx(
        1,
        1,
        &Filter::NONCREATURE,
        Modifier::AnimateNoncreatureArtifact,
    );
    state.effects.register(animation);
    register(&mut state, 2, Modifier::LoseAllAbilities);

    let plan = LayerPlan::build(&state.effects);
    assert_eq!(plan.layer(Layer::Type), plan.layer(Layer::PtSet));
    for _ in 0..2 {
        state.invalidate_projections();
        state.refresh_characteristics();
        let c = state.object(artifact).expect("in play").characteristics();
        assert_eq!(c.types, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
        assert_eq!((c.power, c.toughness), (Some(3), Some(3)));
        assert!(c.abilities_lost.is_some());
    }
}

/// An independent animation applies first even with a later timestamp,
/// because it changes whether the noncreature condition holds (CR 613.8a).
#[test]
fn conditional_animation_waits_for_other_animations_and_resumes_when_they_end() {
    for (conditional_timestamp, other_timestamp) in [(1, 2), (2, 1)] {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(
            &mut state,
            conditional_timestamp,
            Modifier::AnimateNoncreatureArtifact,
        );
        register(
            &mut state,
            other_timestamp,
            Modifier::AddType(TypeSet::CREATURE),
        );
        register(&mut state, other_timestamp, Modifier::SetPT(2, 4));
        assert_eq!(body(&state, artifact), (Some(2), Some(4)));

        state.effects.remove_where(|effect| {
            matches!(effect.modifier, Modifier::AddType(_) | Modifier::SetPT(..))
        });
        assert_eq!(body(&state, artifact), (Some(5), Some(5)));
    }
}

/// Two conditional animations form a dependency loop, so the earlier
/// one starts. The skipped later one's P/T portion must stay skipped,
/// allowing a setting effect between their timestamps to win in 7b.
#[test]
fn only_the_conditional_animation_that_started_can_set_power_and_toughness() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{5}");
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
    register(&mut state, 2, Modifier::SetPT(2, 4));
    register(&mut state, 3, Modifier::AnimateNoncreatureArtifact);
    assert_eq!(body(&state, artifact), (Some(2), Some(4)));
    state.effects.remove_where(|effect| effect.timestamp == 1);
    assert_eq!(body(&state, artifact), (Some(5), Some(5)));
}

/// Setting P/T precedes modifiers and counters, but competes with
/// other settings by its own timestamp (CR 613.4b, 613.7).
#[test]
fn conditional_animation_uses_its_timestamp_in_layer_seven_b() {
    for (animation_timestamp, set_timestamp, expected) in [(1, 2, (5, 8)), (2, 1, (7, 9))] {
        let mut state = fresh();
        let artifact = artifact_with_cost(&mut state, "{5}");
        register(
            &mut state,
            animation_timestamp,
            Modifier::AnimateNoncreatureArtifact,
        );
        register(&mut state, set_timestamp, Modifier::SetPT(3, 4));
        register(&mut state, 0, Modifier::ModifyPT(1, 2));
        state.object_mut(artifact).expect("in play").counters.add(
            crate::object::CounterKind::Plus {
                power: 1,
                toughness: 2,
            },
            1,
        );
        assert_eq!(body(&state, artifact), (Some(expected.0), Some(expected.1)));
    }
}

/// Mana value is read from this projection, including copied values,
/// without requiring another projection pass through a stale cache.
#[test]
fn conditional_animation_uses_the_copy_layers_mana_value() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{3}");
    let original = artifact_with_cost(&mut state, "{7}");
    state.refresh_characteristics();
    let mut copy = fx(1, 2, &ANY_F, Modifier::BecomeCopyOf(original));
    copy.layer = Layer::Copy;
    copy.filter = EffectFilter::object(&state, artifact);
    state.effects.register(copy);
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);

    let projected = recompute(&state, state.object(artifact).expect("in play"));
    assert_eq!(projected.characteristics.mana_value(), 7);
    assert_eq!(projected.characteristics.power, Some(7));
    assert_eq!(projected.characteristics.toughness, Some(7));
    assert!(
        !projected.read_board,
        "this object's mana value needs no board read"
    );
}

/// CR 202.3b: the native back face's mana value belongs to the front,
/// even when its castable disturb cost differs. CR 202.3e makes X zero
/// off the stack.
#[test]
fn conditional_animation_uses_front_face_mana_value_and_zero_for_x() {
    let mut state = fresh();
    let transformed = artifact_with_cost(&mut state, "{2}");
    state
        .object_mut(transformed)
        .expect("in play")
        .base_mut()
        .front_mana_value = Some(6);
    let variable = artifact_with_cost(&mut state, "{X}{3}");
    let zero = artifact_with_cost(&mut state, "");
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
    assert_eq!(body(&state, transformed), (Some(6), Some(6)));
    assert_eq!(body(&state, variable), (Some(3), Some(3)));
    assert_eq!(body(&state, zero), (Some(0), Some(0)));
}

/// Removing creature from something that is not a creature changes
/// no applicability. On a printed creature it enables the animation,
/// so that animation waits for it regardless of their timestamps.
#[test]
fn conditional_animation_depends_on_removal_only_when_it_changes_creature_membership() {
    for was_creature in [false, true] {
        for (animation_timestamp, removal_timestamp) in [(1, 2), (2, 1)] {
            let mut state = fresh();
            let artifact = artifact_with_cost(&mut state, "{5}");
            if was_creature {
                let base = state.object_mut(artifact).expect("in play").base_mut();
                base.types = base.types.union(TypeSet::CREATURE);
                base.power = Some(2);
                base.toughness = Some(4);
            }
            register(
                &mut state,
                animation_timestamp,
                Modifier::AnimateNoncreatureArtifact,
            );
            register(
                &mut state,
                removal_timestamp,
                Modifier::RemoveType(TypeSet::CREATURE),
            );
            let c = recompute(&state, state.object(artifact).expect("in play")).characteristics;
            let animated = was_creature || removal_timestamp < animation_timestamp;
            assert_eq!(c.types.contains(TypeSet::CREATURE), animated);
            assert_eq!(c.power, animated.then_some(5));
            assert_eq!(c.toughness, animated.then_some(5));
        }
    }
}

/// Adding artifact to an existing artifact does not change what the
/// other animation applies to: there is no reverse dependency loop.
#[test]
fn conditional_animation_waits_for_artifact_filtered_animation_without_a_false_cycle() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{5}");
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
    state.effects.register(fx(
        1,
        2,
        &Filter::ARTIFACT,
        Modifier::AddType(TypeSet::CREATURE),
    ));
    register(&mut state, 0, Modifier::SetPT(2, 4));
    assert_eq!(body(&state, artifact), (Some(2), Some(4)));
}

/// Adding unrelated type effects cannot make a real dependency fall
/// back to timestamps. The competing animation is in a second bitset
/// word, after the 64-effect budget of the ordinary shared-plan sort.
#[test]
fn conditional_animation_keeps_dependencies_on_large_boards() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{5}");
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
    for timestamp in 2..66 {
        register(&mut state, timestamp, Modifier::AddType(TypeSet::ARTIFACT));
    }
    register(&mut state, 66, Modifier::AddType(TypeSet::CREATURE));
    register(&mut state, 0, Modifier::SetPT(2, 4));
    assert_eq!(
        LayerPlan::build(&state.effects).layer(Layer::Type).len(),
        66
    );
    assert_eq!(body(&state, artifact), (Some(2), Some(4)));
}

/// Ignore the two animations' loop before choosing among *all* ready
/// effects. The earlier animation starts, removal then takes creature
/// away, and the later animation starts too and wins the P/T timestamp.
#[test]
fn conditional_animation_loop_does_not_let_an_unrelated_ready_effect_jump_ahead() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{5}");
    register(&mut state, 1, Modifier::AnimateNoncreatureArtifact);
    register(&mut state, 2, Modifier::RemoveType(TypeSet::CREATURE));
    register(&mut state, 2, Modifier::SetPT(2, 4));
    register(&mut state, 3, Modifier::AnimateNoncreatureArtifact);
    assert_eq!(body(&state, artifact), (Some(5), Some(5)));
}

/// Named effects on different objects are independent. Animating the
/// other artifact must not postpone the older animation until after
/// the creature-removing effect on this one.
#[test]
fn conditional_animation_dependencies_respect_named_object_filters() {
    let mut state = fresh();
    let artifact = artifact_with_cost(&mut state, "{5}");
    let other = artifact_with_cost(&mut state, "{3}");
    let mut animation = fx(1, 1, &ANY_F, Modifier::AnimateNoncreatureArtifact);
    animation.filter = EffectFilter::object(&state, artifact);
    state.effects.register(animation);
    let mut removal = fx(2, 3, &ANY_F, Modifier::RemoveType(TypeSet::CREATURE));
    removal.filter = EffectFilter::object(&state, artifact);
    state.effects.register(removal);
    let mut other_animation = fx(3, 4, &ANY_F, Modifier::AddType(TypeSet::CREATURE));
    other_animation.filter = EffectFilter::object(&state, other);
    state.effects.register(other_animation);
    assert_eq!(body(&state, artifact), (None, None));
    assert!(
        recompute(&state, state.object(other).expect("in play"))
            .characteristics
            .types
            .contains(TypeSet::CREATURE)
    );
}

/// CR 205.1b: "becomes a Golem artifact creature" replaces the creature
/// types it had and keeps every other type and subtype.
#[test]
fn a_new_creature_type_replaces_the_old_ones_and_keeps_the_rest() {
    use baylee_core::generated::subtypes::{artifact, creature};
    let mut state = fresh();
    let statue = permanent(
        &mut state,
        "Statue",
        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        Some((1, 1)),
    );
    {
        let base = state.object_mut(statue).expect("in play").base_mut();
        base.subtypes.insert(creature::HUMAN);
        base.subtypes.insert(artifact::EQUIPMENT);
    }
    register(
        &mut state,
        1,
        Modifier::ReplaceCreatureTypes(creature::GOLEM),
    );
    let obj = state.object(statue).expect("in play");
    let c = recompute(&state, obj).characteristics;
    assert!(c.subtypes.contains(creature::GOLEM));
    assert!(
        !c.subtypes.contains(creature::HUMAN),
        "the old creature type goes"
    );
    assert!(
        c.subtypes.contains(artifact::EQUIPMENT),
        "an artifact type stays"
    );
    assert_eq!(c.types, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
}

/// "A creature died" asks what the permanent was on the battlefield
/// (CR 700.4): a land an effect made a creature dies as a creature,
/// though the card in the graveyard is only a land.
#[test]
fn a_land_that_was_a_creature_as_it_died_is_a_creature_that_died() {
    let mut state = fresh();
    let land = permanent(&mut state, "Animated", TypeSet::LAND, None);
    register(&mut state, 1, Modifier::AddType(TypeSet::CREATURE));
    state.refresh_characteristics();
    assert!(
        state
            .object(land)
            .expect("in play")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "the effect made it a creature"
    );
    let before = state.per_turn.creatures_died;
    state
        .move_object(
            land,
            crate::zone::ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("it moves");
    assert_eq!(state.per_turn.creatures_died, before + 1);
}

/// CR 613.4: within layer 7 the sublayers run in order, and the order is
/// the answer — 7b sets, 7c modifies (effects and counters both), 7d
/// switches. The timestamps here **contradict** it: the anthem is older
/// than the setting effect, so an implementation that ordered layer 7 by
/// timestamp alone would add first and then throw the sum away.
///
/// Every stage is asymmetric, so each one is visible in the answer
/// rather than hidden by a square body.
#[test]
fn the_sublayers_of_layer_seven_run_in_the_order_the_rules_give_them() {
    let mut state = fresh();
    let creature = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((1, 1)));

    register(&mut state, 1, Modifier::ModifyPT(1, 0));
    assert_eq!(body(&state, creature), (Some(2), Some(1)), "7c alone");

    register(&mut state, 5, Modifier::SetPT(4, 2));
    assert_eq!(
        body(&state, creature),
        (Some(5), Some(2)),
        "7b first although it is the later effect, then 7c on top of it"
    );

    {
        let counters = &mut state.object_mut(creature).expect("in play").counters;
        counters.add(
            crate::object::CounterKind::Plus {
                power: 1,
                toughness: 1,
            },
            1,
        );
        // Asymmetric on purpose. A square change would commute with the
        // switch below, and then this test would pass against a
        // projection that applied the counters *after* it.
        counters.add(
            crate::object::CounterKind::Minus {
                power: 0,
                toughness: 1,
            },
            1,
        );
    }
    assert_eq!(
        body(&state, creature),
        (Some(6), Some(2)),
        "and a counter modifies in the same sublayer, after the effects"
    );

    register(&mut state, 9, Modifier::SwitchPT);
    assert_eq!(
        body(&state, creature),
        (Some(2), Some(6)),
        "the switch reads what every earlier sublayer left (CR 613.4d)"
    );
}

/// "The number of creatures named Plague Rats on the battlefield": every
/// controller's, where "you control" counts one side of the table.
#[test]
fn a_count_on_the_battlefield_is_everybodys() {
    static RATS: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::Named("Plague Rats");
    let mut state = fresh();
    let mine = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
    let _second = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
    let theirs = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
    state.object_mut(theirs).expect("in play").controller = PlayerId::new(1);
    let _bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));

    register(
        &mut state,
        1,
        Modifier::SetPTToCount(baylee_cards_dsl::PtCount::OnBattlefield(&RATS)),
    );
    assert_eq!(body(&state, mine), (Some(3), Some(3)));
    assert_eq!(body(&state, theirs), (Some(3), Some(3)));

    let mut state = fresh();
    let mine = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
    let theirs = permanent(&mut state, "Plague Rats", TypeSet::CREATURE, Some((0, 0)));
    state.object_mut(theirs).expect("in play").controller = PlayerId::new(1);
    register(
        &mut state,
        1,
        Modifier::SetPTToCount(baylee_cards_dsl::PtCount::YouControl(&RATS)),
    );
    assert_eq!(body(&state, mine), (Some(1), Some(1)), "the control");
}

/// A setting effect asks whether the permanent is a creature **now**,
/// after layer 4, and not whether it was printed with a P/T box.
///
/// This is the Treetop Village shape. Guarding on the printed value left
/// an animated land a creature with no toughness, which the state-based
/// actions put in the graveyard the instant its own ability resolved
/// (CR 704.5f) — a card that killed itself by working.
#[test]
fn a_land_that_layer_four_animated_can_be_given_a_body() {
    let mut state = fresh();
    let land = permanent(&mut state, "Treetop Village", TypeSet::LAND, None);

    register(&mut state, 1, Modifier::SetPT(3, 3));
    assert_eq!(
        body(&state, land),
        (None, None),
        "nothing that is not a creature has a P/T to set (CR 208.3)"
    );

    register(&mut state, 2, Modifier::AddType(TypeSet::CREATURE));
    assert_eq!(
        body(&state, land),
        (Some(3), Some(3)),
        "layer 4 runs first, so layer 7b finds a creature"
    );
}

/// Counters are read off the permanent rather than off the effect table,
/// so they reach a creature that no effect matches at all — and they
/// reach nothing that is not a creature, because a P/T is a creature's
/// (CR 208.3). Each counter kind carries its own arithmetic (CR 122.1a),
/// which is what lets a −0/−1 and a +1/+1 sit on one permanent without
/// either being a special case.
#[test]
fn a_counter_changes_a_body_only_where_there_is_one() {
    use crate::object::CounterKind;
    let mut state = fresh();
    let creature = permanent(&mut state, "Wall of Roots", TypeSet::CREATURE, Some((0, 5)));
    let rock = permanent(&mut state, "Sol Ring", TypeSet::ARTIFACT, None);

    for id in [creature, rock] {
        let counters = &mut state.object_mut(id).expect("in play").counters;
        counters.add(
            CounterKind::Plus {
                power: 2,
                toughness: 2,
            },
            1,
        );
        counters.add(
            CounterKind::Minus {
                power: 0,
                toughness: 1,
            },
            3,
        );
    }

    assert_eq!(
        body(&state, creature),
        (Some(2), Some(4)),
        "+2/+2 and three −0/−1: power and toughness are summed apart"
    );
    assert_eq!(
        body(&state, rock),
        (None, None),
        "an artifact with a +1/+1 counter on it is still not a creature"
    );
}

/// A projection is skipped entirely when nothing could change the
/// answer, which is what keeps the pass off 2716 objects in an ordinary
/// game. Counters and changeling are the two reasons that have nothing
/// to do with the effect table, and both were found the hard way: a
/// permanent wearing a counter needs the pass even on an empty board.
#[test]
fn nothing_is_projected_that_no_effect_and_no_counter_reaches() {
    use crate::object::CounterKind;
    let mut state = fresh();
    let bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));

    let plan = LayerPlan::build(&state.effects);
    assert!(plan.is_empty());
    assert!(
        !needs_projection(&state, &plan, state.object(bear).expect("in play")),
        "an empty table over a bare creature is nothing to compute"
    );

    state.object_mut(bear).expect("in play").counters.add(
        CounterKind::Plus {
            power: 1,
            toughness: 1,
        },
        1,
    );
    assert!(
        needs_projection(&state, &plan, state.object(bear).expect("in play")),
        "a counter is a reason of its own — the table says nothing about it"
    );

    let mut state = fresh();
    let bear = permanent(&mut state, "Bear", TypeSet::CREATURE, Some((2, 2)));
    state.object_mut(bear).expect("in play").base_mut().keywords = KeywordSet::CHANGELING;
    assert!(
        needs_projection(
            &state,
            &LayerPlan::build(&state.effects),
            state.object(bear).expect("in play")
        ),
        "and so is changeling, which is a mask rather than an effect"
    );
}

/// CR 202.3e: "If an object on the stack has {X} in its mana cost, X is
/// the number chosen for it." No effect has to reach the spell for that:
/// a plain X spell on an empty table reads its X, and off the stack the
/// same card reads zero again.
#[test]
fn an_x_spell_no_effect_reaches_counts_its_x_on_the_stack() {
    let mut state = fresh();
    let spell = artifact_with_cost(&mut state, "{X}{1}{R}");
    let stack = crate::zone::ZoneLocation::Stack;
    state
        .move_object(
            spell,
            stack,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("onto the stack");
    state.object_mut(spell).expect("on the stack").x_value = 2;
    state.invalidate_projections();
    state.refresh_characteristics();
    assert_eq!(
        state
            .object(spell)
            .expect("on the stack")
            .characteristics()
            .mana_value(),
        4,
        "X = 2 counts on the stack with nothing else touching the spell"
    );
}

#[test]
fn text_changes_type_line_before_type_effects_and_preserves_copiable_values() {
    use crate::text_changes::TextReplacement;
    use baylee_cards_dsl::TextWordKind;
    use baylee_core::generated::subtypes::land;
    let mut state = fresh();
    let id = permanent(&mut state, "Forest", TypeSet::LAND, None);
    let original_name = state.object(id).unwrap().base.name;
    state.object_mut(id).unwrap().base_mut().subtypes = SubtypeSet::from_slice(&[land::FOREST]);
    let identity = baylee_core::ids::DamageSourceRef {
        object: id,
        version: state.object(id).unwrap().version,
    };
    assert!(state.text_changes.replace(
        identity,
        TextReplacement {
            kind: TextWordKind::BasicLandType,
            from: 4,
            to: 1
        }
    ));
    state.invalidate_projections();
    state.refresh_characteristics();
    let changed = state.object(id).unwrap().characteristics();
    assert_eq!(changed.subtypes, SubtypeSet::from_slice(&[land::ISLAND]));
    assert_eq!(changed.name, original_name);
    assert_eq!(
        crate::casting::intrinsic_mana_colors(&state, id),
        vec![baylee_core::mana::ManaColor::Blue]
    );
    assert_eq!(
        copiable_values(&state, id).unwrap().subtypes,
        SubtypeSet::from_slice(&[land::FOREST])
    );
    register(&mut state, 4, Modifier::SetLandType(land::SWAMP));
    state.refresh_characteristics();
    assert_eq!(
        state.object(id).unwrap().characteristics().subtypes,
        SubtypeSet::from_slice(&[land::SWAMP])
    );
    assert_eq!(
        crate::casting::intrinsic_mana_colors(&state, id),
        vec![baylee_core::mana::ManaColor::Black]
    );
}

#[test]
fn printed_landwalk_changes_while_later_granted_landwalk_does_not() {
    use crate::text_changes::TextReplacement;
    let mut state = fresh();
    let id = permanent(&mut state, "Walker", TypeSet::CREATURE, Some((2, 2)));
    state.object_mut(id).unwrap().base_mut().keywords = KeywordSet::FORESTWALK;
    let identity = baylee_core::ids::DamageSourceRef {
        object: id,
        version: state.object(id).unwrap().version,
    };
    assert!(state.text_changes.replace(
        identity,
        TextReplacement {
            kind: baylee_cards_dsl::TextWordKind::BasicLandType,
            from: 4,
            to: 1
        }
    ));
    register(&mut state, 4, Modifier::AddKeyword(KeywordSet::FORESTWALK));
    state.refresh_characteristics();
    let keywords = state.object(id).unwrap().characteristics().keywords;
    assert!(keywords.contains(KeywordSet::ISLANDWALK));
    assert!(keywords.contains(KeywordSet::FORESTWALK));
}

#[test]
fn text_filter_snapshot_uses_the_ability_words_after_source_changes_again() {
    use crate::text_changes::{RuleContext, TextReplacement};
    use baylee_core::color::{Color, ColorSet};
    static FILTER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::HasColor(ColorSet::of(Color::Black)),
    ]);
    let mut state = fresh();
    let source = permanent(&mut state, "Text source", TypeSet::CREATURE, Some((2, 2)));
    let target = permanent(&mut state, "Blue target", TypeSet::CREATURE, Some((2, 2)));
    state.object_mut(target).unwrap().base_mut().colors = ColorSet::of(Color::Blue);
    let identity = baylee_core::ids::DamageSourceRef {
        object: source,
        version: state.object(source).unwrap().version,
    };
    assert!(state.text_changes.replace(
        identity,
        TextReplacement {
            kind: baylee_cards_dsl::TextWordKind::Color,
            from: 2,
            to: 1
        }
    ));
    let captured = RuleContext {
        source,
        text: state.text_changes.get(identity),
        resolved_name: None,
        reference: None,
    };
    assert!(state.text_changes.replace(
        identity,
        TextReplacement {
            kind: baylee_cards_dsl::TextWordKind::Color,
            from: 1,
            to: 4
        }
    ));
    let object = state.object(target).unwrap();
    assert!(eval::matches_with_context(
        &FILTER,
        &state,
        object,
        me(),
        captured
    ));
    assert!(!eval::matches(&FILTER, &state, object, me(), source));
}
#[test]
fn text_changes_do_not_rewrite_a_dynamically_chosen_land_type() {
    use crate::text_changes::{TextOrigin, TextReplacement};
    use baylee_core::generated::subtypes::land;
    let mut state = fresh();
    let source = permanent(&mut state, "Choice source", TypeSet::ARTIFACT, None);
    let target = permanent(&mut state, "Chosen land", TypeSet::LAND, None);
    state.object_mut(source).unwrap().chosen_subtype = Some(land::FOREST);
    let identity = state.source_identity(source).unwrap();
    assert!(state.text_changes.replace(
        identity,
        TextReplacement {
            kind: baylee_cards_dsl::TextWordKind::BasicLandType,
            from: 4,
            to: 1,
        }
    ));
    let effect = state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(source),
        controller: me(),
        origin: crate::effects::EffectOrigin::Static,
        layer: Layer::Type,
        timestamp: 1,
        duration: Duration::WhileSourceOnBattlefield,
        filter: EffectFilter::object(&state, target),
        modifier: Modifier::SetLandTypeToChosen,
    });
    state
        .effect_text_overrides
        .push((effect, TextOrigin::Live(identity)));
    state.refresh_characteristics();
    assert_eq!(
        state.object(target).unwrap().characteristics().subtypes,
        SubtypeSet::from_slice(&[land::FOREST]),
        "the chosen value is not the word Forest in the ability's text"
    );
    assert_eq!(
        crate::casting::intrinsic_mana_colors(&state, target),
        vec![baylee_core::mana::ManaColor::Green]
    );
}
