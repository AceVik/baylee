//! Rule tests for granted triggered/activated definitions (Layer 6, CR 613.1f,
//! CR 613.7–8, CR 603.4, CR 707.2) and Phasing (`KeywordSet::PHASING`,
//! `Effect::PhaseOut`, `Trigger::PhasesIn` / `PhasesOut` /
//! `PhasesOutOrLeavesBattlefield`, indirect phasing, and `Modifier::CantPhaseOut`
//! — CR 702.26, CR 502.1, CR 603.10b).

use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::Cause;
use crate::object::Status;
use baylee_cards_dsl::{
    Amount, Condition, Duration, Filter, Layer, Modifier, PlayerRel, SpellMode, TargetSpec,
    Trigger, TriggerZone, activated, static_ability, triggered,
};
use baylee_core::generated::subtypes::enchantment;
use baylee_core::ids::{Defender, EffectId};

const PROBE_A: u32 = 99_810;
const PROBE_B: u32 = 99_811;
const PROBE_C: u32 = 99_812;
const PROBE_D: u32 = 99_813;

static GRANTED_SHOT: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget))
);
static AURA_GRANT: &[AbilityDef] = &[static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::GrantActivatedDefinition(&GRANTED_SHOT)
)];
static CLONE_ABILITIES: &[AbilityDef] = &[AbilityDef::CopyOnEnter {
    target: TargetSpec::Object(&Filter::CREATURE),
    mods: &[],
}];

static OUTGOING_PHASING_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::PHASES_OUT,
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }]
    ),
    triggered!(
        Trigger::ETB,
        &[Effect::LoseLife {
            amount: Amount::Fixed(10),
            target: PlayerRel::You,
        }]
    ),
    triggered!(
        Trigger::LeavesBattlefield(&Filter::This),
        &[Effect::LoseLife {
            amount: Amount::Fixed(10),
            target: PlayerRel::You,
        }]
    ),
];
static INCOMING_PHASING_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::PHASES_IN,
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]
    ),
    triggered!(
        Trigger::PhasesOut(&Filter::CREATURE),
        &[Effect::LoseLife {
            amount: Amount::Fixed(10),
            target: PlayerRel::You,
        }]
    ),
    triggered!(
        Trigger::ETB,
        &[Effect::LoseLife {
            amount: Amount::Fixed(10),
            target: PlayerRel::You,
        }]
    ),
];

static HOST_PHASE_OUT: AbilityDef = triggered!(
    Trigger::PHASES_OUT,
    &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }]
);
static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);
static CLOAK_AURA_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Spell {
        effects: &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    },
    static_ability!(
        ENCHANTED_CREATURE,
        Modifier::GrantTriggeredDefinition(&HOST_PHASE_OUT)
    ),
];
static PHASE_SPELL_FX: &[Effect] = &[Effect::PhaseOut {
    target: Some(TargetSpec::Object(&Filter::CREATURE)),
}];
static BLINK_FX: &[Effect] = &[Effect::Blink {
    target: TargetSpec::Object(&Filter::CREATURE),
    owner_control: true,
}];
static DESTROY_FX: &[Effect] = &[Effect::Destroy {
    target: TargetSpec::Object(&Filter::CREATURE),
    no_regen: true,
}];

fn spell_card(
    index: u32,
    name: &'static str,
    effects: &'static [Effect],
    targets: Option<TargetReq>,
) -> &'static CardDef {
    let abilities = Box::leak(Box::new([AbilityDef::Spell {
        effects,
        targets,
        second_targets: None,
        third_targets: None,
        condition: None,
    }]));
    card(
        index,
        face(name, "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        abilities,
    )
}

fn permanent_card(
    index: u32,
    name: &'static str,
    types: TypeSet,
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    card(index, face(name, "{0}", types), keywords, abilities)
}

fn cast_at_object(engine: &mut Bench, seat: PlayerId, card_idx: u32, target: ObjectId) {
    let hand_id = the(engine, ZoneLocation::Hand(seat), card_idx);
    engine
        .apply(seat, PlayerAction::CastSpell { card: hand_id })
        .expect("cast targeted spell");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("choose object target");
}

/// Conditional static self-grant (`Condition::GraveyardCountAtLeast(7)` +
/// `Modifier::GrantTriggeredDefinition` on `Trigger::ETB`) fires when seven or
/// more cards are in your graveyard as it enters and stays silent below seven.
#[test]
fn conditional_static_self_grants_triggered_definition_with_etb_check() {
    static THRESHOLD_ETB: AbilityDef = triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }]
    );
    static ABILITIES: &[AbilityDef] = &[AbilityDef::Static(baylee_cards_dsl::StaticAbility {
        layer: Layer::Ability,
        filter: Filter::This,
        modifier: Modifier::GrantTriggeredDefinition(&THRESHOLD_ETB),
        condition: Some(Condition::GraveyardCountAtLeast(7)),
        source_zone: baylee_cards_dsl::ZoneRef::Battlefield,
    })];

    let entrant = card(
        PROBE_A,
        creature_face("Threshold Entrant", "{0}", 2, 2),
        KeywordSet::EMPTY,
        ABILITIES,
    );
    let mut engine = bench(
        101,
        vec![entrant],
        [
            Seat::default().holding(&[PROBE_A, PROBE_A]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let first_hand = objects(&engine, ZoneLocation::Hand(me()), PROBE_A)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: first_hand })
        .expect("cast first entrant");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20, "no threshold means no ETB life");

    let gy_cards: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .iter()
        .take(7)
        .copied()
        .collect();
    for id in gy_cards {
        let _ = engine.state.move_object(
            id,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            Cause::Effect,
        );
    }
    engine.sync_static_effects();
    engine.state.refresh_characteristics();

    let second_hand = objects(&engine, ZoneLocation::Hand(me()), PROBE_A)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: second_hand })
        .expect("cast second entrant with threshold");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        24,
        "threshold static self-grant fires ETB trigger on entry"
    );
}

/// `Modifier::GrantTriggeredDefinition` enforces both checks of an
/// intervening-`if` clause (CR 603.4) and tracks `once_per_turn` independently
/// of the recipient's own printed ability 0 (`fire_key`).
#[test]
fn granted_triggered_definition_enforces_intervening_if_both_checks_and_once_per_turn() {
    static PRINTED_TAP_TRIGGER: AbilityDef = AbilityDef::Triggered {
        trigger: Trigger::BecomesTapped(&Filter::This),
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        zone: TriggerZone::Battlefield,
        once_per_turn: true,
        condition: None,
        resolves_if_targets_illegal: false,
    };
    static GRANTED_TAP_TRIGGER: AbilityDef = AbilityDef::Triggered {
        trigger: Trigger::BecomesTapped(&Filter::This),
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        zone: TriggerZone::Battlefield,
        once_per_turn: true,
        condition: Some(Condition::LifeAtMost(20)),
        resolves_if_targets_illegal: false,
    };
    static TAP_UNTAP_ABILITIES: &[AbilityDef] = &[
        PRINTED_TAP_TRIGGER,
        paid(Cost::TAP, &[Effect::UntapSelf], None),
    ];
    static GRANTOR_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::GrantTriggeredDefinition(&GRANTED_TAP_TRIGGER)
    )];
    static HEAL_BURST: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(5),
    }];

    let dork = card(
        PROBE_A,
        creature_face("Tap Probe", "{0}", 1, 2),
        KeywordSet::HASTE,
        TAP_UNTAP_ABILITIES,
    );
    let shrine = permanent_card(
        PROBE_B,
        "Low-Life Shrine",
        TypeSet::ENCHANTMENT,
        KeywordSet::EMPTY,
        GRANTOR_ABILITIES,
    );
    let heal = spell_card(PROBE_C, "Heal Burst", HEAL_BURST, None);

    let mut engine = bench(
        102,
        vec![dork, shrine, heal],
        [
            Seat::with(&[PROBE_A, PROBE_B]).holding(&[PROBE_C]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let dork_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);
    engine.state.players[me().get() as usize].life = 17;

    activate(&mut engine, me(), dork_id, 1);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20);

    activate(&mut engine, me(), dork_id, 1);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20, "once_per_turn blocks second fire");

    walk_until(&mut engine, |e| e.state().turn.active == them());
    to_main(&mut engine, me());
    activate(&mut engine, me(), dork_id, 1);
    let heal_id = the(&engine, ZoneLocation::Hand(me()), PROBE_C);
    engine
        .apply(me(), PlayerAction::CastSpell { card: heal_id })
        .expect("cast instant in response");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        26,
        "intervening-if false on resolution suppresses granted trigger"
    );

    walk_until(&mut engine, |e| e.state().turn.active == them());
    to_main(&mut engine, me());
    activate(&mut engine, me(), dork_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        27,
        "only printed +1 fires when intervening-if is false at trigger time"
    );
}

/// Temporary `Animate.Triggers`-style effects granting a `ModalTriggered`
/// definition offer modes and targets on attack and expire at cleanup.
#[test]
fn until_eot_animation_grants_modal_triggered_definition_and_expires_at_cleanup() {
    static MODAL_ATTACK: AbilityDef = AbilityDef::ModalTriggered {
        trigger: Trigger::Attacks(&Filter::This),
        restriction: baylee_cards_dsl::ModalRestriction::None,
        modes: &[
            SpellMode {
                effects: &[Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::AnyTarget,
                }],
                targets: Some(TargetReq::one(TargetSpec::AnyTarget)),
                second_targets: None,
                additional_cost: None,
                cost_override: None,
            },
            SpellMode {
                effects: &[Effect::GainLife {
                    amount: Amount::Fixed(3),
                }],
                targets: None,
                second_targets: None,
                additional_cost: None,
                cost_override: None,
            },
        ],
        zone: TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    };
    static ANIMATE_EFFECTS: &[Effect] = &[
        Effect::ModifySource {
            modifier: Modifier::AddType(TypeSet::CREATURE),
            duration: Duration::UntilEndOfTurn,
        },
        Effect::ModifySource {
            modifier: Modifier::SetPT(3, 3),
            duration: Duration::UntilEndOfTurn,
        },
        Effect::ModifySource {
            modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            duration: Duration::UntilEndOfTurn,
        },
        Effect::ModifySource {
            modifier: Modifier::GrantTriggeredDefinition(&MODAL_ATTACK),
            duration: Duration::UntilEndOfTurn,
        },
    ];
    static RELIC_ABILITIES: &[AbilityDef] = &[free(ANIMATE_EFFECTS, None)];

    let relic = permanent_card(
        PROBE_A,
        "Awakened Relic",
        TypeSet::ARTIFACT,
        KeywordSet::EMPTY,
        RELIC_ABILITIES,
    );
    let mut engine = bench(103, vec![relic], [Seat::with(&[PROBE_A]), Seat::default()]);
    to_main(&mut engine, me());
    let relic_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);

    activate(&mut engine, me(), relic_id, 0);
    settle(&mut engine, me());

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(relic_id, Defender::Player(them()))],
            },
        )
        .expect("animated relic attacks");

    assert!(matches!(engine.pending(), Pending::ChooseCastMode { .. }));
    engine
        .apply(me(), PlayerAction::ChooseMode(0))
        .expect("choose mode 0 (2 damage to any target)");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent");

    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 15);
    let chars = engine
        .state()
        .object(relic_id)
        .expect("relic stays on battlefield")
        .characteristics();
    assert!(
        !chars.types.contains(TypeSet::CREATURE),
        "animation expired at cleanup"
    );
}

/// `Modifier::GrantActivatedDefinition` offers targeted activated abilities on
/// matching permanents, while a Layer-1 `CopyOnEnter` clone does NOT copy the
/// Layer-6 granted ability (CR 707.2).
#[test]
fn granted_activated_definition_targets_and_is_not_copied_by_clone() {
    let archer = card(
        PROBE_A,
        creature_face("Training Archer", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let armory = permanent_card(
        PROBE_B,
        "Bowyer Armory",
        TypeSet::ENCHANTMENT,
        KeywordSet::EMPTY,
        AURA_GRANT,
    );
    let clone = card(
        PROBE_C,
        creature_face("Mirrorform", "{0}", 0, 0),
        KeywordSet::EMPTY,
        CLONE_ABILITIES,
    );

    let mut engine = bench(
        104,
        vec![archer, armory, clone],
        [
            Seat::with(&[PROBE_A, PROBE_B]),
            Seat::default().holding(&[PROBE_C]),
        ],
    );
    to_main(&mut engine, me());
    let archer_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);

    let (_, legal) = offer(&engine);
    let &(_, slot) = legal
        .abilities
        .iter()
        .find(|&&(src, _)| src == archer_id)
        .expect("granted activated ability is offered");
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: archer_id,
                ability_index: slot,
            },
        )
        .expect("activate granted shot");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("aim granted shot at opponent");
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 18);

    to_main(&mut engine, them());
    let clone_hand = the(&engine, ZoneLocation::Hand(them()), PROBE_C);
    engine
        .apply(them(), PlayerAction::CastSpell { card: clone_hand })
        .expect("cast clone");
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo { player, .. } | Pending::ChooseTargets { player, .. }
                if *player == them()
        )
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(them(), PlayerAction::YesNo(true))
            .expect("accept copy on enter");
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                them(),
                PlayerAction::ChooseObjects {
                    objects: vec![archer_id],
                },
            )
            .expect("copy archer");
    }
    settle(&mut engine, them());

    let clone_bf = the(&engine, ZoneLocation::Battlefield, PROBE_C);
    let (_, opp_legal) = offer(&engine);
    assert!(
        !opp_legal.abilities.iter().any(|&(src, _)| src == clone_bf),
        "clone must not inherit a Layer-6 granted ability (CR 707.2)"
    );
}

/// CR 613.8 dependency in Layer 6: an earlier `"Creatures with flying lose all
/// abilities"` waits on a later `"Creatures have flying"`, wiping an
/// intermediate grant (`t = 2`) and flying (`t = 3`) while preserving a grant
/// with a timestamp later than the depended-on flying effect (`t = 4`).
#[test]
fn layer_six_dependency_wipes_earlier_grants_and_preserves_later_grants_cr_613_8() {
    static FLYING_CREATURE: Filter =
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);
    static EARLY_TAP_GAIN: AbilityDef = triggered!(
        Trigger::BecomesTapped(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }]
    );
    static LATE_TAP_GAIN: AbilityDef = triggered!(
        Trigger::BecomesTapped(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]
    );

    let bear = card(
        PROBE_A,
        creature_face("Subject Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(105, vec![bear], [Seat::with(&[PROBE_A]), Seat::default()]);
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);

    for (filter, modifier) in [
        (&FLYING_CREATURE, Modifier::LoseAllAbilities),
        (
            &Filter::CREATURE,
            Modifier::GrantTriggeredDefinition(&EARLY_TAP_GAIN),
        ),
        (&Filter::CREATURE, Modifier::AddKeyword(KeywordSet::FLYING)),
        (
            &Filter::CREATURE,
            Modifier::GrantTriggeredDefinition(&LATE_TAP_GAIN),
        ),
    ] {
        let ts = engine.state.next_timestamp();
        engine.state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: me(),
            origin: EffectOrigin::Static,
            layer: Layer::Ability,
            timestamp: ts,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(filter),
            modifier,
        });
    }
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();

    assert!(
        !engine
            .state()
            .object(bear_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "dependent LoseAllAbilities wipes the flying it waited on"
    );

    let from = engine.state.journal.last_seq();
    engine.state.set_tapped(bear_id, true);
    engine
        .state
        .journal
        .record(crate::event::GameEvent::ObjectTapped {
            object: bear_id,
            cause: Cause::Effect,
        });
    let found = crate::trigger::collect(&engine.state, &engine.lookup, from);
    assert_eq!(
        found.len(),
        1,
        "only the post-dependency grant (t=4) survives; t=2 was wiped"
    );
    engine.trigger_queue.extend(found);
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    engine.collect_triggers();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23, "post-dependency +3 life resolved");
}

/// Simultaneous untap-step Phasing (CR 502.1 + CR 603.10b + CR 702.26d):
/// outgoing fires `Trigger::PHASES_OUT` via look-back, incoming fires
/// `Trigger::PHASES_IN` without observing the simultaneous phase-out, and
/// neither fires ETB or LTB triggers.
#[test]
fn simultaneous_untap_phasing_fires_phase_out_lookback_and_phase_in_without_cross_talk() {
    let outgoing = card(
        PROBE_A,
        creature_face("Shimmer Outgoing", "{0}", 2, 2),
        KeywordSet::PHASING,
        OUTGOING_PHASING_ABILITIES,
    );
    let incoming = card(
        PROBE_B,
        creature_face("Wurm Incoming", "{0}", 4, 4),
        KeywordSet::PHASING,
        INCOMING_PHASING_ABILITIES,
    );

    let mut engine = bench(
        106,
        vec![outgoing, incoming],
        [
            Seat::default().holding(&[PROBE_A, PROBE_B]).at(40),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    for idx in [PROBE_A, PROBE_B] {
        let hand_id = the(&engine, ZoneLocation::Hand(me()), idx);
        engine
            .apply(me(), PlayerAction::CastSpell { card: hand_id })
            .expect("cast phasing creature");
        settle(&mut engine, me());
    }
    assert_eq!(life(&engine, me()), 20);
    let outgoing_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);
    let incoming_id = the(&engine, ZoneLocation::Battlefield, PROBE_B);

    engine.state.phase_out(&[incoming_id]);
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    engine.trigger_scan_seq = engine.state.journal.last_seq();

    walk_until(&mut engine, |e| e.state().turn.active == them());
    to_main(&mut engine, me());

    assert!(
        engine
            .state()
            .object(outgoing_id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "outgoing permanent phased out in untap step"
    );
    assert!(
        !engine
            .state()
            .object(incoming_id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "incoming permanent phased in simultaneously"
    );
    assert_eq!(
        life(&engine, me()),
        25,
        "+2 from outgoing PHASES_OUT +3 from incoming PHASES_IN, 0 ETB/LTB/cross-talk"
    );
}

/// Indirect attachment phasing (CR 702.26d, CR 702.26g, CR 603.10b):
/// direct `Effect::PhaseOut` on a host phases its Aura out indirectly, fires
/// an Aura-granted `Trigger::PHASES_OUT` on the host via look-back, and
/// preserves `+1/+1` counters and `attached_to` across phase-in.
#[test]
fn indirect_attachment_phasing_preserves_counters_and_fires_aura_granted_phase_out_trigger() {
    let mut aura_face = face("Cloak of Mist", "{0}", TypeSet::ENCHANTMENT);
    aura_face.subtypes = Box::leak(Box::new([enchantment::AURA]));
    let host = card(
        PROBE_A,
        creature_face("Cloaked Host", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let aura = card(PROBE_B, aura_face, KeywordSet::EMPTY, CLOAK_AURA_ABILITIES);
    let ripple = spell_card(
        PROBE_C,
        "Phase Ripple",
        PHASE_SPELL_FX,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    );

    let mut engine = bench(
        107,
        vec![host, aura, ripple],
        [
            Seat::with(&[PROBE_A]).holding(&[PROBE_B, PROBE_C]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let host_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);
    crate::replacement::put_counters(
        &mut engine.state,
        host_id,
        baylee_cards_dsl::CounterKind::P1P1,
        2,
    );
    engine.state.refresh_characteristics();

    let aura_id = the(&engine, ZoneLocation::Hand(me()), PROBE_B);
    cast_at_object(&mut engine, me(), PROBE_B, host_id);
    settle(&mut engine, me());

    cast_at_object(&mut engine, me(), PROBE_C, host_id);
    settle(&mut engine, me());

    assert_eq!(
        life(&engine, me()),
        22,
        "Aura-granted PHASES_OUT trigger fired via CR 603.10b look-back"
    );
    let aura_obj = engine.state().object(aura_id).unwrap();
    assert!(
        aura_obj.status.contains(Status::PHASED_OUT)
            && aura_obj.status.contains(Status::PHASED_OUT_INDIRECTLY),
        "Aura phased out indirectly with host"
    );
    assert_eq!(aura_obj.attached_to, Some(host_id));

    walk_until(&mut engine, |e| e.state().turn.active == them());
    to_main(&mut engine, me());
    let host_obj = engine.state().object(host_id).unwrap();
    let aura_obj = engine.state().object(aura_id).unwrap();
    assert!(!host_obj.status.contains(Status::PHASED_OUT));
    assert!(!aura_obj.status.contains(Status::PHASED_OUT));
    assert_eq!(
        host_obj.counters.get(baylee_cards_dsl::CounterKind::P1P1),
        2
    );
    assert_eq!(aura_obj.attached_to, Some(host_id));
}

/// Delayed `AtEndOfCombat` `PhaseOut { target: Some(TargetSpec::EventObject) }`
/// (*Teferi's Veil*) phases out the exact attacking incarnation at end of
/// combat even if the source leaves, while ignoring a blinked new incarnation (CR 400.7).
#[test]
fn delayed_end_of_combat_phase_out_tracks_exact_incarnation() {
    static VEIL_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::YOUR_CREATURE),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::PhaseOut {
                target: Some(TargetSpec::EventObject),
            }],
        }]
    )];

    let veil = permanent_card(
        PROBE_A,
        "Veil of Combat",
        TypeSet::ENCHANTMENT,
        KeywordSet::EMPTY,
        VEIL_ABILITIES,
    );
    let attacker_one = card(
        PROBE_B,
        creature_face("Steady Attacker", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let attacker_two = card(
        PROBE_C,
        creature_face("Blinked Runner", "{0}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );
    let blink = spell_card(
        PROBE_D,
        "Quick Blink",
        BLINK_FX,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    );

    let mut engine = bench(
        108,
        vec![veil, attacker_one, attacker_two, blink],
        [
            Seat::with(&[PROBE_A, PROBE_B, PROBE_C]).holding(&[PROBE_D]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let veil_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);
    let steady_id = the(&engine, ZoneLocation::Battlefield, PROBE_B);
    let runner_id = the(&engine, ZoneLocation::Battlefield, PROBE_C);

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (steady_id, Defender::Player(them())),
                    (runner_id, Defender::Player(them())),
                ],
            },
        )
        .expect("declare attackers");
    settle(&mut engine, me());

    let _ = engine.state.move_object(
        veil_id,
        ZoneLocation::Graveyard(me()),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    cast_at_object(&mut engine, me(), PROBE_D, runner_id);
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });

    assert!(
        engine
            .state()
            .object(steady_id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "steady attacker phased out via delayed EventObject trigger"
    );
    assert!(
        !engine
            .state()
            .object(runner_id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "blinked incarnation must not phase out from old delayed trigger (CR 400.7)"
    );
}

/// `Modifier::CantPhaseOut` + `Trigger::PhasesOutOrLeavesBattlefield`
/// (*Ertai's Familiar*): prevents phasing out until next upkeep, while firing
/// the trigger on both phase-out and battlefield departure.
#[test]
fn cant_phase_out_blocks_untap_phasing_and_phases_out_or_leaves_fires_on_both() {
    static FAMILIAR_ABILITIES: &[AbilityDef] = &[
        triggered!(
            Trigger::PhasesOutOrLeavesBattlefield(&Filter::This),
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }]
        ),
        free(
            &[Effect::ModifySource {
                modifier: Modifier::CantPhaseOut,
                duration: Duration::UntilEndOfYourNextUpkeep,
            }],
            None,
        ),
    ];

    let familiar = card(
        PROBE_A,
        creature_face("Anchored Familiar", "{0}", 2, 2),
        KeywordSet::PHASING,
        FAMILIAR_ABILITIES,
    );
    let ripple = spell_card(
        PROBE_B,
        "Phase Ripple",
        PHASE_SPELL_FX,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    );
    let kill = spell_card(
        PROBE_C,
        "Hard Terminate",
        DESTROY_FX,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    );

    let mut engine = bench(
        109,
        vec![familiar, ripple, kill],
        [
            Seat::default().holding(&[PROBE_A, PROBE_B, PROBE_C]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let fam_hand = the(&engine, ZoneLocation::Hand(me()), PROBE_A);
    engine
        .apply(me(), PlayerAction::CastSpell { card: fam_hand })
        .expect("cast familiar");
    settle(&mut engine, me());
    let familiar_id = the(&engine, ZoneLocation::Battlefield, PROBE_A);

    // Phase out via Phase Ripple: PhasesOutOrLeavesBattlefield mills 3 (+ Phase Ripple = 4).
    cast_at_object(&mut engine, me(), PROBE_B, familiar_id);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .len(),
        4
    );

    // Next turn it phases back in; activate CantPhaseOut, then verify next untap step skips phasing!
    walk_until(&mut engine, |e| e.state().turn.active == them());
    to_main(&mut engine, me());
    activate(&mut engine, me(), familiar_id, 1);
    settle(&mut engine, me());

    walk_until(&mut engine, |e| e.state().turn.active == them());
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Upkeep)
    });
    assert!(
        !engine
            .state()
            .object(familiar_id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "CantPhaseOut prevented untap-step phasing"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .len(),
        4
    );

    // Destroy familiar_id: PhasesOutOrLeavesBattlefield fires on LTB (+3 milled + kill + familiar = 9).
    to_main(&mut engine, me());
    cast_at_object(&mut engine, me(), PROBE_C, familiar_id);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .len(),
        9
    );
}
