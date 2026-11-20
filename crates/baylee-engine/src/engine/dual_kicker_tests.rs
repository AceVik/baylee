//! Dual kicker (CR 702.33a–b, CR 702.33d–f): `Kicker [A] and/or [B]`,
//! `Condition::KickedWith`, Battlemage intervening `if` triggers (CR 603.4,
//! CR 608.2h, CR 400.7d), and Volver `EnterModifier::IfKicked` (CR 614.1c,
//! CR 614.16, CR 611.2e, CR 707.2, CR 400.7).

#[allow(clippy::wildcard_imports)]
use super::*;

use super::synthetic::{self, SyntheticLookup};
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Condition, CounterKind, Coverage, Effect,
    EnterModifier, FaceDef, Filter, KeywordSet, Layer, Modifier, StaticAbility, TargetReq,
    TargetSpec, Trigger, TriggeredParts,
};
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);
const OPP: PlayerId = PlayerId::new(1);

const DUAL_SPELL: u32 = 990_201;
const BATTLEMAGE: u32 = 990_202;
const VOLVER: u32 = 990_203;
const DOUBLER: u32 = 990_204;
const CLONE: u32 = 990_205;
const TARGET_CREATURE: u32 = 990_206;
const SPELL_COPIER: u32 = 990_207;

/// Dual-kicker sorcery: "{R}, Kicker {1}{G} and/or {2}{U}".
/// Gain 1 life; if kicked with 0, gain 3 life; if kicked with 1, draw 2 cards.
const DUAL_SPELL_EFFECTS: &[Effect] = &[
    Effect::GainLife {
        amount: Amount::Fixed(1),
    },
    Effect::IfCondition {
        condition: Condition::KickedWith(0),
        then: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        otherwise: &[],
    },
    Effect::IfCondition {
        condition: Condition::KickedWith(1),
        then: &[Effect::draw(2)],
        otherwise: &[],
    },
];

const DUAL_SPELL_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(DUAL_SPELL_EFFECTS)];

/// Battlemage: "{2}{W}, Kicker {1}{G} and/or {2}{U}".
/// ETB if kicked with 0 -> destroy target creature;
/// ETB if kicked with 1 -> draw 2 cards.
const BM_DESTROY: &[Effect] = &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))];
const BM_DRAW: &[Effect] = &[Effect::draw(2)];

const BATTLEMAGE_ABILITIES: &[AbilityDef] = &[
    {
        let mut p = TriggeredParts::new(Trigger::ETB, BM_DESTROY);
        p.targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)));
        p.condition = Some(Condition::KickedWith(0));
        p.build()
    },
    {
        let mut p = TriggeredParts::new(Trigger::ETB, BM_DRAW);
        p.condition = Some(Condition::KickedWith(1));
        p.build()
    },
];

/// Volver: "{3}{G}, Kicker {1}{U} and/or {2}{B}".
/// If kicked with 0 -> 2 +1/+1 counters and flying.
/// If kicked with 1 -> 1 +1/+1 counter, "{1}{B}: Regenerate this creature",
/// and "Whenever this creature deals damage, you gain that much life."
const REGEN_SELF: &[Effect] = &[Effect::Regenerate {
    target: TargetSpec::ThisObject,
}];
const GAIN_DEALT: &[Effect] = &[Effect::GainLife {
    amount: Amount::EventAmount,
}];

const VOLVER_MODS_0: &[Modifier] = &[Modifier::AddKeyword(KeywordSet::FLYING)];
const VOLVER_MODS_1: &[Modifier] = &[
    Modifier::GrantActivated {
        cost: baylee_cards_dsl::cost!("{1}{B}"),
        effects: REGEN_SELF,
        mana_ability: false,
    },
    Modifier::GrantTriggered {
        trigger: Trigger::DealsDamage(&Filter::This),
        effects: GAIN_DEALT,
        target: None,
    },
];

const VOLVER_ENTERS: &[EnterModifier] = &[
    EnterModifier::IfKicked {
        index: Some(0),
        counters: 2,
        modifiers: VOLVER_MODS_0,
    },
    EnterModifier::IfKicked {
        index: Some(1),
        counters: 1,
        modifiers: VOLVER_MODS_1,
    },
];

const DOUBLER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::Ability,
    filter: Filter::ControlledByYou,
    modifier: Modifier::DoubleCountersPlacedByYou,
    condition: None,
    source_zone: baylee_cards_dsl::ZoneRef::Battlefield,
})];

const CLONE_ABILITIES: &[AbilityDef] = &[AbilityDef::CopyOnEnter {
    target: TargetSpec::Object(&Filter::CREATURE),
    mods: &[],
}];

const COPY_SPELL_EFFECTS: &[Effect] = &[Effect::CopyTargetSpell { mods: &[] }];
const SPELL_COPIER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    COPY_SPELL_EFFECTS,
    targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
)];

fn defined(index: u32, face: FaceDef, abilities: &'static [AbilityDef]) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn cards() -> SyntheticLookup {
    let dual_spell = FaceDef {
        mana_cost: ManaCost::parse("{R}"),
        types: TypeSet::SORCERY,
        castable_from_hand: true,
        additional_costs: &[
            baylee_cards_dsl::cost!("{1}{G}"),
            baylee_cards_dsl::cost!("{2}{U}"),
        ],
        ..synthetic::land_face("Dual Spell")
    };
    let battlemage = FaceDef {
        mana_cost: ManaCost::parse("{2}{W}"),
        castable_from_hand: true,
        additional_costs: &[
            baylee_cards_dsl::cost!("{1}{G}"),
            baylee_cards_dsl::cost!("{2}{U}"),
        ],
        ..synthetic::creature_face("Dual Battlemage", 2, 2, &[])
    };
    let volver = FaceDef {
        mana_cost: ManaCost::parse("{3}{G}"),
        castable_from_hand: true,
        additional_costs: &[
            baylee_cards_dsl::cost!("{1}{U}"),
            baylee_cards_dsl::cost!("{2}{B}"),
        ],
        ..synthetic::creature_face("Dual Volver", 3, 3, VOLVER_ENTERS)
    };
    let doubler = FaceDef {
        mana_cost: ManaCost::parse("{4}{G}"),
        types: TypeSet::ENCHANTMENT,
        castable_from_hand: true,
        ..synthetic::land_face("Counter Doubler")
    };
    let clone = FaceDef {
        mana_cost: ManaCost::parse("{3}{U}"),
        castable_from_hand: true,
        ..synthetic::creature_face("Plain Clone", 0, 0, &[])
    };
    let dummy = FaceDef {
        mana_cost: ManaCost::parse("{1}"),
        castable_from_hand: true,
        ..synthetic::creature_face("Target Dummy", 2, 2, &[])
    };
    let copier = FaceDef {
        mana_cost: ManaCost::parse("{R}"),
        types: TypeSet::INSTANT,
        castable_from_hand: true,
        ..synthetic::land_face("Spell Copier")
    };
    SyntheticLookup::new(vec![
        defined(DUAL_SPELL, dual_spell, DUAL_SPELL_ABILITIES),
        defined(BATTLEMAGE, battlemage, BATTLEMAGE_ABILITIES),
        defined(VOLVER, volver, &[]),
        defined(DOUBLER, doubler, DOUBLER_ABILITIES),
        defined(CLONE, clone, CLONE_ABILITIES),
        defined(TARGET_CREATURE, dummy, &[]),
        defined(SPELL_COPIER, copier, SPELL_COPIER_ABILITIES),
    ])
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    }
}

fn setup(
    hand: &[u32],
    my_board: &[u32],
    opp_board: &[u32],
    mana: &[(ManaColor, u32)],
) -> Engine<SyntheticLookup> {
    let mut preset: GamePreset = synthetic::preset(97, my_board);
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(entry).collect());
    preset.seats[1].starting_battlefield = opp_board.iter().copied().map(entry).collect();
    let mut engine = Engine::new(&preset, cards()).expect("board builds");
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..40 {
        if engine.state().turn.active == ME
            && engine.state().turn.phase == Phase::FirstMain
            && matches!(engine.pending(), Pending::Priority { player: ME, .. })
        {
            break;
        }
        let q = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &q));
    }
    assert_eq!(engine.state().turn.active, ME);
    assert_eq!(engine.state().turn.phase, Phase::FirstMain);
    {
        let state = engine.dev_state_mut(ME).unwrap();
        for &(color, amount) in mana {
            state.players[0].mana_pool.add(color, amount);
        }
    }
    // Recompute legal actions with the newly added floating mana.
    engine.refresh_offer();
    engine
}

fn flush_triggers_and_sbas(engine: &mut Engine<SyntheticLookup>) {
    engine.awaiting_answer = false;
    engine.after_action(ME);
    engine.run_machine();
}

fn in_hand(engine: &Engine<SyntheticLookup>, card: u32) -> ObjectId {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index.get() == card)
        })
        .expect("card in hand")
}

fn resolve_all(engine: &mut Engine<SyntheticLookup>) {
    for _ in 0..30 {
        if engine.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending while resolving: {other:?}"),
        }
    }
    panic!("stack did not empty");
}

#[test]
fn dual_kicker_spell_supports_none_first_second_and_both_with_times_kicked_two() {
    let plenty = &[
        (ManaColor::Red, 1),
        (ManaColor::Green, 2),
        (ManaColor::Blue, 3),
        (ManaColor::Colorless, 4),
    ];

    for (kick0, kick1, exp_times, exp_mask, exp_life, exp_draws) in [
        (false, false, 0u8, None, 21i32, 0usize),
        (true, false, 1u8, Some(0b01), 24i32, 0usize),
        (false, true, 1u8, Some(0b10), 21i32, 2usize),
        (true, true, 2u8, Some(0b11), 24i32, 2usize),
    ] {
        let mut engine = setup(&[DUAL_SPELL], &[], &[], plenty);
        let hand_before = engine.state().zones.list(ZoneLocation::Hand(ME)).len();
        let spell = in_hand(&engine, DUAL_SPELL);
        engine
            .apply(ME, PlayerAction::CastSpell { card: spell })
            .unwrap();

        match engine.pending() {
            Pending::YesNo {
                prompt: YesNoPrompt::KickerOption { index: 0, cost, .. },
                ..
            } => assert_eq!(*cost, ManaCost::parse("{1}{G}")),
            other => panic!("expected KickerOption 0, got {other:?}"),
        }
        engine.apply(ME, PlayerAction::YesNo(kick0)).unwrap();

        match engine.pending() {
            Pending::YesNo {
                prompt: YesNoPrompt::KickerOption { index: 1, cost, .. },
                ..
            } => assert_eq!(*cost, ManaCost::parse("{2}{U}")),
            other => panic!("expected KickerOption 1, got {other:?}"),
        }
        engine.apply(ME, PlayerAction::YesNo(kick1)).unwrap();

        let stack_id = *engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .last()
            .expect("spell on stack");
        let stack_obj = engine.state().object(stack_id).unwrap();
        assert_eq!(stack_obj.times_kicked, exp_times);
        assert_eq!(stack_obj.kicked_mask(), exp_mask);
        assert_eq!(stack_obj.kicked_with(0), kick0);
        assert_eq!(stack_obj.kicked_with(1), kick1);

        resolve_all(&mut engine);
        assert_eq!(engine.state().players[0].life, exp_life);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(ME)).len(),
            hand_before - 1 + exp_draws
        );
    }
}

#[test]
fn dual_kicker_skips_second_option_when_first_exhausts_mana() {
    // {R} + {1}{G} + {2}{U} = {3}{R}{G}{U}; give only {2}{R}{G}{U} (enough for
    // either kicker individually, not both together).
    let tight = &[
        (ManaColor::Red, 1),
        (ManaColor::Green, 1),
        (ManaColor::Blue, 1),
        (ManaColor::Colorless, 2),
    ];

    // Taking kicker 0 leaves only {1}{U}, which cannot pay {2}{U}, so kicker 1 is skipped.
    let mut engine = setup(&[DUAL_SPELL], &[], &[], tight);
    let spell = in_hand(&engine, DUAL_SPELL);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spell })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::KickerOption { index: 0, .. },
            ..
        }
    ));
    engine.apply(ME, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "second kicker should be skipped when combined cost is unpayable, got {:?}",
        engine.pending()
    );

    // Declining kicker 0 leaves {2}{G}{U}, which CAN pay {2}{U}, so kicker 1 IS offered.
    let mut engine2 = setup(&[DUAL_SPELL], &[], &[], tight);
    let spell2 = in_hand(&engine2, DUAL_SPELL);
    engine2
        .apply(ME, PlayerAction::CastSpell { card: spell2 })
        .unwrap();
    engine2.apply(ME, PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(
            engine2.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::KickerOption { index: 1, .. },
                ..
            }
        ),
        "second kicker should be offered when first was declined, got {:?}",
        engine2.pending()
    );
}

#[test]
fn copying_a_dual_kicked_spell_preserves_kicked_mask_and_resolves_same_branch() {
    // {R} + {1}{G} = {1}{R}{G}, leaving {R} in the pool for the instant copier.
    let mana = &[(ManaColor::Red, 3), (ManaColor::Green, 1)];
    let mut engine = setup(&[DUAL_SPELL, SPELL_COPIER], &[], &[], mana);
    let spell = in_hand(&engine, DUAL_SPELL);
    let copier = in_hand(&engine, SPELL_COPIER);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(ME, PlayerAction::YesNo(true)).unwrap();
    let stack_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    // Cast the instant copier targeting the kicked spell on the stack (CR 707.10).
    engine
        .apply(ME, PlayerAction::CastSpell { card: copier })
        .unwrap();
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![stack_id],
            },
        )
        .unwrap();
    // Resolve the copier so the copy sits on top of the stack.
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    engine.apply(OPP, PlayerAction::PassPriority).unwrap();
    let copy_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    assert_ne!(copy_id, stack_id);
    let copy_obj = engine.state().object(copy_id).unwrap();
    assert_eq!(copy_obj.times_kicked, 1);
    assert_eq!(copy_obj.kicked_mask(), Some(0b01));
    assert!(copy_obj.kicked_with(0));
    assert!(!copy_obj.kicked_with(1));

    // Distinct kicker masks produce distinct state hashes.
    let hash_mask_1 = engine.state().snapshot_hash();
    {
        let state = engine.dev_state_mut(ME).unwrap();
        let riders = &mut state.object_mut(copy_id).unwrap().riders;
        for rider in riders.iter_mut() {
            if let crate::object::Rider::KickedMask(mask) = rider {
                *mask = 0b10;
            }
        }
    }
    let hash_mask_2 = engine.state().snapshot_hash();
    assert_ne!(
        hash_mask_1, hash_mask_2,
        "snapshot_hash must distinguish KickedMask(0b01) from KickedMask(0b10)"
    );
    {
        let state = engine.dev_state_mut(ME).unwrap();
        let riders = &mut state.object_mut(copy_id).unwrap().riders;
        for rider in riders.iter_mut() {
            if let crate::object::Rider::KickedMask(mask) = rider {
                *mask = 0b01;
            }
        }
    }

    resolve_all(&mut engine);
    // Original + copy each gain 1 + 3 = 4 life (total 28).
    assert_eq!(engine.state().players[0].life, 28);
}

#[test]
fn battlemage_intervening_if_only_stacks_chosen_kicker_and_survives_lki_departure() {
    let mana = &[
        (ManaColor::White, 1),
        (ManaColor::Green, 1),
        (ManaColor::Blue, 1),
        (ManaColor::Colorless, 5),
    ];
    let mut engine = setup(&[BATTLEMAGE], &[], &[TARGET_CREATURE], mana);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(ME)).len();
    let dummy = synthetic::permanents(&engine, TARGET_CREATURE)[0];
    let bm = in_hand(&engine, BATTLEMAGE);

    // Cast kicking ONLY 1st kicker ({1}{G} -> destroy target creature).
    engine
        .apply(ME, PlayerAction::CastSpell { card: bm })
        .unwrap();
    engine.apply(ME, PlayerAction::YesNo(true)).unwrap();
    engine.apply(ME, PlayerAction::YesNo(false)).unwrap();
    // Resolve creature spell onto the battlefield.
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    engine.apply(OPP, PlayerAction::PassPriority).unwrap();

    // Only kicker 0 trigger fires and asks for a creature target (CR 603.4).
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected trigger target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&dummy));
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![dummy],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "only the kicked trigger should be on the stack (CR 603.4)"
    );

    // Remove the Battlemage from the battlefield before its trigger resolves (CR 608.2h).
    let bm_perm = synthetic::permanents(&engine, BATTLEMAGE)[0];
    engine
        .dev_state_mut(ME)
        .unwrap()
        .move_object(
            bm_perm,
            ZoneLocation::Graveyard(ME),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();

    resolve_all(&mut engine);
    assert_eq!(
        engine.state().object(dummy).map(|o| o.zone),
        Some(Zone::Graveyard),
        "kicked trigger resolves via LKI even after Battlemage left the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(ME)).len(),
        hand_before - 1,
        "unchosen 2nd kicker trigger must never draw cards"
    );

    // Reanimating the Battlemage (not from stack) must NOT trigger either ETB (CR 400.7).
    let dummy2 = {
        let state = engine.dev_state_mut(ME).unwrap();
        let card_ref = crate::object::CardRef {
            index: CardIndex::new(TARGET_CREATURE),
            print: PrintRef::new(0),
        };
        let def = state.bases.card(CardIndex::new(TARGET_CREATURE)).unwrap();
        let name = state.names.intern(def.faces[0].name);
        let base = crate::object::Characteristics::from_face(def, 0, name);
        let id = state
            .arena
            .insert_with(|id| GameObject::new_card(id, OPP, card_ref, base));
        state
            .move_object(
                id,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                bm_perm,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        id
    };
    let _ = engine.apply(ME, PlayerAction::PassPriority);
    let bm_reanimated = engine.state().object(bm_perm).unwrap();
    assert!(!bm_reanimated.kicked());
    assert_eq!(bm_reanimated.kicked_mask(), None);
    assert!(engine.state().zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(
        engine.state().object(dummy2).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

fn setup_kicked_volver() -> (Engine<SyntheticLookup>, ObjectId) {
    // Plenty of colored mana for Volver ({3}{G} + {1}{U} + {2}{B}),
    // the granted {1}{B} regeneration activation, and Clone ({3}{U}).
    let mana = &[
        (ManaColor::Green, 2),
        (ManaColor::Blue, 10),
        (ManaColor::Black, 10),
    ];
    let mut engine = setup(&[VOLVER, CLONE], &[DOUBLER], &[], mana);
    let volver = in_hand(&engine, VOLVER);
    engine
        .apply(ME, PlayerAction::CastSpell { card: volver })
        .unwrap();
    engine.apply(ME, PlayerAction::YesNo(true)).unwrap();
    engine.apply(ME, PlayerAction::YesNo(true)).unwrap();
    resolve_all(&mut engine);
    let perm = synthetic::permanents(&engine, VOLVER)[0];
    (engine, perm)
}

#[test]
fn volver_enter_modifier_places_doubled_counters_and_grants_abilities() {
    let (mut engine, perm) = setup_kicked_volver();
    let obj = engine.state().object(perm).unwrap();
    assert_eq!(obj.times_kicked, 2);
    // 2 + 1 = 3 counters doubled by Doubling Season to 6 (CR 614.16).
    assert_eq!(obj.counters.get(CounterKind::P1P1), 6);
    assert!(
        obj.characteristics().keywords.contains(KeywordSet::FLYING),
        "kicker 0 granted flying"
    );
    assert_eq!(obj.characteristics().power, Some(9));
    assert_eq!(obj.characteristics().toughness, Some(9));

    // Granted activated ability ({1}{B}: Regenerate) is offered and activates.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let regen_index = legal
        .abilities
        .iter()
        .find_map(|&(src, idx)| (src == perm).then_some(idx))
        .expect("kicker 1 granted activated regeneration ability");
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: perm,
                ability_index: regen_index,
            },
        )
        .unwrap();
    resolve_all(&mut engine);
    assert_eq!(
        engine.state().object(perm).unwrap().regeneration_shields,
        1,
        "activating the granted regeneration ability adds a regeneration shield"
    );

    // Granted triggered ability ("Whenever this creature deals damage, you gain that much life")
    // fires when the Volver deals damage.
    let life_before = engine.state().players[0].life;
    {
        let state = engine.dev_state_mut(ME).unwrap();
        let req = crate::damage::Assignment {
            source: perm,
            source_version: Some(state.object(perm).unwrap().version),
            recipient: crate::event::DamageTarget::Player(OPP),
            amount: 4,
            is_combat: false,
        };
        let mut work = crate::damage::DamageWork::new(state, vec![req]);
        assert!(work.advance(state).is_none());
    }
    flush_triggers_and_sbas(&mut engine);
    resolve_all(&mut engine);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 4,
        "granted damage trigger gained life equal to damage dealt"
    );
}

#[test]
fn volver_granted_abilities_are_noncopiable_and_cleared_on_blink() {
    let (mut engine, perm) = setup_kicked_volver();

    // Copying the kicked Volver does NOT copy its counters or Layer 6 granted abilities (CR 707.2).
    let clone = in_hand(&engine, CLONE);
    engine
        .apply(ME, PlayerAction::CastSpell { card: clone })
        .unwrap();
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    engine.apply(OPP, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected clone copy choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&perm));
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![perm],
            },
        )
        .unwrap();
    resolve_all(&mut engine);
    let clone_obj = engine.state().object(clone).unwrap();
    assert_eq!(clone_obj.zone, Zone::Battlefield);
    assert_eq!(clone_obj.counters.get(CounterKind::P1P1), 0);
    assert!(
        !clone_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "Layer 6 granted flying must not be copied (CR 707.2)"
    );
    assert_eq!(clone_obj.characteristics().power, Some(3));

    // Blinking the kicked Volver clears counters, KickedMask, and indefinite Layer 6 grants (CR 400.7).
    {
        let state = engine.dev_state_mut(ME).unwrap();
        state
            .move_object(
                perm,
                ZoneLocation::Exile(ME),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                perm,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    flush_triggers_and_sbas(&mut engine);
    let blinked = engine.state().object(perm).unwrap();
    assert_eq!(blinked.counters.get(CounterKind::P1P1), 0);
    assert_eq!(blinked.kicked_mask(), None);
    assert!(
        !blinked
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "blinking clears Layer 6 granted abilities (CR 400.7)"
    );
    assert_eq!(blinked.characteristics().power, Some(3));
}
