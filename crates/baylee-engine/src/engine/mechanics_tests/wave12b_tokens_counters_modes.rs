//! Beat 18 / Engine Wave 12b deterministic rule tests:
//! 1. `Effect::CreateTokenSized` (`CR 111.3`, `CR 707.2`)
//! 2. `Effect::PutCountersDivided` (`CR 601.2d`, `CR 602.2b`, `CR 603.3d`, `CR 608.2b`, `CR 707.10`)
//! 3. `AbilityDef::Modal` with `can_repeat_modes: true` (`CR 700.2d`, `CR 608.2b–c`, `CR 707.10`)

use super::*;
use crate::choice::{CastModeKind, NumberPrompt};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::prelude::{cost, loyalty, spell, triggered};
use baylee_cards_dsl::{
    Amount, CopyCleanup, CounterKind, Filter, ModeCount, PlayerRel, ReplacementRule, SpellMode,
    SupertypeSet, TargetReq, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::ids::TargetRef;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_860;
const C1: u32 = 99_861;
const C2: u32 = 99_862;
const C3: u32 = 99_863;

static WURM_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[creature::WURM];
static PHYREXIAN_HORROR_SUBTYPES: &[baylee_core::ids::SubtypeId] =
    &[creature::PHYREXIAN, creature::HORROR];

static X_WURM_TOKEN: TokenDef = TokenDef {
    name: "Wurm",
    colors: ColorSet::of(Color::Green),
    types: TypeSet::CREATURE,
    subtypes: WURM_SUBTYPES,
    keywords: KeywordSet::TRAMPLE,
    ..TokenDef::DEFAULT
};

static FORGE_HORROR_TOKEN: TokenDef = TokenDef {
    name: "Phyrexian Horror",
    colors: ColorSet::of(Color::Red),
    types: TypeSet::CREATURE,
    subtypes: PHYREXIAN_HORROR_SUBTYPES,
    toughness: Some(1),
    keywords: KeywordSet::TRAMPLE,
    ..TokenDef::DEFAULT
};

static CONFLUENCE_MODES: &[SpellMode] = &[
    SpellMode {
        effects: &[Effect::DealDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(2),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        cost_override: None,
        additional_cost: None,
    },
    SpellMode {
        effects: &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        cost_override: None,
        additional_cost: None,
    },
    SpellMode {
        effects: &[Effect::draw(1)],
        targets: None,
        second_targets: None,
        cost_override: None,
        additional_cost: None,
    },
];

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player: p, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player: p,
            legal: Box::new(engine.compute_legal(p)),
        };
    }
}

fn counters(engine: &Bench, id: ObjectId, kind: CounterKind) -> u16 {
    engine.state().object(id).unwrap().counters.get(kind)
}

fn aim_objects(engine: &mut Bench, seat: PlayerId, objs: &[ObjectId]) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: objs.to_vec(),
                players: vec![],
            },
        )
        .unwrap();
}

fn keep_targets_on_copy(engine: &mut Bench, seat: PlayerId) {
    walk_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    while matches!(
        engine.pending(),
        Pending::ChooseTargets {
            reason: crate::choice::TargetPrompt::Retarget { .. },
            ..
        }
    ) {
        aim_objects(engine, seat, &[]);
    }
}

fn sized_tokens(engine: &Bench, controller: PlayerId, p: i16, t: i16) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                let ch = o.characteristics();
                o.token.is_some()
                    && o.controller == controller
                    && ch.power == Some(p)
                    && ch.toughness == Some(t)
            })
        })
        .collect()
}

#[test]
fn create_token_sized_stamps_copiable_pt_for_populate_and_token_doubling() {
    static HYDRA_CALL: &[AbilityDef] = &[spell!(&[Effect::CreateTokenSized {
        token: &X_WURM_TOKEN,
        amount: Amount::Fixed(1),
        power: Amount::X,
        toughness: Amount::X,
        who: PlayerRel::You,
        tapped: false,
        attacking: false,
        haste_until_eot: false,
        cleanup: CopyCleanup::None,
    }])];
    static POPULATE_RING: &[AbilityDef] = &[free(&[Effect::Populate], None)];
    static DOUBLER_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleTokenCreation {
            controller_filter: &Filter::ControlledByYou,
        },
    )];

    let hydra_call = card(
        C0,
        face("Brood Call", "{X}{G}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        HYDRA_CALL,
    );
    let populate_ring = card(
        C1,
        face("Populate Ring", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        POPULATE_RING,
    );
    let doubler = card(
        C2,
        face("Parallel Grove", "{1}{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        DOUBLER_AB,
    );

    let mut engine = bench(
        18201,
        vec![hydra_call, populate_ring, doubler],
        [Seat::with(&[C1]).holding(&[C0, C2, C0]), Seat::default()],
    );
    to_main(&mut engine, me());

    let hand_spells = objects(&engine, ZoneLocation::Hand(me()), C0);
    add_pool(&mut engine, me(), ManaColor::Green, 6);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: hand_spells[0],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::ChooseNumber(5)).unwrap();
    settle(&mut engine, me());

    let first_wurm = sized_tokens(&engine, me(), 5, 5)[0];
    let ring = the(&engine, ZoneLocation::Battlefield, C1);
    activate(&mut engine, me(), ring, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![first_wurm],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let five_wurms = sized_tokens(&engine, me(), 5, 5);
    assert_eq!(five_wurms.len(), 2);
    for id in five_wurms {
        let wurm = engine.state().object(id).unwrap();
        assert_eq!((wurm.base.power, wurm.base.toughness), (Some(5), Some(5)));
    }

    let grove = the(&engine, ZoneLocation::Hand(me()), C2);
    add_pool(&mut engine, me(), ManaColor::Green, 6);
    engine
        .apply(me(), PlayerAction::CastSpell { card: grove })
        .unwrap();
    settle(&mut engine, me());
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: hand_spells[1],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    settle(&mut engine, me());

    assert_eq!(sized_tokens(&engine, me(), 3, 3).len(), 2);
}

#[test]
fn create_token_sized_asymmetric_forge_cleanup_and_chosen_player_tokens() {
    static FORGE_AB: &[AbilityDef] = &[
        free(
            &[
                Effect::AddCounterSource {
                    kind: CounterKind::Charge,
                    amount: Amount::Fixed(4),
                },
                Effect::CreateTokenSized {
                    token: &FORGE_HORROR_TOKEN,
                    amount: Amount::Fixed(1),
                    power: Amount::CountersOnSource(CounterKind::Charge),
                    toughness: Amount::Fixed(1),
                    who: PlayerRel::You,
                    tapped: true,
                    attacking: false,
                    haste_until_eot: true,
                    cleanup: CopyCleanup::SacrificeAtNextEndStep,
                },
            ],
            None,
        ),
        free(
            &[Effect::CreateTokenSized {
                token: &X_WURM_TOKEN,
                amount: Amount::Fixed(2),
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(2),
                who: PlayerRel::Chosen,
                tapped: false,
                attacking: false,
                haste_until_eot: false,
                cleanup: CopyCleanup::None,
            }],
            Some(TargetReq::one(TargetSpec::AnyPlayer)),
        ),
    ];

    let forge = card(
        C0,
        face("Synthetic Forge", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FORGE_AB,
    );
    let mut engine = bench(18202, vec![forge], [Seat::with(&[C0]), Seat::default()]);
    to_main(&mut engine, me());

    let forge_id = the(&engine, ZoneLocation::Battlefield, C0);
    activate(&mut engine, me(), forge_id, 0);
    settle(&mut engine, me());

    let horrors = sized_tokens(&engine, me(), 4, 1);
    assert_eq!(horrors.len(), 1);
    let horror = engine.state().object(horrors[0]).unwrap();
    assert!(
        horror.status.contains(crate::object::Status::TAPPED)
            && horror
                .characteristics()
                .keywords
                .contains(KeywordSet::HASTE)
    );

    activate(&mut engine, me(), forge_id, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(sized_tokens(&engine, them(), 3, 2).len(), 2);

    to_main(&mut engine, them());
    assert!(
        engine
            .state()
            .object(horrors[0])
            .is_none_or(|o| o.zone != Zone::Battlefield)
    );
}

#[test]
fn put_counters_divided_spell_copy_and_partial_illegal_target_no_redistribution() {
    static BLESSING_AB: &[AbilityDef] = &[spell!(
        &[Effect::PutCountersDivided {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
        targets = Some(TargetReq {
            min: 1,
            ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 3)
        }),
    )];
    static FORK_KILL_AB: &[AbilityDef] = &[
        free(
            &[Effect::CopyTargetSpell { mods: &[] }],
            Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        ),
        free(
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    let blessing = card(
        C0,
        face("Divided Blessing", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BLESSING_AB,
    );
    let rig = card(
        C1,
        face("Copy and Kill Rig", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FORK_KILL_AB,
    );
    let bear = card(
        C2,
        creature_face("Target Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18203,
        vec![blessing, rig, bear],
        [Seat::with(&[C1, C2, C2]).holding(&[C0]), Seat::default()],
    );
    to_main(&mut engine, me());

    let bears = objects(&engine, ZoneLocation::Battlefield, C2);
    let (bear_a, bear_b) = (bears[0], bears[1]);
    let spell_card = the(&engine, ZoneLocation::Hand(me()), C0);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C1);

    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_card })
        .unwrap();
    aim_objects(&mut engine, me(), &[bear_a, bear_b]);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 1,
            max: 2,
            reason: NumberPrompt::DivideCounters {
                kind: CounterKind::P1P1,
                target: TargetRef::Object(r),
                index: 0,
                of: 2,
                left: 3,
            },
            ..
        } if r.object == bear_a
    ));
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();

    let stack_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    activate(&mut engine, me(), rig_id, 0);
    aim_objects(&mut engine, me(), &[stack_spell]);
    keep_targets_on_copy(&mut engine, me());
    walk_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(counters(&engine, bear_a, CounterKind::P1P1), 2);
    assert_eq!(counters(&engine, bear_b, CounterKind::P1P1), 1);

    activate(&mut engine, me(), rig_id, 1);
    aim_objects(&mut engine, me(), &[bear_a]);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(bear_b).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(counters(&engine, bear_b, CounterKind::P1P1), 2);
}

#[test]
fn put_counters_divided_dies_lki_trigger_preserves_source_counters() {
    static BEASTCALLER_AB: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PutCountersDivided {
            kind: CounterKind::P1P1,
            amount: Amount::CountersOnSource(CounterKind::P1P1),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::YOUR_CREATURE),
            3,
        )),
    )];
    static PUMP_SAC_RIG: &[AbilityDef] = &[
        free(
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(3),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    let beastcaller = card(
        C0,
        creature_face("Beastcaller", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        BEASTCALLER_AB,
    );
    let rig = card(
        C1,
        face("Setup Rig", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PUMP_SAC_RIG,
    );
    let bear = card(
        C2,
        creature_face("Ally Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18204,
        vec![beastcaller, rig, bear],
        [Seat::with(&[C0, C1, C2, C2]), Seat::default()],
    );
    to_main(&mut engine, me());

    let caller_id = the(&engine, ZoneLocation::Battlefield, C0);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C1);
    let bears = objects(&engine, ZoneLocation::Battlefield, C2);
    let (bear_a, bear_b) = (bears[0], bears[1]);

    activate(&mut engine, me(), rig_id, 0);
    aim_objects(&mut engine, me(), &[caller_id]);
    settle(&mut engine, me());

    activate(&mut engine, me(), rig_id, 1);
    aim_objects(&mut engine, me(), &[caller_id]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_objects(&mut engine, me(), &[bear_a, bear_b]);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 1,
            max: 2,
            reason: NumberPrompt::DivideCounters { left: 3, .. },
            ..
        }
    ));
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());

    assert_eq!(counters(&engine, bear_a, CounterKind::P1P1), 2);
    assert_eq!(counters(&engine, bear_b, CounterKind::P1P1), 1);
}

#[test]
fn put_counters_divided_sacrifice_activation_and_loyalty_ability() {
    static STRONGHOLD_AB: &[AbilityDef] = &[paid(
        cost!(SacrificeSelf),
        &[Effect::PutCountersDivided {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
        Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::YOUR_CREATURE),
            2,
        )),
    )];
    static MENTOR_AB: &[AbilityDef] = &[loyalty!(
        1,
        &[Effect::PutCountersDivided {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::YOUR_CREATURE),
            3,
        )),
    )];

    let stronghold = card(
        C0,
        face("Stronghold", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STRONGHOLD_AB,
    );
    let mentor = card(
        C1,
        FaceDef {
            loyalty: Some(4),
            supertypes: SupertypeSet::LEGENDARY,
            ..face("Mentor Walker", "{3}{G}{W}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        MENTOR_AB,
    );
    let bear = card(
        C2,
        creature_face("Ally Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18205,
        vec![stronghold, mentor, bear],
        [Seat::with(&[C0, C1, C2, C2]), Seat::default()],
    );
    to_main(&mut engine, me());

    let stronghold_id = the(&engine, ZoneLocation::Battlefield, C0);
    let mentor_id = the(&engine, ZoneLocation::Battlefield, C1);
    let bears = objects(&engine, ZoneLocation::Battlefield, C2);
    let (bear_a, bear_b) = (bears[0], bears[1]);

    activate(&mut engine, me(), stronghold_id, 0);
    aim_objects(&mut engine, me(), &[bear_a, bear_b]);
    engine.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());
    assert_eq!(counters(&engine, bear_a, CounterKind::P1P1), 1);
    assert_eq!(counters(&engine, bear_b, CounterKind::P1P1), 1);

    activate(&mut engine, me(), mentor_id, 0);
    aim_objects(&mut engine, me(), &[bear_a, bear_b]);
    engine.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());
    assert_eq!(counters(&engine, mentor_id, CounterKind::Loyalty), 5);
    assert_eq!(counters(&engine, bear_a, CounterKind::P1P1), 2);
    assert_eq!(counters(&engine, bear_b, CounterKind::P1P1), 3);
}

#[test]
fn modal_can_repeat_modes_triple_untargeted_and_distinct_non_repeating_command() {
    static CONFLUENCE_AB: &[AbilityDef] = &[AbilityDef::Modal {
        modes: CONFLUENCE_MODES,
        choose: ModeCount::THREE,
        can_repeat_modes: true,
    }];
    static DISTINCT_COMMAND_AB: &[AbilityDef] = &[AbilityDef::Modal {
        modes: CONFLUENCE_MODES,
        choose: ModeCount::TWO,
        can_repeat_modes: false,
    }];

    let confluence = card(
        C0,
        face("Synthetic Confluence", "{3}{U}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CONFLUENCE_AB,
    );
    let command = card(
        C1,
        face("Distinct Command", "{2}{U}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DISTINCT_COMMAND_AB,
    );
    let bear = card(
        C2,
        creature_face("Board Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18206,
        vec![confluence, command, bear],
        [Seat::with(&[C2]).holding(&[C0, C1]), Seat::default()],
    );
    to_main(&mut engine, me());

    let conf_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let cmd_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();

    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: conf_id })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let triple_draw_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::RepeatedModes(3 << 8))
        .expect("3x Mode 2 offered");
    engine
        .apply(me(), PlayerAction::ChooseMode(triple_draw_idx))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1 + 3
    );

    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cmd_id })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    assert!(
        options
            .iter()
            .all(|o| matches!(o.kind, CastModeKind::Modes(_)))
    );
}

#[test]
fn modal_can_repeat_modes_targets_same_creature_twice_and_copies_on_stack() {
    static CONFLUENCE_AB: &[AbilityDef] = &[AbilityDef::Modal {
        modes: CONFLUENCE_MODES,
        choose: ModeCount::THREE,
        can_repeat_modes: true,
    }];
    static COPY_RIG_AB: &[AbilityDef] = &[free(
        &[Effect::CopyTargetSpell { mods: &[] }],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];

    let confluence = card(
        C0,
        face("Synthetic Confluence", "{3}{U}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CONFLUENCE_AB,
    );
    let copy_rig = card(
        C1,
        face("Spell Copier", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        COPY_RIG_AB,
    );
    let ally = card(
        C2,
        creature_face("Ally Sprite", "{U}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let ogre = card(
        C3,
        creature_face("Enemy Ogre", "{3}{R}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18207,
        vec![confluence, copy_rig, ally, ogre],
        [Seat::with(&[C1, C2]).holding(&[C0]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());

    let conf_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C1);
    let ally_id = the(&engine, ZoneLocation::Battlefield, C2);
    let ogre_id = the(&engine, ZoneLocation::Battlefield, C3);

    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: conf_id })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let mixed_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::RepeatedModes(2 | (1 << 4)))
        .expect("2x Mode 0 + 1x Mode 1 offered");
    engine
        .apply(me(), PlayerAction::ChooseMode(mixed_idx))
        .unwrap();
    for target in [ogre_id, ogre_id, ally_id] {
        aim_objects(&mut engine, me(), &[target]);
    }
    let stack_conf = engine.state().zones.list(ZoneLocation::Stack)[0];
    activate(&mut engine, me(), rig_id, 0);
    aim_objects(&mut engine, me(), &[stack_conf]);
    keep_targets_on_copy(&mut engine, me());
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(ogre_id).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(counters(&engine, ally_id, CounterKind::P1P1), 2);
}
