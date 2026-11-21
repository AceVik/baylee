//! Static cost reductions/increases (CR 118.7, CR 601.2f) and non-mana /
//! variable-X kicker + mandatory additional spell costs (CR 702.33a, CR 118.8).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use baylee_cards_dsl::{
    Amount, CostPart, CounterKind, EnterModifier, Filter, Modifier, StaticAbility, TargetSpec,
    Trigger, ZoneRef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;

const EDGEWALKER_REDUCER: u32 = 7950;
const WB_CLERIC: u32 = 7951;
const W_CLERIC_B_KICKER: u32 = 7952;
const NON_CLERIC_WB: u32 = 7953;
const SPILLOVER_REDUCER: u32 = 7954;
const GREEN_TWO_G: u32 = 7955;
const GREEN_TAXER: u32 = 7956;
const COLORED_ONLY_G_REDUCER: u32 = 7957;
const GENERIC_ONE_SPELL: u32 = 7958;
const FLUCTUATOR_REDUCER: u32 = 7959;
const CYCLER_TWO: u32 = 7960;
const DREAM_CHISEL_REDUCER: u32 = 7961;
const MORPH_BEAR: u32 = 7962;
const MISTFORM_REDUCER: u32 = 7963;
const WIZARD_SPELL: u32 = 7964;
const GOBLIN_SPELL: u32 = 7965;
const SELF_STACK_REDUCED: u32 = 7966;
const TORCH_ON_STACK: u32 = 7967;
const DECOY_INSTANT: u32 = 7968;
const PROBE_COUNTER: u32 = 7969;
const BOUNCE_KICKER_MERFOLK: u32 = 7970;
const HELPER_BEAR: u32 = 7971;
const X_KICKER_ANCIENT: u32 = 7972;
const MANDATORY_BOUNCE_SPELL: u32 = 7973;
const MANDATORY_RANDOM_DISCARD: u32 = 7974;
const MANDATORY_SPITE: u32 = 7975;
const MANDATORY_EXILE_TOP: u32 = 7976;

static NOTHING_SPELL: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[],
    targets: None,
    second_targets: None,
    condition: None,
}];

static YOUR_CLERIC: Filter = Filter::And(&[
    Filter::HasSubtype(creature::CLERIC),
    Filter::ControlledByYou,
]);
static YOUR_GREEN_SPELL: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::of(Color::Green)),
    Filter::ControlledByYou,
]);
static YOUR_FACEDOWN_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::FaceDown, Filter::ControlledByYou]);
static YOUR_SHARING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::SharesCreatureTypeWith(&Filter::This),
]);

fn cleric_creature(index: u32, name: &'static str, cost: &'static str) -> &'static CardDef {
    card(
        index,
        FaceDef {
            subtypes: &[creature::CLERIC],
            ..creature_face(name, cost, 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    )
}

#[test]
fn colored_only_cost_reduction_subtracts_colored_pips_without_reducing_generic_and_credits_kicker()
{
    static REDUCER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: YOUR_CLERIC,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLessMana {
            cost: ManaCost::parse("{W}{B}"),
            colored_only: true,
        },
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static KICKER_B: &[Cost] = &[Cost {
        mana: ManaCost::parse("{B}"),
        parts: &[],
    }];

    let reducer = card(
        EDGEWALKER_REDUCER,
        creature_face("Colored Cleric Reducer", "{1}{W}{B}", 2, 2),
        KeywordSet::EMPTY,
        REDUCER_ABILITIES,
    );
    let wb_cleric = cleric_creature(WB_CLERIC, "WB Cleric", "{1}{W}{B}");
    let w_cleric_kicker = card(
        W_CLERIC_B_KICKER,
        FaceDef {
            subtypes: &[creature::CLERIC],
            additional_costs: KICKER_B,
            enter_modifiers: &[EnterModifier::IfKicked {
                index: None,
                counters: 1,
                modifiers: &[],
            }],
            ..creature_face("W Cleric With B Kicker", "{2}{W}", 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let non_cleric = card(
        NON_CLERIC_WB,
        creature_face("WB Non-Cleric", "{1}{W}{B}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. {1}{W}{B} Cleric becomes {1}, castable off one Forest; {1}{W}{B} non-Cleric is not.
    let mut engine = bench(
        1,
        vec![reducer, wb_cleric, w_cleric_kicker, non_cleric],
        [
            Seat::with(&[EDGEWALKER_REDUCER, forest()]).holding(&[
                WB_CLERIC,
                W_CLERIC_B_KICKER,
                NON_CLERIC_WB,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let wb_id = the(&engine, ZoneLocation::Hand(me()), WB_CLERIC);
    let w_id = the(&engine, ZoneLocation::Hand(me()), W_CLERIC_B_KICKER);
    let non_id = the(&engine, ZoneLocation::Hand(me()), NON_CLERIC_WB);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(wb_id, CastModeKind::Normal), 2);
    assert_eq!(
        legal.spell_reduction(w_id, CastModeKind::Normal),
        1,
        "colored_only reducer saves {{W}} on {{2}}{{W}} and leaves {{B}} unused"
    );
    assert_eq!(legal.spell_reduction(non_id, CastModeKind::Normal), 0);

    float_all(&mut engine, me());
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&wb_id),
        "{{1}}{{W}}{{B}} reduced by {{W}}{{B}} costs {{1}}, payable by one Forest"
    );
    assert!(
        !legal.castable.contains(&w_id),
        "{{2}}{{W}} reduced by colored-only {{W}}{{B}} still costs {{2}}, not {{1}}"
    );
    assert!(!legal.castable.contains(&non_id));

    engine
        .apply(me(), PlayerAction::CastSpell { card: wb_id })
        .expect("WB Cleric casts for {1}");
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, WB_CLERIC).len(),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // 2. Leftover {B} credit applies to optional {B} kicker on {2}{W} Cleric -> total {2}!
    let mut engine = bench(
        2,
        vec![reducer, w_cleric_kicker],
        [
            Seat::with(&[EDGEWALKER_REDUCER, forest(), forest()]).holding(&[W_CLERIC_B_KICKER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let w_id = the(&engine, ZoneLocation::Hand(me()), W_CLERIC_B_KICKER);
    engine
        .apply(me(), PlayerAction::CastSpell { card: w_id })
        .expect("cast W Cleric");
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::Kicker,
                ..
            }
        ),
        "kicker {{B}} is payable because leftover {{B}} reduction covers it (CR 601.2f)"
    );
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept kicker");
    settle(&mut engine, me());
    let landed = the(&engine, ZoneLocation::Battlefield, W_CLERIC_B_KICKER);
    assert_eq!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "entered kicked for total {{2}}"
    );
}

#[test]
fn colored_cost_reduction_with_spillover_and_after_increases() {
    static SPILLOVER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: YOUR_GREEN_SPELL,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLessMana {
            cost: ManaCost::parse("{G}{G}"),
            colored_only: false,
        },
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static TAX_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::ControlledByYou,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostMoreMana(ManaCost::parse("{G}")),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static CO_G_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::ControlledByYou,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLessMana {
            cost: ManaCost::parse("{G}"),
            colored_only: true,
        },
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];

    let spillover = card(
        SPILLOVER_REDUCER,
        face("Green Spillover Reducer", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SPILLOVER_ABILITIES,
    );
    let green_spell = card(
        GREEN_TWO_G,
        creature_face("Two-G Bear", "{2}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let taxer = card(
        GREEN_TAXER,
        face("Green Pip Tax", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TAX_ABILITIES,
    );
    let co_reducer = card(
        COLORED_ONLY_G_REDUCER,
        face("Colored Only G Reducer", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CO_G_ABILITIES,
    );
    let generic_one = card(
        GENERIC_ONE_SPELL,
        creature_face("Colorless Construct", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    // Part A: CR 118.7c — {2}{G} reduced by {G}{G} (colored_only: false) becomes {1}.
    let mut engine = bench(
        3,
        vec![spillover, green_spell],
        [
            Seat::with(&[SPILLOVER_REDUCER, forest()]).holding(&[GREEN_TWO_G]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let spell_id = the(&engine, ZoneLocation::Hand(me()), GREEN_TWO_G);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_id })
        .expect("{2}{G} reduced by {G}{G} with spillover casts for {1}");
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, GREEN_TWO_G).len(),
        1
    );

    // Part B: CR 601.2f — increases apply before reductions: {1} + {G} tax - {G} colored_only = {1}.
    let mut engine = bench(
        4,
        vec![taxer, co_reducer, generic_one],
        [
            Seat::with(&[GREEN_TAXER, COLORED_ONLY_G_REDUCER, forest()])
                .holding(&[GENERIC_ONE_SPELL]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let construct_id = the(&engine, ZoneLocation::Hand(me()), GENERIC_ONE_SPELL);
    float_all(&mut engine, me());
    {
        let state = engine.dev_state_mut(me()).unwrap();
        state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
        state.players[0]
            .mana_pool
            .add(baylee_core::mana::ManaColor::Colorless, 1);
    }
    engine
        .apply(me(), PlayerAction::CastSpell { card: construct_id })
        .expect("tax {G} is removed by colored-only {G} reducer, leaving {1} colorless");
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, GENERIC_ONE_SPELL).len(),
        1
    );
}

#[test]
fn cycling_cost_reduction_makes_two_generic_cycling_free_for_controller_only() {
    static FLUCTUATOR_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::ControlledByYou,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::CyclingCostsLess(2),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static CYCLING_TWO: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{2}"),
            parts: &[CostPart::DiscardSelf],
        },
        effects: &[Effect::draw(1)],
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];

    let fluctuator = card(
        FLUCTUATOR_REDUCER,
        face("Cycling Reducer", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FLUCTUATOR_ABILITIES,
    );
    let cycler = card(
        CYCLER_TWO,
        creature_face("Two-Cycle Beast", "{4}{G}", 4, 4),
        KeywordSet::EMPTY,
        CYCLING_TWO,
    );

    let mut engine = bench(
        5,
        vec![fluctuator, cycler],
        [
            Seat::with(&[FLUCTUATOR_REDUCER]).holding(&[CYCLER_TWO]),
            Seat::default().holding(&[CYCLER_TWO]),
        ],
    );
    to_main(&mut engine, me());

    let my_cycler = the(&engine, ZoneLocation::Hand(me()), CYCLER_TWO);
    let their_cycler = the(&engine, ZoneLocation::Hand(them()), CYCLER_TWO);
    assert!(
        !engine
            .compute_legal(them())
            .abilities
            .contains(&(their_cycler, 0)),
        "opponent's cycling ability still costs {{2}} and is not payable off an empty pool"
    );

    activate(&mut engine, me(), my_cycler, 0);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), CYCLER_TWO).len(),
        1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        1,
        "cycled for {{0}} and drew a card"
    );
}

#[test]
fn face_down_and_creature_type_sharing_cost_reductions_apply_selectively() {
    static CHISEL_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: YOUR_FACEDOWN_CREATURE,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLess(1),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static WARCHIEF_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: YOUR_SHARING_CREATURE,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLess(1),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];

    let chisel = card(
        DREAM_CHISEL_REDUCER,
        face("Face-Down Reducer", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CHISEL_ABILITIES,
    );
    let morph_bear = card(
        MORPH_BEAR,
        FaceDef {
            morph: Some(ManaCost::parse("{2}{G}")),
            ..creature_face("Morphing Beast", "{4}{G}", 4, 4)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let warchief = card(
        MISTFORM_REDUCER,
        FaceDef {
            subtypes: &[creature::WIZARD],
            ..creature_face("Tribal Reducer", "{2}{U}", 1, 3)
        },
        KeywordSet::EMPTY,
        WARCHIEF_ABILITIES,
    );
    let wizard_spell = card(
        WIZARD_SPELL,
        FaceDef {
            subtypes: &[creature::WIZARD],
            ..creature_face("Apprentice Wizardling", "{1}{G}", 2, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let goblin_spell = card(
        GOBLIN_SPELL,
        FaceDef {
            subtypes: &[creature::GOBLIN],
            ..creature_face("Unrelated Goblin", "{1}{G}", 2, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );

    // 1. Face-down {3} cast reduced to {2} by Filter::FaceDown.
    let mut engine = bench(
        6,
        vec![chisel, morph_bear],
        [
            Seat::with(&[DREAM_CHISEL_REDUCER, forest(), forest()]).holding(&[MORPH_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let morph_id = the(&engine, ZoneLocation::Hand(me()), MORPH_BEAR);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(morph_id, CastModeKind::Normal), 0);
    assert_eq!(legal.spell_reduction(morph_id, CastModeKind::Disguise), 1);

    float_all(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: morph_id })
        .expect("cast face-down for reduced {2}");
    settle(&mut engine, me());
    let landed = the(&engine, ZoneLocation::Battlefield, MORPH_BEAR);
    assert!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .status
            .contains(crate::object::Status::FACE_DOWN)
    );

    // 2. SharesCreatureTypeWith(&Filter::This) reduces Wizard {1}{G} to {G}, leaves Goblin at {1}{G}.
    let mut engine = bench(
        7,
        vec![warchief, wizard_spell, goblin_spell],
        [
            Seat::with(&[MISTFORM_REDUCER, forest()]).holding(&[WIZARD_SPELL, GOBLIN_SPELL]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let wiz_id = the(&engine, ZoneLocation::Hand(me()), WIZARD_SPELL);
    let gob_id = the(&engine, ZoneLocation::Hand(me()), GOBLIN_SPELL);
    let (_, legal) = offer(&engine);
    assert!(legal.castable.contains(&wiz_id));
    assert!(!legal.castable.contains(&gob_id));
}

#[test]
fn stack_zone_self_reduction_and_target_dependent_stack_tax_work_end_to_end() {
    static SELF_REDUCE_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostLessByAmount(Amount::CountOf {
            filter: &Filter::YOUR_LAND,
            zone: baylee_cards_dsl::ZoneSel::Battlefield,
        }),
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static TORCH_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::SpellTargetingAffected,
            layer: baylee_cards_dsl::Layer::Text,
            modifier: Modifier::SpellsCostMore(2),
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        AbilityDef::Spell {
            effects: &[],
            targets: None,
            second_targets: None,
            condition: None,
        },
    ];
    static COUNTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        second_targets: None,
        condition: None,
    }];

    let self_reduced = card(
        SELF_STACK_REDUCED,
        creature_face("Land-Count Golem", "{4}", 3, 3),
        KeywordSet::EMPTY,
        SELF_REDUCE_ABILITIES,
    );
    let torch = card(
        TORCH_ON_STACK,
        face("Taxing Torch", "{G}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        TORCH_ABILITIES,
    );
    let decoy = card(
        DECOY_INSTANT,
        face("Decoy Instant", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NOTHING_SPELL,
    );
    let counter = card(
        PROBE_COUNTER,
        face("Green Cancel", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTER_ABILITIES,
    );

    // 1. Self ZoneRef::Stack cost reduction applies while card is still in hand:
    //    3 Forests reduce {4} by 3 -> {1}, playable by tapping just 1 of the 3 Forests!
    let mut engine = bench(
        8,
        vec![self_reduced],
        [
            Seat::with(&[forest(), forest(), forest()]).holding(&[SELF_STACK_REDUCED]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let golem_id = the(&engine, ZoneLocation::Hand(me()), SELF_STACK_REDUCED);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(golem_id, CastModeKind::Normal), 3);
    let one_forest = legal.mana_abilities[0];
    engine
        .apply(
            me(),
            PlayerAction::ActivateManaAbility { source: one_forest },
        )
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: golem_id })
        .expect("{4} reduced by 3 controlled lands casts for {1}");
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, SELF_STACK_REDUCED).len(),
        1
    );

    // 2. Stack-zone target tax (Kaervek's Torch): targeting Torch costs {2}{G},
    //    targeting Decoy on the same stack costs {G}.
    let mut engine = bench(
        9,
        vec![torch, decoy, counter],
        [
            Seat::with(&[forest(), forest(), forest()]).holding(&[
                TORCH_ON_STACK,
                DECOY_INSTANT,
                PROBE_COUNTER,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me()); // 3 green floating, 0 untapped lands left.

    let torch_id = the(&engine, ZoneLocation::Hand(me()), TORCH_ON_STACK);
    let decoy_id = the(&engine, ZoneLocation::Hand(me()), DECOY_INSTANT);
    let counter_id = the(&engine, ZoneLocation::Hand(me()), PROBE_COUNTER);

    engine
        .apply(me(), PlayerAction::CastSpell { card: torch_id })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: decoy_id })
        .unwrap();
    // 1 green mana left in pool; no untapped lands!
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    // Attempting to target Torch with only {G} floating reverses the cast (needs {2}{G})!
    engine
        .apply(me(), PlayerAction::CastSpell { card: counter_id })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![torch_id],
            },
        )
        .unwrap();
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), PROBE_COUNTER).len(),
        1,
        "targeting Torch requires {{2}}{{G}}, so {{G}} alone reverses the cast back to hand"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    // Targeting Decoy with the same {G} succeeds!
    engine
        .apply(me(), PlayerAction::CastSpell { card: counter_id })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![decoy_id],
            },
        )
        .unwrap();
    assert_eq!(
        objects(&engine, ZoneLocation::Stack, PROBE_COUNTER).len(),
        1,
        "targeting Decoy incurs no tax and goes onto the stack for {{G}}"
    );
}

#[test]
fn non_mana_return_to_hand_kicker_skips_when_unpayable_and_pays_when_accepted() {
    static BOUNCE_KICKER: &[Cost] = &[Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::ReturnToHand(&Filter::YOUR_CREATURE)],
    }];

    let merfolk = card(
        BOUNCE_KICKER_MERFOLK,
        FaceDef {
            additional_costs: BOUNCE_KICKER,
            enter_modifiers: &[EnterModifier::IfKicked {
                index: None,
                counters: 1,
                modifiers: &[],
            }],
            ..creature_face("Bounce-Kicker Merfolk", "{1}{G}", 1, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let bear = card(
        HELPER_BEAR,
        creature_face("Helper Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. Empty creature board: Kicker prompt is skipped automatically!
    let mut engine = bench(
        10,
        vec![merfolk, bear],
        [
            Seat::with(&[forest(), forest()]).holding(&[BOUNCE_KICKER_MERFOLK]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let merfolk_id = the(&engine, ZoneLocation::Hand(me()), BOUNCE_KICKER_MERFOLK);
    engine
        .apply(me(), PlayerAction::CastSpell { card: merfolk_id })
        .expect("cast Merfolk without creatures on board");
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::Kicker,
                ..
            }
        ),
        "unpayable non-mana kicker must not prompt YesNoPrompt::Kicker"
    );
    settle(&mut engine, me());
    let landed = the(&engine, ZoneLocation::Battlefield, BOUNCE_KICKER_MERFOLK);
    assert_eq!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // 2. With Helper Bear on board: Kicker prompts, bounces Bear, enters with +1/+1 counter!
    let mut engine = bench(
        11,
        vec![merfolk, bear],
        [
            Seat::with(&[HELPER_BEAR, forest(), forest()]).holding(&[BOUNCE_KICKER_MERFOLK]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, HELPER_BEAR);
    let merfolk_id = the(&engine, ZoneLocation::Hand(me()), BOUNCE_KICKER_MERFOLK);
    engine
        .apply(me(), PlayerAction::CastSpell { card: merfolk_id })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::CostReturn,
            ..
        }
    ));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), HELPER_BEAR).len(),
        1
    );
    let landed = the(&engine, ZoneLocation::Battlefield, BOUNCE_KICKER_MERFOLK);
    assert_eq!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
}

#[test]
fn variable_x_kicker_skips_when_unkicked_and_charges_chosen_x_when_kicked() {
    static X_ONE_KICKER: &[Cost] = &[Cost {
        mana: ManaCost::parse("{X}{1}"),
        parts: &[],
    }];
    static ETB_X_COUNTERS: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::IfKicked {
            then: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::X,
            }],
            otherwise: &[],
        }]
    )];

    let ancient = card(
        X_KICKER_ANCIENT,
        FaceDef {
            additional_costs: X_ONE_KICKER,
            ..creature_face("X-Kicker Ancient", "{1}{G}", 2, 2)
        },
        KeywordSet::EMPTY,
        ETB_X_COUNTERS,
    );

    // 1. Only {1}{G} floating (cannot afford even X=0's fixed {1} kicker): Kicker & X both skipped!
    let mut engine = bench(
        12,
        vec![ancient],
        [
            Seat::with(&[forest(), forest()]).holding(&[X_KICKER_ANCIENT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let ancient_id = the(&engine, ZoneLocation::Hand(me()), X_KICKER_ANCIENT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ancient_id })
        .unwrap();
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo { .. } | Pending::ChooseNumber { .. }
        ),
        "neither Kicker nor X is asked when {{1}} fixed kicker mana is unaffordable"
    );
    settle(&mut engine, me());

    // 2. {4}{G} floating (5 Forests): declining kicker skips X and spends only {1}{G}!
    let mut engine = bench(
        13,
        vec![ancient],
        [
            Seat::with(&[forest(), forest(), forest(), forest(), forest()])
                .holding(&[X_KICKER_ANCIENT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let ancient_id = the(&engine, ZoneLocation::Hand(me()), X_KICKER_ANCIENT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ancient_id })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::ChooseNumber { .. }),
        "declining variable-X kicker must not ask for X"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    settle(&mut engine, me());

    // 3. {4}{G} floating (5 Forests): accepting kicker asks NumberPrompt::X with max = 2,
    //    spends all 5 mana for X = 2, and enters with 2 +1/+1 counters!
    let mut engine = bench(
        14,
        vec![ancient],
        [
            Seat::with(&[forest(), forest(), forest(), forest(), forest()])
                .holding(&[X_KICKER_ANCIENT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let ancient_id = the(&engine, ZoneLocation::Hand(me()), X_KICKER_ANCIENT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ancient_id })
        .unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            player,
            min: 0,
            max: 2,
            reason: crate::choice::NumberPrompt::X,
        } if *player == me()
    ));
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    settle(&mut engine, me());
    let landed = the(&engine, ZoneLocation::Battlefield, X_KICKER_ANCIENT);
    assert_eq!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
}

#[test]
fn mandatory_additional_spell_costs_pay_return_to_hand_random_discard_and_sacrifice_all() {
    static BOUNCE_PART: &[CostPart] = &[CostPart::ReturnToHand(&Filter::YOUR_CREATURE)];
    static RANDOM_PART: &[CostPart] = &[CostPart::DiscardRandom];
    static SPITE_PARTS: &[CostPart] = &[
        CostPart::SacrificeAll(&Filter::CREATURE),
        CostPart::DiscardHand,
    ];
    static EXILE_TOP_PART: &[CostPart] = &[CostPart::ExileTopOfLibrary(1)];

    let bounce_spell = card(
        MANDATORY_BOUNCE_SPELL,
        FaceDef {
            mandatory_additional_costs: BOUNCE_PART,
            ..face("Rescue Cost Sorcery", "{G}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        NOTHING_SPELL,
    );
    let random_spell = card(
        MANDATORY_RANDOM_DISCARD,
        FaceDef {
            mandatory_additional_costs: RANDOM_PART,
            ..face("Sonic Cost Instant", "{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        NOTHING_SPELL,
    );
    let spite_spell = card(
        MANDATORY_SPITE,
        FaceDef {
            mandatory_additional_costs: SPITE_PARTS,
            ..face("All-In Spite", "{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        NOTHING_SPELL,
    );
    let bear = card(
        HELPER_BEAR,
        creature_face("Helper Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. ReturnToHand mandatory cost: refused without creature, paid when creature present.
    let mut engine = bench(
        15,
        vec![bounce_spell, random_spell, bear],
        [
            Seat::with(&[forest()]).holding(&[MANDATORY_BOUNCE_SPELL, MANDATORY_RANDOM_DISCARD]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let bounce_id = the(&engine, ZoneLocation::Hand(me()), MANDATORY_BOUNCE_SPELL);
    let random_id = the(&engine, ZoneLocation::Hand(me()), MANDATORY_RANDOM_DISCARD);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&bounce_id),
        "cannot cast ReturnToHand additional-cost spell with no creature controlled"
    );
    assert!(
        legal.castable.contains(&random_id),
        "can cast DiscardRandom spell while holding another card in hand"
    );

    // Cast random_spell: discards bounce_spell at random (the only other card in hand)!
    engine
        .apply(me(), PlayerAction::CastSpell { card: random_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        objects(
            &engine,
            ZoneLocation::Graveyard(me()),
            MANDATORY_BOUNCE_SPELL
        )
        .len(),
        1
    );
    assert_eq!(
        objects(
            &engine,
            ZoneLocation::Graveyard(me()),
            MANDATORY_RANDOM_DISCARD
        )
        .len(),
        1
    );

    // 2. ReturnToHand mandatory cost paid + SacrificeAll & DiscardHand paid.
    let mut engine = bench(
        16,
        vec![bounce_spell, spite_spell, bear],
        [
            Seat::with(&[HELPER_BEAR, HELPER_BEAR, forest(), forest()])
                .holding(&[MANDATORY_BOUNCE_SPELL, MANDATORY_SPITE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let bears = objects(&engine, ZoneLocation::Battlefield, HELPER_BEAR);
    let bounce_id = the(&engine, ZoneLocation::Hand(me()), MANDATORY_BOUNCE_SPELL);
    let spite_id = the(&engine, ZoneLocation::Hand(me()), MANDATORY_SPITE);

    engine
        .apply(me(), PlayerAction::CastSpell { card: bounce_id })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bears[0]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), HELPER_BEAR).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, HELPER_BEAR).len(),
        1
    );

    // Cast spite_spell: sacrifices remaining Bear on battlefield and discards returned Bear from hand!
    engine
        .apply(me(), PlayerAction::CastSpell { card: spite_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, HELPER_BEAR).len(),
        0
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty()
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), HELPER_BEAR).len(),
        2
    );

    // 3. ExileTopOfLibrary(1) mandatory cost: exiles the top card of library (.last()),
    //    not the bottom card (.first()), into its owner's exile zone.
    let exile_top_spell = card(
        MANDATORY_EXILE_TOP,
        FaceDef {
            mandatory_additional_costs: EXILE_TOP_PART,
            ..face("Top Exile Instant", "{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        NOTHING_SPELL,
    );
    let mut engine = bench(
        17,
        vec![exile_top_spell, bear],
        [
            Seat::with(&[forest()]).holding(&[MANDATORY_EXILE_TOP, HELPER_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let top_bear = the(&engine, ZoneLocation::Hand(me()), HELPER_BEAR);
    engine
        .state
        .move_object(
            top_bear,
            ZoneLocation::Library(me()),
            crate::engine::ZonePosition::Top,
            crate::engine::Cause::Effect,
        )
        .unwrap();
    let exile_spell_id = the(&engine, ZoneLocation::Hand(me()), MANDATORY_EXILE_TOP);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: exile_spell_id,
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), HELPER_BEAR).len(),
        1,
        "ExileTopOfLibrary(1) must exile the top card (HELPER_BEAR), not a bottom Forest"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), forest()).len(),
        0
    );
}
