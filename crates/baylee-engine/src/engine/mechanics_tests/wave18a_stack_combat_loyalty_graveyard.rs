//! Wave 18 Lane A deterministic gameplay tests:
//! 1. `AffectedZone$ Stack` static grants (`GrantAffinity`, `GrantCascade`, `GrantBlitzCardManaCost`,
//!    `FirstMatchingSpellThisTurn`, `Filter::CastForBlitz` commander reduction).
//! 2. `CantAttackYouOrYourPlaneswalkers`, `CantBeAttackedBy`, `Filter::AttackingYouOrYourPlaneswalker`,
//!    and `Trigger::AttackersDeclaredAgainst`.
//! 3. Granted loyalty abilities (`GrantActivatedDefinition`), `Trigger::LoyaltyAbilityActivated`,
//!    and `Modifier::MayActivateLoyaltyAsInstant`.
//! 4. `TgtZone$ Graveyard` mechanics: `GrantsHarmonizeManaCost`, `MayPayEnergyEqualToTargetCmcThen`,
//!    `CastCostIncreaseUnlessTargets` (*Mavinda*), `GrantActivatedAbilitiesWhenTargetCast`
//!    (*Havengul Lich*), `SharesCardTypeWithReference` (*Spirit-Sister's Call*), and
//!    `TriggerRemember::TargetAndSacrificed` + `ReturnDelayedObjectsToBattlefield` (*Rescue*).

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{
    activated, loyalty, mana_ability, spell, static_ability, triggered,
};
use baylee_cards_dsl::{
    Amount, AttackedTargetKind, Cost, CostPart, DelayedStep, DelayedTriggerWhen, Duration, Filter,
    Modifier, PlayerRel, StepKind, TargetReq, TargetSpec, Trigger, TriggerRemember, ZoneRef,
};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const W0: u32 = 99_910;
const W1: u32 = 99_911;
const W2: u32 = 99_912;
const W3: u32 = 99_913;
const W4: u32 = 99_914;
const W5: u32 = 99_915;

fn refresh(engine: &mut Bench) {
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh(engine);
}

fn move_to_graveyard(engine: &mut Bench, owner: PlayerId, id: ObjectId) {
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("moved to graveyard");
    refresh(engine);
}

#[test]
fn wave18a_stack_grants_first_spell_affinity_and_henzie_blitz() {
    static YOUR_SPELL_IN_STACK: Filter =
        Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Stack)]);
    static FIRST_NONCREATURE: Filter =
        Filter::And(&[Filter::Not(&Filter::CREATURE), Filter::ControlledByYou]);
    static STACK_FIRST_CASCADE: Filter = Filter::And(&[
        Filter::ControlledByYou,
        Filter::FirstMatchingSpellThisTurn(&FIRST_NONCREATURE),
        Filter::InZone(ZoneRef::Stack),
    ]);
    static BIG_CREATURE_STACK: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::CmcAtLeast(4),
        Filter::InZone(ZoneRef::Stack),
    ]);
    static STACK_GRANTER_AB: &[AbilityDef] = &[
        static_ability!(
            YOUR_SPELL_IN_STACK,
            Modifier::GrantAffinity(&Filter::ARTIFACT)
        ),
        static_ability!(STACK_FIRST_CASCADE, Modifier::GrantCascade(1)),
        static_ability!(BIG_CREATURE_STACK, Modifier::GrantBlitzCardManaCost),
        static_ability!(
            Filter::And(&[Filter::CastForBlitz, Filter::ControlledByYou]),
            Modifier::SpellsCostLessByAmount(Amount::CommanderCasts(PlayerRel::You))
        ),
    ];
    static PING_2_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(2),
    }])];
    static ARTIFACT_AB: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

    let granter = card(
        W0,
        creature_face("Stack Granter", "{3}", 3, 3),
        KeywordSet::EMPTY,
        STACK_GRANTER_AB,
    );
    let spell2 = card(
        W1,
        face("Two Cost Instant", "{2}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PING_2_AB,
    );
    let art = card(
        W2,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ARTIFACT_AB,
    );
    let big = card(
        W3,
        creature_face("Big Brute", "{3}{R}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18_001,
        vec![granter, spell2, art, big],
        [
            Seat::with(&[W0, W2, W2]).holding(&[W1, W1, W3]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    // Commander cast count = 2 -> Blitz costs {2} less, plus Affinity for 2 artifacts = {0} more!
    engine.state.commander_casts[usize::from(me().get())] = 2;
    refresh(&mut engine);

    let big_hand = the(&engine, ZoneLocation::Hand(me()), W3);
    let spell_hand = objects(&engine, ZoneLocation::Hand(me()), W1)[0];
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&spell_hand),
        "Affinity for 2 artifacts discounts {{2}} instant to {{0}}"
    );
    assert!(
        legal
            .cast_ways
            .iter()
            .any(|(id, w)| *id == big_hand && w.kind == CastModeKind::Blitz),
        "Big Brute (MV 4) gains CastModeKind::Blitz on stack: castable={:?} payable={:?} cast_ways={:?}",
        legal.castable,
        legal.payable,
        legal.cast_ways
    );

    // Cast first noncreature instant: should still cascade or resolve with life gain.
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_hand })
        .expect("cast discounted instant");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);

    // Cast Big Brute via Blitz with 1 Red (affinity 2 + commander_casts 2 reduces {3}{R} -> {R}).
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: big_hand })
        .expect("start casting big brute");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
            unreachable!();
        };
        let pos = options
            .iter()
            .position(|o| o.kind == CastModeKind::Blitz)
            .expect("blitz option present");
        engine.apply(me(), PlayerAction::ChooseMode(pos)).unwrap();
    }
    settle(&mut engine, me());
    let brute_bf = the(&engine, ZoneLocation::Battlefield, W3);
    assert!(
        engine
            .state()
            .object(brute_bf)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "granted Blitz gives Haste"
    );
}

#[test]
fn wave18a_combat_you_or_planeswalker_triggers_and_cant_attack() {
    static DETERRENT_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::Attacks(&Filter::And(&[
                Filter::CREATURE,
                Filter::AttackingYouOrYourPlaneswalker,
            ])),
            &[Effect::LoseLife {
                target: PlayerRel::ControllerOfEvent,
                amount: Amount::Fixed(1),
            }]
        ),
        triggered!(
            Trigger::AttackersDeclaredAgainst {
                attacker: PlayerRel::EachOpponent,
                attacked: AttackedTargetKind::YouOrYourPlaneswalker,
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }]
        ),
    ];
    static FLYERS_CANT_ATK_AB: &[AbilityDef] = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]),
        Modifier::CantAttackYouOrYourPlaneswalkers
    )];
    static AETHERSPARK_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::CantBeAttackedBy(&Filter::CREATURE)
    )];

    let deterrent = card(
        W0,
        face("Deterrent", "{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        DETERRENT_AB,
    );
    let flyer = card(
        W1,
        creature_face("Flyer", "{U}", 2, 2),
        KeywordSet::FLYING,
        &[],
    );
    let bear = card(
        W2,
        creature_face("Bear", "{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let pw = card(
        W3,
        FaceDef {
            loyalty: Some(4),
            ..face("Walker", "{3}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let cant_atk = card(
        W4,
        face("Wurm Shield", "{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        FLYERS_CANT_ATK_AB,
    );
    let spark_pw = card(
        W5,
        FaceDef {
            loyalty: Some(4),
            ..face("Protected Walker", "{4}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        AETHERSPARK_AB,
    );

    let mut engine = bench(
        18_002,
        vec![deterrent, flyer, bear, pw, cant_atk, spark_pw],
        [Seat::with(&[W0, W3, W4, W5]), Seat::with(&[W1, W2])],
    );
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );

    let pw_id = the(&engine, ZoneLocation::Battlefield, W3);
    let spark_id = the(&engine, ZoneLocation::Battlefield, W5);
    let flyer_id = the(&engine, ZoneLocation::Battlefield, W1);
    let bear_id = the(&engine, ZoneLocation::Battlefield, W2);

    // Flying creature cannot attack you OR your planeswalker.
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(flyer_id, Defender::Planeswalker(pw_id))],
                },
            )
            .is_err(),
        "CantAttackYouOrYourPlaneswalkers forbids Flyer attacking planeswalker"
    );
    // Bear cannot attack Protected Walker (CantBeAttackedBy).
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(bear_id, Defender::Planeswalker(spark_id))],
                },
            )
            .is_err(),
        "CantBeAttackedBy forbids Bear attacking protected planeswalker"
    );

    // Bear CAN attack normal planeswalker `pw_id` -> triggers both AttackingYouOrYourPlaneswalker & AttackersDeclaredAgainst!
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear_id, Defender::Planeswalker(pw_id))],
            },
        )
        .expect("Bear attacks planeswalker");
    settle(&mut engine, them());
    assert_eq!(life(&engine, me()), 21, "AttackersDeclaredAgainst gained 1");
    assert_eq!(
        life(&engine, them()),
        19,
        "AttackingYouOrYourPlaneswalker drained 1"
    );
}

#[test]
fn wave18a_granted_loyalty_trigger_and_instant_speed() {
    static YOUR_PW: Filter = Filter::And(&[Filter::PLANESWALKER, Filter::ControlledByYou]);
    static GRANTED_PLUS: AbilityDef = loyalty!(
        1,
        &[Effect::GainLife {
            amount: Amount::Fixed(3)
        }]
    );
    static GRANT_LOYALTY_AB: &[AbilityDef] = &[
        static_ability!(YOUR_PW, Modifier::GrantActivatedDefinition(&GRANTED_PLUS)),
        triggered!(
            Trigger::LoyaltyAbilityActivated {
                filter: &YOUR_PW,
                who: PlayerRel::You,
            },
            &[Effect::draw(1)]
        ),
        static_ability!(
            YOUR_PW,
            Modifier::MayActivateLoyaltyAsInstant {
                who: PlayerRel::You,
            }
        ),
    ];
    let pw = card(
        W0,
        FaceDef {
            loyalty: Some(2),
            ..face("Blank Walker", "{3}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let talent = card(
        W1,
        face("Talent Grant", "{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        GRANT_LOYALTY_AB,
    );

    let mut engine = bench(
        18_003,
        vec![pw, talent],
        [Seat::with(&[W0, W1]), Seat::default()],
    );
    // Advance to opponent's main phase to verify MayActivateLoyaltyAsInstant!
    walk_until(&mut engine, |e| {
        e.state().turn.active == them()
            && e.state().turn.step == crate::turn::Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    let pw_id = the(&engine, ZoneLocation::Battlefield, W0);
    let (_, legal) = offer(&engine);
    let granted_idx = legal
        .abilities
        .iter()
        .find(|(src, _)| *src == pw_id)
        .map(|(_, idx)| *idx)
        .expect("Granted loyalty ability activatable on opponent's turn via MayActivateLoyaltyAsInstant");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), pw_id, granted_idx);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23, "Granted [+1] gained 3 life");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "LoyaltyAbilityActivated drew 1 card"
    );
}

#[test]
fn wave18a_graveyard_harmonize_energy_mavinda_and_havengul() {
    // 1. Harmonize + Mavinda (`CastCostIncreaseUnlessTargets` {8})
    static SHOCK_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static SONG_HARMONIZE_AB: &[AbilityDef] = &[activated!(
        Cost::TAP,
        &[Effect::ModifyTargets {
            modifier: Modifier::GrantsHarmonizeManaCost,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT,
            PlayerRel::You,
        )))
    )];
    let song = card(
        W0,
        creature_face("Songcrafter", "{G}", 2, 2),
        KeywordSet::EMPTY,
        SONG_HARMONIZE_AB,
    );
    let giant_spell = card(
        W1,
        face("Four Mana Shock", "{3}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SHOCK_AB,
    );
    let bear = card(
        W2,
        creature_face("Bear Helper", "{1}{G}", 3, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18_004,
        vec![song, giant_spell, bear],
        [Seat::with(&[W0, W2]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut engine, me());
    let spell_id = the(&engine, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut engine, me(), spell_id);
    let song_bf = the(&engine, ZoneLocation::Battlefield, W0);
    activate(&mut engine, me(), song_bf, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![spell_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Tap Bear Helper (power 3) for Harmonize to pay {3} of {3}{R}; add {R} to pay remainder!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&spell_id)
            && legal
                .cast_ways
                .iter()
                .any(|(id, w)| *id == spell_id && matches!(w.kind, CastModeKind::Permission(_))),
        "Harmonize cast route offered from graveyard: castable={:?} cast_ways={:?}",
        legal.castable,
        legal.cast_ways
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_id })
        .unwrap();
    // Choose Bear Helper when asked for Harmonize tap, or handle prompt sequence:
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                let bear_id = the(&engine, ZoneLocation::Battlefield, W2);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![bear_id],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                if player_options.is_empty() {
                    let bear_id = the(&engine, ZoneLocation::Battlefield, W2);
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseTargets {
                                objects: vec![bear_id],
                                players: Vec::new(),
                            },
                        )
                        .unwrap();
                } else {
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseTargets {
                                objects: Vec::new(),
                                players: vec![them()],
                            },
                        )
                        .unwrap();
                }
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending during Harmonize cast: {other:?}"),
        }
    }
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 18, "Harmonize shock dealt 2 damage");
    assert_eq!(
        engine.state().object(spell_id).unwrap().zone,
        Zone::Exile,
        "Harmonize exiles spell instead of putting into graveyard"
    );

    // 2. Jolted Awake: MayPayEnergyEqualToTargetCmcThen
    static JOLT_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::Energy {
                who: PlayerRel::You,
                amount: Amount::Fixed(2),
            },
            Effect::MayPayEnergyEqualToTargetCmcThen {
                then: &[Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::You,
                ))],
            },
        ],
        targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        )))
    )];
    let jolt = card(
        W0,
        face("Jolt Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        JOLT_AB,
    );
    let target_cr = card(
        W1,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut ej = bench(
        18_005,
        vec![jolt, target_cr],
        [Seat::default().holding(&[W0, W1]), Seat::default()],
    );
    to_main(&mut ej, me());
    let jolt_id = the(&ej, ZoneLocation::Hand(me()), W0);
    let bear_gy = the(&ej, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut ej, me(), bear_gy);
    ej.apply(me(), PlayerAction::CastSpell { card: jolt_id })
        .unwrap();
    ej.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![bear_gy],
            players: Vec::new(),
        },
    )
    .unwrap();
    for _ in 0..32 {
        let pending = ej.pending().clone();
        match pending {
            Pending::Priority { .. } if ej.state().zones.stack_is_empty() => break,
            Pending::Priority { .. } => {
                assert!(walk_past(&mut ej, &pending), "walk_past priority");
            }
            Pending::YesNo { player, .. } => {
                ej.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            other => panic!("unexpected pending during Jolted Awake: {other:?}"),
        }
    }
    let bear_bf = ej.state().battlefield_seen().find(|&id| {
        ej.state()
            .object(id)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    });
    assert!(
        bear_bf.is_some(),
        "Jolted Awake paid 2 energy for MV 2 creature and reanimated it"
    );

    // 3. Rescue from the Underworld: DelayedTrigger with TargetAndSacrificed + ReturnDelayedObjectsToBattlefield
    static RESCUE_MANDATORY: &[CostPart] = &[CostPart::Sacrifice(&Filter::CREATURE)];
    static RESCUE_AB: &[AbilityDef] = &[spell!(
        &[Effect::RegisterDelayedTrigger {
            when: DelayedTriggerWhen::AtStep {
                step: DelayedStep::Upkeep,
                whose: Some(PlayerRel::You),
                this_turn_only: false,
                recurring: false,
            },
            condition: None,
            targets: None,
            remember: TriggerRemember::TargetAndSacrificed,
            effects: &[Effect::ReturnDelayedObjectsToBattlefield {
                owner_control: false,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        )))
    )];
    let rescue = card(
        W0,
        FaceDef {
            mandatory_additional_costs: RESCUE_MANDATORY,
            ..face("Rescue", "{0}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        RESCUE_AB,
    );
    let dead_cr = card(
        W1,
        creature_face("Fallen", "{1}{B}", 2, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let sac_cr = card(
        W2,
        creature_face("Martyr", "{B}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let mut er = bench(
        18_006,
        vec![rescue, dead_cr, sac_cr],
        [Seat::with(&[W2]).holding(&[W0, W1]), Seat::default()],
    );
    to_main(&mut er, me());
    let fallen_gy = the(&er, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut er, me(), fallen_gy);
    let martyr_bf = the(&er, ZoneLocation::Battlefield, W2);
    let rescue_id = the(&er, ZoneLocation::Hand(me()), W0);
    er.apply(me(), PlayerAction::CastSpell { card: rescue_id })
        .unwrap();
    while !matches!(er.pending(), Pending::Priority { .. }) {
        match er.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                er.apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![fallen_gy],
                        players: Vec::new(),
                    },
                )
                .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                er.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![martyr_bf],
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected pending during Rescue: {other:?}"),
        }
    }
    settle(&mut er, me());
    // Walk to your next upkeep.
    let next_turn = er.state().turn.number + 2;
    walk_until(&mut er, |e| {
        e.state().turn.number >= next_turn
            && e.state().turn.active == me()
            && e.state().turn.step == crate::turn::Step::Main
    });
    let bf_count = er.state().battlefield_seen().count();
    assert_eq!(
        bf_count, 2,
        "Rescue returns both the targeted card and the sacrificed creature at your next upkeep"
    );

    // 4. Havengul Lich + Mavinda + Spirit-Sister's Call
    static LICH_TARGET_AB: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::GainLife {
            amount: Amount::Fixed(5)
        }]
    )];
    static HAVENGUL_AB: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[
            Effect::ModifyTargets {
                modifier: Modifier::MayCastFromGraveyardTargeted {
                    free: false,
                    exile_instead_of_graveyard: false,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::GrantActivatedAbilitiesWhenTargetCast {
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::EachPlayer,
        )))
    )];
    let lich = card(
        W0,
        creature_face("Lich", "{3}{U}{B}", 4, 4),
        KeywordSet::EMPTY,
        HAVENGUL_AB,
    );
    let gift_cr = card(
        W1,
        creature_face("Gifted", "{0}", 1, 1),
        KeywordSet::EMPTY,
        LICH_TARGET_AB,
    );
    let mut el = bench(
        18_007,
        vec![lich, gift_cr],
        [Seat::with(&[W0]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut el, me());
    let lich_bf = the(&el, ZoneLocation::Battlefield, W0);
    let gift_gy = the(&el, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut el, me(), gift_gy);
    activate(&mut el, me(), lich_bf, 0);
    el.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![gift_gy],
        },
    )
    .unwrap();
    settle(&mut el, me());
    el.apply(me(), PlayerAction::CastSpell { card: gift_gy })
        .unwrap();
    settle(&mut el, me());
    // Now Lich gained Gifted's activated ability (Gain 5 life)!
    let (_, legal_lich) = offer(&el);
    let stolen_opt = legal_lich
        .abilities
        .iter()
        .find(|(src, idx)| *src == lich_bf && *idx != 0);
    assert!(
        stolen_opt.is_some(),
        "Havengul Lich gains targeted creature's activated ability when cast"
    );
    let (_, stolen_idx) = *stolen_opt.unwrap();
    activate(&mut el, me(), lich_bf, stolen_idx);
    settle(&mut el, me());
    assert_eq!(life(&el, me()), 25);
}

#[test]
fn wave18a_mavinda_command_and_spirit_sisters_call() {
    static PERMANENT_FILTER: Filter = Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::LAND)
            .union(TypeSet::PLANESWALKER),
    );
    // 1. Mavinda: CastCostIncreaseUnlessTargets { filter: YOUR_CREATURE, generic: 8 }
    static BOLT_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static MAVINDA_AB: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[
            Effect::ModifyTargets {
                modifier: Modifier::MayCastFromGraveyardTargeted {
                    free: false,
                    exile_instead_of_graveyard: true,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::CastCostIncreaseUnlessTargets {
                    filter: &Filter::YOUR_CREATURE,
                    generic: 8,
                },
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT,
            PlayerRel::You,
        )))
    )];
    let mav = card(
        W0,
        creature_face("Mav", "{2}{W}", 2, 3),
        KeywordSet::EMPTY,
        MAVINDA_AB,
    );
    let bolt = card(
        W1,
        face("Zap", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BOLT_AB,
    );
    let my_bear = card(
        W2,
        creature_face("Friend", "{1}{G}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut em = bench(
        18_008,
        vec![mav, bolt, my_bear],
        [Seat::with(&[W0, W2]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut em, me());
    let bolt_gy = the(&em, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut em, me(), bolt_gy);
    let mav_bf = the(&em, ZoneLocation::Battlefield, W0);
    activate(&mut em, me(), mav_bf, 0);
    em.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![bolt_gy],
        },
    )
    .unwrap();
    settle(&mut em, me());
    add_pool(&mut em, me(), ManaColor::Red, 1);
    // With only {R} (not {8}{R}), bolt_gy is still castable because Friend (YOUR_CREATURE) is legal!
    let (_, legal_m) = offer(&em);
    assert!(
        legal_m.castable.contains(&bolt_gy),
        "Mavinda avoids {{8}} tax when targeting your own creature"
    );
    em.apply(me(), PlayerAction::CastSpell { card: bolt_gy })
        .unwrap();
    let friend_id = the(&em, ZoneLocation::Battlefield, W2);
    em.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![friend_id],
            players: Vec::new(),
        },
    )
    .unwrap();
    settle(&mut em, me());
    assert_eq!(
        em.state().object(bolt_gy).unwrap().zone,
        Zone::Exile,
        "Mavinda replacement exiles spell instead of graveyard"
    );

    // 2. Command the Dreadhorde: TotalTargetsManaValue + multi-reanimate
    static CMD_SPEC: TargetSpec =
        TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);
    static CMD_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::TotalTargetsManaValue,
                target: TargetSpec::Player(PlayerRel::You),
            },
            Effect::reanimate(CMD_SPEC),
        ],
        targets = Some(TargetReq::up_to(CMD_SPEC, u8::MAX))
    )];
    let cmd = card(
        W0,
        face("Command", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        CMD_AB,
    );
    let c3 = card(
        W1,
        creature_face("Three", "{2}{B}", 3, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let c2 = card(
        W2,
        creature_face("Two", "{1}{B}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut ec = bench(
        18_009,
        vec![cmd, c3, c2],
        [Seat::default().holding(&[W0, W1, W2]), Seat::default()],
    );
    to_main(&mut ec, me());
    let c3_gy = the(&ec, ZoneLocation::Hand(me()), W1);
    let c2_gy = the(&ec, ZoneLocation::Hand(me()), W2);
    move_to_graveyard(&mut ec, me(), c3_gy);
    move_to_graveyard(&mut ec, me(), c2_gy);
    let cmd_id = the(&ec, ZoneLocation::Hand(me()), W0);
    ec.apply(me(), PlayerAction::CastSpell { card: cmd_id })
        .unwrap();
    ec.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![c3_gy, c2_gy],
            players: Vec::new(),
        },
    )
    .unwrap();
    settle(&mut ec, me());
    assert_eq!(
        life(&ec, me()),
        15,
        "Command dealt 3+2=5 damage to controller"
    );
    assert_eq!(
        ec.state().battlefield_seen().count(),
        2,
        "Command reanimated both graveyard creatures"
    );

    // 3. Spirit-Sister's Call: SharesCardTypeWithReference + ExileInsteadOfLeavingBattlefield
    static SAC_SHARED: Filter =
        Filter::And(&[PERMANENT_FILTER, Filter::SharesCardTypeWithReference]);
    static CALL_THEN: Effect = Effect::Sequence(&[
        Effect::reanimate(TargetSpec::CardInGraveyard(
            &PERMANENT_FILTER,
            PlayerRel::You,
        )),
        Effect::ModifyTargets {
            modifier: Modifier::ExileInsteadOfLeavingBattlefield,
            duration: Duration::Indefinitely,
        },
    ]);
    static CALL_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMaySacrificeNThen {
            player: PlayerRel::You,
            filter: &SAC_SHARED,
            count: 1,
            effect: &CALL_THEN,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &PERMANENT_FILTER,
            PlayerRel::You,
        )))
    )];
    let call = card(
        W0,
        face("Call", "{3}{W}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CALL_AB,
    );
    let sac_art = card(
        W1,
        face("Bauble", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let gy_art = card(
        W2,
        face("Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let mut es = bench(
        18_010,
        vec![call, sac_art, gy_art],
        [Seat::with(&[W0, W1]).holding(&[W2]), Seat::default()],
    );
    let gy_id = the(&es, ZoneLocation::Hand(me()), W2);
    move_to_graveyard(&mut es, me(), gy_id);
    let bauble_id = the(&es, ZoneLocation::Battlefield, W1);
    walk_until(
        &mut es,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == me()),
    );
    es.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![gy_id],
            players: Vec::new(),
        },
    )
    .unwrap();
    for _ in 0..32 {
        let pending = es.pending().clone();
        match pending {
            Pending::Priority { .. } if es.state().zones.stack_is_empty() => break,
            Pending::Priority { .. } => {
                assert!(walk_past(&mut es, &pending), "walk_past priority");
            }
            Pending::YesNo { player, .. } => {
                es.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                es.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![bauble_id],
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected pending during Spirit-Sister's Call: {other:?}"),
        }
    }
    assert!(es.state().zones.stack_is_empty(), "trigger resolved");
    assert_ne!(es.state().snapshot_hash(), 0, "deterministic hash nonzero");
    assert_eq!(
        es.state().object(gy_id).unwrap().zone,
        Zone::Battlefield,
        "Spirit-Sister's Call reanimated artifact sharing card type with sacrificed Bauble"
    );
    assert!(
        es.state().effects.iter().any(|fx| fx.modifier
            == Modifier::ExileInsteadOfLeavingBattlefield
            && crate::effects::applies_to(&es.state, fx, es.state.object(gy_id).unwrap())),
        "reanimated permanent has ExileInsteadOfLeavingBattlefield"
    );
}

#[test]
fn wave18a_demonstrate_and_ripple_copy_and_cast() {
    static STACK_YOU: Filter =
        Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Stack)]);
    static DEMO_AB: &[AbilityDef] = &[static_ability!(
        STACK_YOU,
        Modifier::AddKeyword(KeywordSet::DEMONSTRATE)
    )];
    static LIFE3_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(3),
    }])];
    let doc = card(
        W0,
        creature_face("Doc", "{3}{U}{R}", 4, 4),
        KeywordSet::EMPTY,
        DEMO_AB,
    );
    let spell3 = card(
        W1,
        face("Spark Tonic", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        LIFE3_AB,
    );
    let mut ed = bench(
        18_011,
        vec![doc, spell3],
        [Seat::with(&[W0]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut ed, me());
    let tonic = the(&ed, ZoneLocation::Hand(me()), W1);
    ed.apply(me(), PlayerAction::CastSpell { card: tonic })
        .unwrap();
    for _ in 0..16 {
        let p = ed.pending().clone();
        match p {
            Pending::Priority { .. } if ed.state().zones.stack_is_empty() => break,
            Pending::Priority { .. } => assert!(walk_past(&mut ed, &p)),
            Pending::YesNo { player, .. } => ed.apply(player, PlayerAction::YesNo(true)).unwrap(),
            other => panic!("unexpected pending during Demonstrate: {other:?}"),
        }
    }
    assert_eq!(life(&ed, me()), 26, "Demonstrate grants controller a copy");
    assert_eq!(life(&ed, them()), 23, "Demonstrate gives opponent a copy");

    static RIPPLE_AB: &[AbilityDef] = &[static_ability!(STACK_YOU, Modifier::GrantRipple(4))];
    static LIFE2_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(2),
    }])];
    let stone = card(
        W0,
        face("Ripple Stone", "{5}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        RIPPLE_AB,
    );
    let wave = card(
        W1,
        face("Ripple Wave", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        LIFE2_AB,
    );
    let mut er = bench(
        18_012,
        vec![stone, wave],
        [Seat::with(&[W0]).holding(&[W1, W1, W1]), Seat::default()],
    );
    to_main(&mut er, me());
    let hand_waves: Vec<ObjectId> = er
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .filter(|&id| {
            er.state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == CardIndex::new(W1)))
        })
        .collect();
    assert_eq!(hand_waves.len(), 3);
    for &lib_card in &hand_waves[1..] {
        er.state
            .move_object(
                lib_card,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    refresh(&mut er);
    let wave_h = hand_waves[0];
    er.apply(me(), PlayerAction::CastSpell { card: wave_h })
        .unwrap();
    for _ in 0..32 {
        let p = er.pending().clone();
        match p {
            Pending::Priority { .. } if er.state().zones.stack_is_empty() => break,
            Pending::Priority { .. } => assert!(walk_past(&mut er, &p)),
            Pending::YesNo { player, .. } => er.apply(player, PlayerAction::YesNo(true)).unwrap(),
            Pending::ChooseCards {
                player, options, ..
            } => {
                er.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: options.into_iter().take(1).collect(),
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected pending during Ripple: {other:?}"),
        }
    }
    assert_eq!(
        life(&er, me()),
        26,
        "Ripple 4 reveals and free-casts both matching library cards"
    );
}

#[test]
fn wave18a_grant_prowl_and_freerunning_alternatives() {
    use baylee_core::generated::subtypes::creature::{ASSASSIN, GOBLIN};
    static STACK_GOB: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::HasSubtype(GOBLIN),
        Filter::ControlledByYou,
        Filter::InZone(ZoneRef::Stack),
    ]);
    static STACK_ASN: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::HasSubtype(ASSASSIN),
        Filter::ControlledByYou,
        Filter::InZone(ZoneRef::Stack),
    ]);
    static GRANTER_AB: &[AbilityDef] = &[
        static_ability!(STACK_GOB, Modifier::GrantProwl(ManaCost::parse("{U}"))),
        static_ability!(
            STACK_ASN,
            Modifier::GrantFreerunning(ManaCost::parse("{B}{B}"))
        ),
    ];
    let mut gob_asn_face = creature_face("Sneak Blade", "{2}", 2, 2);
    gob_asn_face.subtypes = &[GOBLIN, ASSASSIN];
    let mut gob_spell_face = creature_face("Expensive Goblin", "{4}{U}{U}", 3, 3);
    gob_spell_face.subtypes = &[GOBLIN];
    let mut asn_spell_face = creature_face("Expensive Assassin", "{5}{B}{B}", 4, 4);
    asn_spell_face.subtypes = &[ASSASSIN];

    let granter = card(
        W0,
        creature_face("Granter", "{1}", 1, 1),
        KeywordSet::EMPTY,
        GRANTER_AB,
    );
    let attacker = card(W1, gob_asn_face, KeywordSet::EMPTY, &[]);
    let gob_card = card(W2, gob_spell_face, KeywordSet::EMPTY, &[]);
    let asn_card = card(W3, asn_spell_face, KeywordSet::EMPTY, &[]);
    let mut engine = bench(
        18_013,
        vec![granter, attacker, gob_card, asn_card],
        [Seat::with(&[W0, W1]).holding(&[W2, W3]), Seat::default()],
    );
    let atk_id = the(&engine, ZoneLocation::Battlefield, W1);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(atk_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let gob_h = the(&engine, ZoneLocation::Hand(me()), W2);
    let asn_h = the(&engine, ZoneLocation::Hand(me()), W3);
    for spell_id in [gob_h, asn_h] {
        engine
            .apply(me(), PlayerAction::CastSpell { card: spell_id })
            .unwrap();
        if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
            let alt = options
                .iter()
                .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                .expect("granted Prowl/Freerunning alternative option offered");
            engine.apply(me(), PlayerAction::ChooseMode(alt)).unwrap();
        }
        settle(&mut engine, me());
    }
    assert_eq!(
        engine.state().object(gob_h).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(asn_h).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn wave18a_grant_casualty_and_replicate_card_mana_cost() {
    static STACK_INST_YOU: Filter = Filter::And(&[
        Filter::INSTANT,
        Filter::ControlledByYou,
        Filter::InZone(ZoneRef::Stack),
    ]);
    static GRANTER_AB: &[AbilityDef] = &[
        static_ability!(STACK_INST_YOU, Modifier::GrantCasualty(2)),
        static_ability!(STACK_INST_YOU, Modifier::GrantReplicateCardManaCost),
    ];
    static GAIN3_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(3),
    }])];
    let painter = card(
        W0,
        creature_face("Painter", "{U}{B}{R}", 1, 3),
        KeywordSet::EMPTY,
        GRANTER_AB,
    );
    let djinn_bolt = card(
        W1,
        face("Boon", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        GAIN3_AB,
    );
    let fodder = card(
        W2,
        creature_face("Zombie", "{B}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(
        18_014,
        vec![painter, djinn_bolt, fodder],
        [Seat::with(&[W0, W2]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let boon = the(&engine, ZoneLocation::Hand(me()), W1);
    let zom = the(&engine, ZoneLocation::Battlefield, W2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: boon })
        .unwrap();
    for _ in 0..16 {
        let p = engine.pending().clone();
        match p {
            Pending::Priority { .. } if engine.state().zones.stack_is_empty() => break,
            Pending::Priority { .. } => assert!(walk_past(&mut engine, &p)),
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                engine.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![zom] })
                    .unwrap();
            }
            other => panic!("unexpected pending during Casualty+Replicate: {other:?}"),
        }
    }
    assert_eq!(
        life(&engine, me()),
        29,
        "Original + Casualty copy + Replicate copy resolve for +9 life"
    );
}

#[test]
fn wave18a_mavinda_tax_branch_and_exile_instead_of_graveyard_rider() {
    static MAV_AB: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[
            Effect::ModifyTargets {
                modifier: Modifier::MayCastFromGraveyardTargeted {
                    free: false,
                    exile_instead_of_graveyard: true,
                },
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::CastCostIncreaseUnlessTargets {
                    filter: &Filter::YOUR_CREATURE,
                    generic: 8,
                },
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        )))
    )];
    static TARGET_PLAYER_OR_CR: &[AbilityDef] = &[spell!(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    let mav = card(
        W0,
        creature_face("Mavinda", "{2}{W}", 2, 3),
        KeywordSet::EMPTY,
        MAV_AB,
    );
    let bolt = card(
        W1,
        face("Spark", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TARGET_PLAYER_OR_CR,
    );
    let opp_cr = card(
        W2,
        creature_face("Opp Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(
        18_015,
        vec![mav, bolt, opp_cr],
        [Seat::with(&[W0]).holding(&[W1]), Seat::with(&[W2])],
    );
    to_main(&mut engine, me());
    let spark_gy = the(&engine, ZoneLocation::Hand(me()), W1);
    move_to_graveyard(&mut engine, me(), spark_gy);
    let mav_bf = the(&engine, ZoneLocation::Battlefield, W0);
    activate(&mut engine, me(), mav_bf, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![spark_gy],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal_short) = offer(&engine);
    assert!(
        legal_short.castable.contains(&spark_gy),
        "offered castable via Mavinda; targeting opponent creature requires +8"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: spark_gy })
        .unwrap();
    let opp_id = the(&engine, ZoneLocation::Battlefield, W2);
    // Targeting opponent's creature triggers +8 tax -> payment fails -> CR 601.2/734.1 reverses cast!
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_id],
                players: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(spark_gy).unwrap().zone,
        Zone::Graveyard,
        "Mavinda +8 tax reverses targeting opponent's creature without 8 generic mana"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "reversed cast returns to priority"
    );
    add_pool(&mut engine, me(), ManaColor::Colorless, 8);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spark_gy })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_id],
                players: Vec::new(),
            },
        )
        .unwrap();
    let on_stack = engine.state().object(spark_gy).unwrap();
    assert!(
        on_stack
            .riders
            .contains(&crate::object::Rider::ExileInsteadOfGraveyard)
            && !on_stack.riders.contains(&crate::object::Rider::Flashback),
        "Mavinda attaches ExileInsteadOfGraveyard, not Flashback"
    );
    // If returned from stack to hand, ExileInsteadOfGraveyard only replaces Graveyard, not Hand.
    let mut bounce_state = engine.state().clone();
    bounce_state
        .move_object(
            spark_gy,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(bounce_state.object(spark_gy).unwrap().zone, Zone::Hand);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(spark_gy).unwrap().zone, Zone::Exile);
}

#[test]
#[allow(clippy::too_many_lines)] // tests all Wave 18a suspended ops, delayed fields, and wizard fields together
fn wave18a_snapshot_hash_and_fingerprint_sensitivity() {
    static SAMPLE_THEN: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    let mut engine = bench(18_016, vec![], [Seat::default(), Seat::default()]);
    to_main(&mut engine, me());
    let hand_card = engine.state().zones.list(ZoneLocation::Library(me()))[0];
    let dummy_ref = baylee_core::ids::DamageSourceRef {
        object: hand_card,
        version: 1,
    };

    // 1. AwaitingOp variants in search_fingerprint() + Engine::snapshot_hash/fingerprint().
    let ops = [
        crate::resolve::AwaitingOp::DemonstrateConfirm,
        crate::resolve::AwaitingOp::DemonstrateOpponent,
        crate::resolve::AwaitingOp::RippleReveal { n: 4 },
        crate::resolve::AwaitingOp::RippleBottom {
            rest: vec![hand_card],
        },
        crate::resolve::AwaitingOp::MayPayEnergyThen {
            amount: 2,
            then: SAMPLE_THEN,
        },
    ];
    let mut op_fps = Vec::new();
    for op in &ops {
        let sf = op.search_fingerprint();
        assert_ne!(sf, 0, "search_fingerprint must be non-zero for {op:?}");
        assert!(
            !op_fps.contains(&sf),
            "distinct search_fingerprint for {op:?}"
        );
        op_fps.push(sf);
    }

    // 2. SubjectContext.demonstrate_opp affects snapshot_hash() and Engine::fingerprint().
    let mut res = crate::resolve::Resolution {
        source: hand_card,
        on_stack: hand_card,
        controller: me(),
        effects: Vec::new(),
        pc: 0,
        targets: smallvec::SmallVec::new(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        targeted: false,
        awaiting: Some(crate::resolve::AwaitingOp::DemonstrateConfirm),
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    engine.resolution = Some(res.clone());
    let h_base = engine.snapshot_hash();
    let f_base = engine.fingerprint();
    let cp = crate::engine::checkpoint::Checkpoint::capture(&engine);
    res.subject.demonstrate_opp = Some((dummy_ref, them()));
    engine.resolution = Some(res);
    assert_ne!(engine.snapshot_hash(), h_base, "demonstrate_opp hashed");
    assert_ne!(
        engine.fingerprint(),
        f_base,
        "demonstrate_opp fingerprinted"
    );
    cp.restore(&mut engine);
    assert_eq!(engine.snapshot_hash(), h_base);
    assert_eq!(engine.fingerprint(), f_base);
    engine.resolution = None;

    // 3. DelayedTriggerSpec.related_objects affects snapshot_hash() & fingerprint().
    let h_pre_rel = engine.snapshot_hash();
    let f_pre_rel = engine.fingerprint();
    engine.state.delayed.push(crate::state::DelayedTrigger {
        controller: me(),
        when: crate::state::DelayedWhen::NextUpkeep,
        action: crate::state::DelayedAction::GeneralTrigger(Box::new(
            crate::state::DelayedTriggerSpec {
                source: hand_card,
                source_version: 1,
                effects: SAMPLE_THEN,
                text: crate::text_changes::TextChangeMap::IDENTITY,
                condition: None,
                targets: None,
                remembered: None,
                related_objects: vec![dummy_ref],
            },
        )),
    });
    assert_ne!(engine.snapshot_hash(), h_pre_rel, "related_objects hashed");
    assert_ne!(
        engine.fingerprint(),
        f_pre_rel,
        "related_objects fingerprinted"
    );

    // 4. CastWizard.{harmonize_tap, casualty_power, granted_replicate} affect snapshot_hash() & fingerprint().
    let mut wiz = Box::new(crate::engine::cast_wizard::CastWizard::new(
        hand_card,
        me(),
        crate::engine::cast_wizard::WizardStage::Done,
        Vec::new(),
    ));
    engine.cast_wizard = Some(wiz.clone());
    let h_w0 = engine.snapshot_hash();
    let f_w0 = engine.fingerprint();
    wiz.harmonize_tap = Some(hand_card);
    engine.cast_wizard = Some(wiz.clone());
    let h_harm = engine.snapshot_hash();
    let f_harm = engine.fingerprint();
    assert_ne!(h_harm, h_w0, "harmonize_tap hashed");
    assert_ne!(f_harm, f_w0, "harmonize_tap fingerprinted");
    wiz.casualty_power = Some(2);
    engine.cast_wizard = Some(wiz.clone());
    let h_cas = engine.snapshot_hash();
    let f_cas = engine.fingerprint();
    assert_ne!(h_cas, h_harm, "casualty_power hashed");
    assert_ne!(f_cas, f_harm, "casualty_power fingerprinted");
    wiz.granted_replicate = Some(ManaCost::parse("{R}"));
    engine.cast_wizard = Some(wiz);
    assert_ne!(engine.snapshot_hash(), h_cas, "granted_replicate hashed");
    assert_ne!(
        engine.fingerprint(),
        f_cas,
        "granted_replicate fingerprinted"
    );
}
