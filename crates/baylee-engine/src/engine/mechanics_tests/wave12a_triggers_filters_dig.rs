//! Beat 18 / Engine Wave 12a deterministic rule tests:
//! 1. `Condition::TriggerOnly` (`Attacks.NoResolvingCheck`, `CR 603.4`) — checked
//!    only at trigger time (not re-checked on resolution), and composable inside
//!    `Condition::All` alongside a real intervening-`if` clause.
//! 2. `Filter::Modified` & `Filter::YOUR_MODIFIED_CREATURE` (`CR 700.9`) —
//!    counters, Equipment (including opponent-controlled), own Aura vs opponent
//!    Aura, non-creature artifacts, off-battlefield cards, and LTB look-back.
//! 3. `Effect::RevealUntilThen` (`DigUntil.RememberFound`, `CR 701.20a`) —
//!    single & multi-card `count`, `exile_while_revealing`, `ExileLinked` + free
//!    `Effect::Play`, `BottomRandom` & `BottomChosenOrder`, and `EventManaValue`.
//! 4. `Effect::RepeatEachObject` (`RepeatEach.RepeatCards`, `CR 101.4`,
//!    `CR 608.2h`) — deterministic APNAP iteration, interactive `Pending`
//!    suspension, mid-loop departure skipping, and `only_found: true` scoping.

use super::*;
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, Filter, PlayCostMode, PlayCount, PlaySource, PlayerRel,
    RevealUntilFound, RevealUntilRest, TargetReq, TargetSpec, Trigger, ZoneSel, spell, triggered,
};
use baylee_core::generated::subtypes::{artifact, enchantment};
use baylee_core::ids::Defender;

const C0: u32 = 99_910;
const C1: u32 = 99_911;
const C2: u32 = 99_912;
const C3: u32 = 99_913;
const C4: u32 = 99_914;
const C5: u32 = 99_915;

static OTHER_ATTACKER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::Attacking,
    Filter::Another,
]);
static TWO_OTHER_ATTACKERS: Condition = Condition::AmountAtLeast(
    Amount::CountOf {
        filter: &OTHER_ATTACKER,
        zone: ZoneSel::Battlefield,
    },
    2,
);
static BATTALION_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::gain_life(4)],
    condition = Some(Condition::TriggerOnly(&TWO_OTHER_ATTACKERS))
)];
static ZAP_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::Destroy {
        target: TargetSpec::Object(&Filter::CREATURE),
        no_regen: true,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];
static AURA_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::AttachSelf {
        target: TargetSpec::Object(&Filter::CREATURE),
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn move_hand_card_to_top(engine: &mut Bench, owner: PlayerId, index: u32) -> ObjectId {
    let id = *objects(engine, ZoneLocation::Hand(owner), index)
        .first()
        .expect("card in hand");
    let _ = engine.state.move_object(
        id,
        ZoneLocation::Library(owner),
        ZonePosition::Top,
        Cause::Effect,
    );
    id
}

fn cast_flash_aura(engine: &mut Bench, caster: PlayerId, aura: ObjectId, host: ObjectId) {
    if caster != me() {
        engine
            .apply(me(), PlayerAction::PassPriority)
            .expect("pass priority to opponent");
    }
    engine
        .apply(caster, PlayerAction::CastSpell { card: aura })
        .expect("cast Flash Aura");
    engine
        .apply(
            caster,
            PlayerAction::ChooseTargets {
                objects: vec![host],
                players: vec![],
            },
        )
        .expect("target host");
    settle(engine, me());
}

fn make_aura_card(idx: u32, name: &'static str) -> &'static CardDef {
    let mut f = face(name, "{0}", TypeSet::ENCHANTMENT);
    f.subtypes = &[enchantment::AURA];
    card(idx, f, KeywordSet::FLASH, AURA_ABILITIES)
}

fn make_equip_card(idx: u32, name: &'static str) -> &'static CardDef {
    let mut f = face(name, "{1}", TypeSet::ARTIFACT);
    f.subtypes = &[artifact::EQUIPMENT];
    card(idx, f, KeywordSet::EMPTY, &[])
}

#[test]
fn trigger_only_battalion_skips_when_attacking_short_of_threshold() {
    let leader = card(
        C0,
        creature_face("Battalion Captain", "{1}{W}", 2, 2),
        KeywordSet::HASTE,
        BATTALION_ABILITIES,
    );
    let ally = card(
        C1,
        creature_face("Legionnaire", "{W}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );
    let mut engine = bench(
        1201,
        vec![leader, ally],
        [Seat::with(&[C0, C1]), Seat::default()],
    );
    let leader_id = the(&engine, ZoneLocation::Battlefield, C0);
    let ally_id = the(&engine, ZoneLocation::Battlefield, C1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (leader_id, Defender::Player(them())),
                    (ally_id, Defender::Player(them())),
                ],
            },
        )
        .expect("declare 2 attackers");
    assert_eq!(
        triggered(&engine, leader_id),
        0,
        "1 co-attacker < 2 must not trigger"
    );
    assert_eq!(life(&engine, me()), 20);
}

#[test]
fn trigger_only_battalion_resolves_after_co_attackers_die() {
    let leader = card(
        C0,
        creature_face("Battalion Captain", "{1}{W}", 2, 2),
        KeywordSet::HASTE,
        BATTALION_ABILITIES,
    );
    let ally = card(
        C1,
        creature_face("Legionnaire", "{W}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );
    let zapper = card(
        C2,
        face("Sniper Tower", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ZAP_ABILITIES,
    );
    let mut engine = bench(
        1202,
        vec![leader, ally, zapper],
        [Seat::with(&[C0, C1, C1, C2]), Seat::default()],
    );
    let leader_id = the(&engine, ZoneLocation::Battlefield, C0);
    let allies = objects(&engine, ZoneLocation::Battlefield, C1);
    let zap_id = the(&engine, ZoneLocation::Battlefield, C2);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (leader_id, Defender::Player(them())),
                    (allies[0], Defender::Player(them())),
                    (allies[1], Defender::Player(them())),
                ],
            },
        )
        .expect("declare 3 attackers");
    assert_eq!(triggered(&engine, leader_id), 1);
    assert!(!engine.state().zones.stack_is_empty());

    for &victim in &allies {
        activate(&mut engine, me(), zap_id, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .expect("target co-attacker");
    }
    settle(&mut engine, me());
    assert!(objects(&engine, ZoneLocation::Battlefield, C1).is_empty());
    assert_eq!(
        life(&engine, me()),
        24,
        "TriggerOnly condition is not re-checked on resolution"
    );
}

#[test]
fn trigger_only_inside_condition_all_still_checks_real_intervening_if() {
    static TRIGGER_PART: Condition = Condition::TriggerOnly(&Condition::YourTurn);
    static REAL_IF_PART: Condition = Condition::IsMonarch(PlayerRel::You);
    static COMBINED: Condition = Condition::All(&[TRIGGER_PART, REAL_IF_PART]);
    static ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::gain_life(5)],
        condition = Some(COMBINED)
    )];
    static STEAL_CROWN: &[AbilityDef] =
        &[free(&[Effect::BecomeMonarch(PlayerRel::Opponent)], None)];

    let attacker = card(
        C0,
        creature_face("Court Vanguard", "{1}{W}", 2, 2),
        KeywordSet::HASTE,
        ABILITIES,
    );
    let thief = card(
        C1,
        face("Abdication Bell", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STEAL_CROWN,
    );
    let mut engine = bench(
        1203,
        vec![attacker, thief],
        [Seat::with(&[C0, C1]), Seat::default()],
    );
    engine.state.monarch = Some(me());
    let atk_id = the(&engine, ZoneLocation::Battlefield, C0);
    let bell_id = the(&engine, ZoneLocation::Battlefield, C1);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(atk_id, Defender::Player(them()))],
            },
        )
        .expect("attack while monarch");
    assert_eq!(triggered(&engine, atk_id), 1);

    activate(&mut engine, me(), bell_id, 0);
    settle(&mut engine, me());
    assert_eq!(engine.state().monarch, Some(them()));
    assert_eq!(
        life(&engine, me()),
        20,
        "real intervening-if inside Condition::All still aborts resolution"
    );
}

static SWEEP_MODIFIED: &[AbilityDef] = &[free(
    &[Effect::AddCounterFilter {
        filter: &Filter::YOUR_MODIFIED_CREATURE,
        kind: CounterKind::Shield,
        amount: Amount::Fixed(1),
    }],
    None,
)];

#[test]
fn modified_filter_distinguishes_counters_equipment_own_aura_and_opponent_aura() {
    let vanilla = card(
        C0,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let relic = card(
        C1,
        face("Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let equip = make_equip_card(C2, "Short Sword");
    let aura = make_aura_card(C3, "Vow");
    let probe = card(
        C4,
        face("Modified Scanner", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SWEEP_MODIFIED,
    );
    let mut engine = bench(
        1204,
        vec![vanilla, relic, equip, aura, probe],
        [
            Seat::with(&[C0, C0, C0, C0, C0, C1, C4]).holding(&[C0, C3]),
            Seat::with(&[C2]).holding(&[C3]),
        ],
    );
    to_main(&mut engine, me());
    let bears = objects(&engine, ZoneLocation::Battlefield, C0);
    let (plain_b, countered_b, opp_eq_b, opp_aura_b, own_aura_b) =
        (bears[0], bears[1], bears[2], bears[3], bears[4]);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C1);
    let my_aura = the(&engine, ZoneLocation::Hand(me()), C3);
    let opp_aura = the(&engine, ZoneLocation::Hand(them()), C3);
    let opp_eq = the(&engine, ZoneLocation::Battlefield, C2);

    cast_flash_aura(&mut engine, me(), my_aura, own_aura_b);
    cast_flash_aura(&mut engine, them(), opp_aura, opp_aura_b);
    engine.state.object_mut(opp_eq).unwrap().attached_to = Some(opp_eq_b);

    let hand_bear = the(&engine, ZoneLocation::Hand(me()), C0);
    let _ = engine.state.move_object(
        hand_bear,
        ZoneLocation::Exile(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    for (id, kind, n) in [
        (hand_bear, CounterKind::Time, 2),
        (countered_b, CounterKind::P1P1, 1),
        (relic_id, CounterKind::Charge, 1),
    ] {
        engine.state.object_mut(id).unwrap().counters.add(kind, n);
    }
    engine.state.refresh_characteristics();
    refresh_offer(&mut engine);

    let scanner = the(&engine, ZoneLocation::Battlefield, C4);
    activate(&mut engine, me(), scanner, 0);
    settle(&mut engine, me());

    let shield = |id: ObjectId| {
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::Shield)
    };
    assert_eq!(shield(plain_b), 0);
    assert_eq!(shield(countered_b), 1);
    assert_eq!(shield(opp_eq_b), 1, "opponent Equipment counts (CR 700.9)");
    assert_eq!(shield(opp_aura_b), 0, "opponent Aura excluded (CR 700.9)");
    assert_eq!(shield(own_aura_b), 1, "own Aura counts (CR 700.9)");
    assert_eq!(shield(relic_id), 0, "non-creature artifact excluded");
    assert_eq!(shield(hand_bear), 0, "exiled card with counters excluded");
}

#[test]
fn modified_filter_ltb_lookback_fires_dies_trigger_for_counter_equipment_and_own_aura() {
    static WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::YOUR_MODIFIED_CREATURE),
        &[Effect::gain_life(3)]
    )];

    let bear = card(
        C0,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let equip = make_equip_card(C1, "Blade");
    let aura = make_aura_card(C2, "Blessing");
    let watcher = card(
        C3,
        face("Shrine", "{1}{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        WATCHER_ABILITIES,
    );
    let killer = card(
        C4,
        face("Altar", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ZAP_ABILITIES,
    );

    let mut engine = bench(
        1205,
        vec![bear, equip, aura, watcher, killer],
        [
            Seat::with(&[C0, C0, C0, C0, C0, C1, C3, C4]).holding(&[C2]),
            Seat::default().holding(&[C2]),
        ],
    );
    to_main(&mut engine, me());
    let bears = objects(&engine, ZoneLocation::Battlefield, C0);
    let (plain_b, opp_aura_b, countered_b, eq_bear, own_aura_b) =
        (bears[0], bears[1], bears[2], bears[3], bears[4]);
    let eq_id = the(&engine, ZoneLocation::Battlefield, C1);
    let my_aura = the(&engine, ZoneLocation::Hand(me()), C2);
    let opp_aura = the(&engine, ZoneLocation::Hand(them()), C2);

    cast_flash_aura(&mut engine, me(), my_aura, own_aura_b);
    cast_flash_aura(&mut engine, them(), opp_aura, opp_aura_b);
    engine.state.object_mut(eq_id).unwrap().attached_to = Some(eq_bear);
    engine
        .state
        .object_mut(countered_b)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    engine.state.refresh_characteristics();
    refresh_offer(&mut engine);

    let altar = the(&engine, ZoneLocation::Battlefield, C4);
    for (victim, expected_life) in [
        (plain_b, 20),
        (opp_aura_b, 20),
        (countered_b, 23),
        (eq_bear, 26),
        (own_aura_b, 29),
    ] {
        activate(&mut engine, me(), altar, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .expect("choose creature to destroy");
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), expected_life);
    }
}

#[test]
fn reveal_until_then_exiles_links_and_casts_free_spell() {
    static EXILE_CAST_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RevealUntilThen {
            library: PlayerRel::You,
            filter: &Filter::NONLAND,
            count: Amount::Fixed(1),
            max_revealed: None,
            exile_while_revealing: true,
            found: RevealUntilFound::ExileLinked,
            rest: RevealUntilRest::BottomRandom,
            then: &[Effect::Play {
                source: PlaySource::LinkedExile,
                filter: &Filter::NONLAND,
                controller: PlayerRel::You,
                count: PlayCount::One,
                cost: PlayCostMode::WithoutPayingManaCost,
                cast_only: true,
                copy: false,
                exile_instead_of_graveyard: false,
                optional: false,
            }],
        }],
        None,
    )];
    static BOLT_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::Player(PlayerRel::Opponent),
    }])];

    let bolt = card(
        C0,
        face("Flame Bolt", "{2}{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT_ABILITIES,
    );
    let cascade_rig = card(
        C2,
        face("Cascade Rig", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EXILE_CAST_ABILITIES,
    );
    let mut engine = bench(
        1206,
        vec![bolt, cascade_rig],
        [
            Seat::with(&[C2]).holding(&[C0, forest(), forest()]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bolt_id = move_hand_card_to_top(&mut engine, me(), C0);
    let _f1 = move_hand_card_to_top(&mut engine, me(), forest());
    let _f2 = move_hand_card_to_top(&mut engine, me(), forest());
    refresh_offer(&mut engine);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C2);

    activate(&mut engine, me(), rig_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bolt_id],
            },
        )
        .expect("choose exiled Flame Bolt to cast for free");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        17,
        "free cast from ExileLinked dealt 3"
    );
}

#[test]
fn reveal_until_then_orders_bottom_and_reads_event_mana_value() {
    static ORDERED_MV_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RevealUntilThen {
            library: PlayerRel::You,
            filter: &Filter::NONLAND,
            count: Amount::Fixed(1),
            max_revealed: None,
            exile_while_revealing: false,
            found: RevealUntilFound::Hand,
            rest: RevealUntilRest::BottomChosenOrder,
            then: &[Effect::DealDamage {
                amount: Amount::EventManaValue,
                target: TargetSpec::Player(PlayerRel::Opponent),
            }],
        }],
        None,
    )];

    let tome = card(
        C1,
        face("Heavy Tome", "{5}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let diviner = card(
        C3,
        face("MV Diviner", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ORDERED_MV_ABILITIES,
    );
    let mut engine = bench(
        1207,
        vec![tome, diviner],
        [
            Seat::with(&[C3]).holding(&[C1, forest(), forest()]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let tome_id = move_hand_card_to_top(&mut engine, me(), C1);
    let f_low = move_hand_card_to_top(&mut engine, me(), forest());
    let f_high = move_hand_card_to_top(&mut engine, me(), forest());
    refresh_offer(&mut engine);
    let div_id = the(&engine, ZoneLocation::Battlefield, C3);

    activate(&mut engine, me(), div_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![vec![f_low, f_high]],
            },
        )
        .expect("order the 2 revealed Forests on the bottom");
    settle(&mut engine, me());

    assert_eq!(engine.state().object(tome_id).unwrap().zone, Zone::Hand);
    assert_eq!(
        life(&engine, them()),
        15,
        "5-MV revealed card dealt 5 damage"
    );
    let lib = engine.state().zones.list(ZoneLocation::Library(me()));
    assert_eq!(lib[0], f_high, "last arranged card sits on the very bottom");
    assert_eq!(lib[1], f_low);
}

#[test]
fn repeat_each_object_handles_pending_choices_on_battlefield() {
    static SWEEP_BATTLEFIELD: &[AbilityDef] = &[free(
        &[Effect::RepeatEachObject {
            zone: ZoneSel::Battlefield,
            filter: &Filter::CREATURE,
            only_found: false,
            then: &[Effect::MayDo {
                effects: &[
                    Effect::Destroy {
                        target: TargetSpec::EventObject,
                        no_regen: true,
                    },
                    Effect::GainLifeFor {
                        amount: Amount::Fixed(2),
                        who: PlayerRel::EventPlayer,
                    },
                ],
            }],
        }],
        None,
    )];

    let bear = card(
        C0,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let sweeper = card(
        C4,
        face("Cull Engine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SWEEP_BATTLEFIELD,
    );
    let mut engine = bench(
        1208,
        vec![bear, sweeper],
        [Seat::with(&[C0, C4]), Seat::with(&[C0])],
    );
    to_main(&mut engine, me());

    let cull_id = the(&engine, ZoneLocation::Battlefield, C4);
    activate(&mut engine, me(), cull_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept first");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("decline second");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);
    assert_eq!(life(&engine, them()), 20);
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C0).len(), 1);
}

#[test]
fn repeat_each_object_only_found_scopes_to_newly_found_exile_cards() {
    static DIG_TWO_AND_REPEAT: &[AbilityDef] = &[free(
        &[Effect::RevealUntilThen {
            library: PlayerRel::You,
            filter: &Filter::NONLAND,
            count: Amount::Fixed(2),
            max_revealed: None,
            exile_while_revealing: true,
            found: RevealUntilFound::Exile,
            rest: RevealUntilRest::BottomRandom,
            then: &[Effect::RepeatEachObject {
                zone: ZoneSel::ExileAll,
                filter: &Filter::NONLAND,
                only_found: true,
                then: &[Effect::DealDamage {
                    amount: Amount::EventManaValue,
                    target: TargetSpec::Player(PlayerRel::Opponent),
                }],
            }],
        }],
        None,
    )];

    let mv3 = card(
        C1,
        face("Orb", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let mv5 = card(
        C2,
        face("Idol", "{5}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let mv7 = card(
        C3,
        face("Decoy", "{7}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let digger = card(
        C5,
        face("Double Conflux", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DIG_TWO_AND_REPEAT,
    );
    let mut engine = bench(
        1209,
        vec![mv3, mv5, mv7, digger],
        [
            Seat::with(&[C5]).holding(&[C1, C2, C3, forest()]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let decoy = the(&engine, ZoneLocation::Hand(me()), C3);
    let _ = engine.state.move_object(
        decoy,
        ZoneLocation::Exile(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    let _ = move_hand_card_to_top(&mut engine, me(), C2);
    let _ = move_hand_card_to_top(&mut engine, me(), forest());
    let _ = move_hand_card_to_top(&mut engine, me(), C1);
    refresh_offer(&mut engine);

    let dig_id = the(&engine, ZoneLocation::Battlefield, C5);
    activate(&mut engine, me(), dig_id, 0);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        12,
        "3 + 5 = 8 damage, skipping 7-MV decoy"
    );
}
