//! Played mechanics and hash-sensitivity tests for Draw replacements, Moved
//! replacements, game-loss / token replacements, and damage prevention,
//! redirection, scaling, and scoped damage riders (CR 121.2, CR 614, CR 615,
//! CR 616.1).
#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, DamageAmountReplacement, DamageRecipients, DamageRider,
    DamageRiderScope, DamagedByScope, Duration, EnterModifier, EntryInsteadZone, EntryPredicate,
    Filter, GraveyardExileRider, GraveyardRedirectDest, Layer, Modifier, MultiDrawReplacementKind,
    NextDrawReplacement, PlayerRel, ReplacementRule, StaticAbility, TargetSpec, ThisWay, TokenDef,
    ZoneRef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature as creature_type;
use baylee_core::ids::Defender;
const BEAR_TOKEN: TokenDef = TokenDef {
    name: "Bear",
    types: TypeSet::CREATURE,
    subtypes: &[creature_type::BEAR],
    colors: ColorSet::of(Color::Green),
    power: Some(2),
    toughness: Some(2),
    ..TokenDef::DEFAULT
};

const ZOMBIE_TOKEN: TokenDef = TokenDef {
    name: "Zombie",
    types: TypeSet::CREATURE,
    subtypes: &[creature_type::ZOMBIE],
    colors: ColorSet::of(Color::Black),
    power: Some(2),
    toughness: Some(2),
    ..TokenDef::DEFAULT
};

fn first(engine: &Bench, zone: ZoneLocation, index: u32) -> ObjectId {
    objects(engine, zone, index)[0]
}

fn cast(engine: &mut Bench, seat: PlayerId, card: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("cast offered spell");
}

fn draw_n_spell(index: u32, n: u32, who: PlayerRel) -> &'static CardDef {
    let effects: &'static [Effect] = Box::leak(Box::new([Effect::DrawCardsFor {
        amount: Amount::Fixed(n),
        who,
    }]));
    let abilities: &'static [AbilityDef] = Box::leak(Box::new([baylee_cards_dsl::spell!(effects)]));
    card(
        index,
        face("DrawSpell", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        abilities,
    )
}

fn static_perm(
    index: u32,
    name: &'static str,
    types: TypeSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    card(
        index,
        face(name, "{0}", types),
        KeywordSet::EMPTY,
        abilities,
    )
}

fn plain_creature(
    index: u32,
    name: &'static str,
    cost: &'static str,
    p: i16,
    t: i16,
) -> &'static CardDef {
    card(
        index,
        creature_face(name, cost, p, t),
        KeywordSet::EMPTY,
        &[],
    )
}

fn to_declare_attackers(engine: &mut Bench, seat: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
}

fn advance_to_main_of(engine: &mut Bench, seat: PlayerId) {
    to_main(engine, seat);
}

// =========================================================================
// 1. Draw Replacements
// =========================================================================

#[test]
fn eruth_exiles_top_two_and_grants_play_this_turn() {
    const ERUTH: u32 = 9600;
    const DRAW1: u32 = 9601;
    const BEAR: u32 = 9602;
    static AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawExileTopN {
            who: PlayerRel::You,
            count: 2,
            face_down: false,
            may_play_this_turn: true,
        },
    )];
    let eruth = card(
        ERUTH,
        creature_face("Eruth", "{0}", 2, 4),
        KeywordSet::EMPTY,
        AB,
    );
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    let mut engine = bench(
        1901,
        vec![eruth, draw1, bear],
        [
            Seat::with(&[ERUTH]).holding(&[DRAW1, BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bear_id = first(&engine, ZoneLocation::Hand(me()), BEAR);
    let _ = engine.state.move_object(
        bear_id,
        ZoneLocation::Library(me()),
        ZonePosition::Top,
        Cause::Setup,
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    let spell = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), spell);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1,
        "Eruth replaces draw with exile; no card drawn to hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(me())).len(),
        2,
        "top two cards exiled"
    );
    assert_eq!(engine.state().object(bear_id).unwrap().zone, Zone::Exile);
    // Cast the exiled Bear directly from exile this turn!
    cast(&mut engine, me(), bear_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        Zone::Battlefield,
        "exiled card is playable this turn"
    );
}

#[test]
fn sages_of_the_anima_reveals_three_keeps_creatures_and_bottoms_rest() {
    const SAGES: u32 = 9603;
    const DRAW1: u32 = 9604;
    const BEAR: u32 = 9605;
    static AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawRevealAllMatching {
            count: 3,
            filter: &Filter::CREATURE,
        },
    )];
    let sages = card(
        SAGES,
        creature_face("Sages", "{0}", 3, 4),
        KeywordSet::EMPTY,
        AB,
    );
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    let mut engine = bench(
        1902,
        vec![sages, draw1, bear],
        [
            Seat::with(&[SAGES]).holding(&[DRAW1, BEAR, BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bears = objects(&engine, ZoneLocation::Hand(me()), BEAR);
    let (b1, b2) = (bears[0], bears[1]);
    let _ = engine.state.move_object(
        b1,
        ZoneLocation::Library(me()),
        ZonePosition::Top,
        Cause::Setup,
    );
    let _ = engine.state.move_object(
        b2,
        ZoneLocation::Library(me()),
        ZonePosition::Top,
        Cause::Setup,
    );
    let lib_before = engine.state().zones.list(ZoneLocation::Library(me())).len();

    let spell = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), spell);
    settle(&mut engine, me());

    assert_eq!(engine.state().object(b1).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().object(b2).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(me())).len(),
        lib_before - 2,
        "the 1 noncreature stayed in library on the bottom"
    );
}

#[test]
fn blood_scrivener_replaces_only_when_hellbent_and_loses_life() {
    const SCRIVENER: u32 = 9606;
    const DRAW1: u32 = 9607;
    const FILLER: u32 = 9608;
    static AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawWithDrawsThen {
            who: PlayerRel::You,
            skip_first_in_draw_step: false,
            first_extra_this_turn: false,
            condition: Some(Condition::HandSizeAtMost(0)),
            count: 2,
            lose_life: 1,
        },
    )];
    let scr = card(
        SCRIVENER,
        creature_face("Scrivener", "{0}", 2, 1),
        KeywordSet::EMPTY,
        AB,
    );
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);
    let filler = plain_creature(FILLER, "Filler", "{0}", 1, 1);

    // Positive: hand empty when draw resolves -> draws 2, loses 1 life.
    let mut engine = bench(
        1903,
        vec![scr, draw1],
        [
            Seat::with(&[SCRIVENER]).holding(&[DRAW1]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let spell = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), spell);
    settle(&mut engine, me());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(me())).len(), 2);
    assert_eq!(engine.state().players[0].life, 19);

    // Negative: hand non-empty -> draws 1, loses 0 life.
    let mut engine2 = bench(
        1904,
        vec![scr, draw1, filler],
        [
            Seat::with(&[SCRIVENER]).holding(&[DRAW1, FILLER]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut engine2, me());
    let spell2 = first(&engine2, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine2, me(), spell2);
    settle(&mut engine2, me());
    assert_eq!(
        engine2.state().zones.list(ZoneLocation::Hand(me())).len(),
        2
    );
    assert_eq!(engine2.state().players[0].life, 20);
}

#[test]
fn reed_richards_replaces_only_first_extra_draw_each_turn() {
    const REED: u32 = 9609;
    const DRAW1: u32 = 9610;
    static AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawWithDrawsThen {
            who: PlayerRel::You,
            skip_first_in_draw_step: true,
            first_extra_this_turn: true,
            condition: None,
            count: 4,
            lose_life: 0,
        },
    )];
    let reed = card(
        REED,
        creature_face("Reed", "{0}", 2, 3),
        KeywordSet::EMPTY,
        AB,
    );
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);

    let mut engine = bench(
        1905,
        vec![reed, draw1],
        [
            Seat::with(&[REED]).holding(&[DRAW1, DRAW1]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let hash_before = engine.state().snapshot_hash();
    let loop_before = engine.state().loop_signature();

    // First extra draw -> replaced with 4 draws!
    let s1 = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), s1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        5, // 1 remaining DRAW1 + 4 drawn
        "first extra draw replaced by 4"
    );
    assert_ne!(engine.state().snapshot_hash(), hash_before);
    assert_ne!(engine.state().loop_signature(), loop_before);

    // Second extra draw same turn -> NOT replaced (draws only 1)!
    let s2 = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), s2);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        5, // 5 - 1 cast + 1 drawn = 5
        "second extra draw in the same turn is not replaced"
    );
}

#[test]
fn ormos_and_out_of_the_tombs_empty_library_replacements() {
    const ORMOS: u32 = 9611;
    const TOMBS: u32 = 9612;
    const DRAW1: u32 = 9613;
    const BEAR: u32 = 9614;

    static ORMOS_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawEmptyLibraryWithCounters {
            kind: CounterKind::P1P1,
            count: 5,
        },
    )];
    static TOMBS_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawEmptyLibraryReanimateOrLose {
            filter: &Filter::CREATURE,
        },
    )];
    let ormos = card(
        ORMOS,
        creature_face("Ormos", "{0}", 5, 5),
        KeywordSet::EMPTY,
        ORMOS_AB,
    );
    let tombs = static_perm(TOMBS, "Tombs", TypeSet::ENCHANTMENT, TOMBS_AB);
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    // Ormos: empty library -> 5 +1/+1 counters, no loss.
    let mut e1 = bench(
        1906,
        vec![ormos, draw1],
        [Seat::with(&[ORMOS]).holding(&[DRAW1]), Seat::default()],
    );
    to_main(&mut e1, me());
    let lib_cards = e1.state().zones.list(ZoneLocation::Library(me())).clone();
    for c in lib_cards {
        let _ = e1.state.move_object(
            c,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        );
    }
    let ormos_id = first(&e1, ZoneLocation::Battlefield, ORMOS);
    let s = first(&e1, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut e1, me(), s);
    settle(&mut e1, me());
    assert!(
        !e1.state().has_left(me()),
        "Ormos prevented empty-library loss"
    );
    assert_eq!(
        e1.state()
            .object(ormos_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );

    // Out of the Tombs (with creature in graveyard): returns it instead of losing.
    let mut e2 = bench(
        1907,
        vec![tombs, draw1, bear],
        [
            Seat::with(&[TOMBS]).holding(&[DRAW1, BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut e2, me());
    let lib_cards = e2.state().zones.list(ZoneLocation::Library(me())).clone();
    for c in lib_cards {
        let _ = e2.state.move_object(
            c,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        );
    }
    let bear_id = first(&e2, ZoneLocation::Hand(me()), BEAR);
    let _ = e2.state.move_object(
        bear_id,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Setup,
    );
    let s = first(&e2, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut e2, me(), s);
    loop {
        match e2.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert!(options.contains(&bear_id));
                e2.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![bear_id],
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e2.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e2.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert!(!e2.state().has_left(me()));
    assert_eq!(e2.state().object(bear_id).unwrap().zone, Zone::Battlefield);

    // Out of the Tombs (empty graveyard): loses the game.
    let mut e3 = bench(
        1908,
        vec![tombs, draw1],
        [Seat::with(&[TOMBS]).holding(&[DRAW1]), Seat::default()],
    );
    to_main(&mut e3, me());
    let lib_cards = e3.state().zones.list(ZoneLocation::Library(me())).clone();
    for c in lib_cards {
        let _ = e3.state.move_object(
            c,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        );
    }
    let s = first(&e3, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut e3, me(), s);
    walk_until(&mut e3, |e| matches!(e.pending(), Pending::GameOver { .. }));
    assert!(
        e3.state().has_left(me()),
        "no creature to reanimate -> loses game"
    );
}

#[test]
fn archmage_ascension_optional_search_and_condition_gate() {
    const ARCHMAGE: u32 = 9615;
    const DRAW1: u32 = 9616;
    const TARGET_CARD: u32 = 9617;
    static AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawMaySearchToHand {
            condition: Some(Condition::CountersOnSelf(
                baylee_cards_dsl::counters::QUEST,
                6,
            )),
        },
    )];
    let arch = static_perm(ARCHMAGE, "Archmage", TypeSet::ENCHANTMENT, AB);
    let draw1 = draw_n_spell(DRAW1, 1, PlayerRel::You);
    let tgt = plain_creature(TARGET_CARD, "TutorTarget", "{0}", 3, 3);

    let mut engine = bench(
        1909,
        vec![arch, draw1, tgt],
        [
            Seat::with(&[ARCHMAGE]).holding(&[DRAW1, DRAW1, TARGET_CARD]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let arch_id = first(&engine, ZoneLocation::Battlefield, ARCHMAGE);
    let tgt_id = first(&engine, ZoneLocation::Hand(me()), TARGET_CARD);
    let _ = engine.state.move_object(
        tgt_id,
        ZoneLocation::Library(me()),
        ZonePosition::Bottom,
        Cause::Setup,
    );

    // Without 6 quest counters -> no offer, normal draw.
    let s1 = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), s1);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(tgt_id).unwrap().zone, Zone::Library);

    // With 6 quest counters -> YesNo offer, accept, choose tgt_id.
    crate::replacement::record_counters(
        &mut engine.state,
        arch_id,
        baylee_cards_dsl::counters::QUEST,
        6,
    );
    let s2 = first(&engine, ZoneLocation::Hand(me()), DRAW1);
    cast(&mut engine, me(), s2);
    let mut offered = false;
    loop {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                offered = true;
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert!(options.contains(&tgt_id));
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![tgt_id],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. }
                if engine.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert!(offered, "Archmage Ascension offered optional search");
    assert_eq!(engine.state().object(tgt_id).unwrap().zone, Zone::Hand);
}

#[test]
fn alms_collector_and_quantum_riddler_multi_draw_and_cr616_ordering() {
    const ALMS: u32 = 9618;
    const RIDDLER: u32 = 9619;
    const OPP_DRAW2: u32 = 9620;
    static ALMS_AB: &[AbilityDef] = &[AbilityDef::Replacement(ReplacementRule::ReplaceMultiDraw {
        who: PlayerRel::Opponent,
        min_cards: 2,
        condition: None,
        kind: MultiDrawReplacementKind::EachDrawsOne,
    })];
    static RIDDLER_AB: &[AbilityDef] =
        &[AbilityDef::Replacement(ReplacementRule::ReplaceMultiDraw {
            who: PlayerRel::You,
            min_cards: 1,
            condition: Some(Condition::HandSizeAtMost(1)),
            kind: MultiDrawReplacementKind::AddCards(1),
        })];
    let alms = card(
        ALMS,
        creature_face("Alms", "{0}", 3, 4),
        KeywordSet::EMPTY,
        ALMS_AB,
    );
    let riddler = card(
        RIDDLER,
        creature_face("Riddler", "{0}", 4, 6),
        KeywordSet::EMPTY,
        RIDDLER_AB,
    );
    let opp_draw2 = draw_n_spell(OPP_DRAW2, 2, PlayerRel::Opponent);

    // Both Alms (mine) and Riddler (theirs) apply when opponent draws 2 with 0 cards in hand.
    // Order choice is offered to the affected player (them).
    for pick_riddler_first in [false, true] {
        let mut engine = bench(
            1910,
            vec![alms, riddler, opp_draw2],
            [
                Seat::with(&[ALMS]).holding(&[OPP_DRAW2]),
                Seat::with(&[RIDDLER]),
            ],
        );
        to_main(&mut engine, me());
        let alms_id = first(&engine, ZoneLocation::Battlefield, ALMS);
        let riddler_id = first(&engine, ZoneLocation::Battlefield, RIDDLER);
        let s = first(&engine, ZoneLocation::Hand(me()), OPP_DRAW2);
        cast(&mut engine, me(), s);

        let mut asked_order = false;
        loop {
            match engine.pending().clone() {
                Pending::ChooseCards {
                    player, options, ..
                } => {
                    asked_order = true;
                    assert_eq!(
                        player,
                        them(),
                        "affected drawing player chooses CR 616.1 order"
                    );
                    assert!(options.contains(&alms_id) && options.contains(&riddler_id));
                    let pick = if pick_riddler_first {
                        riddler_id
                    } else {
                        alms_id
                    };
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseObjects {
                                objects: vec![pick],
                            },
                        )
                        .unwrap();
                }
                Pending::Priority { player, .. }
                    if engine.state().zones.stack_is_empty() && player == me() =>
                {
                    break;
                }
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected pending: {other:?}"),
            }
        }
        assert!(
            asked_order,
            "CR 616.1 multi-draw order choice must be offered"
        );
        // If Alms first: opp draw 2 -> (me draw 1, them draw 1); then Riddler applies to their 1-card draw -> them draws 2!
        // If Riddler first: opp draw 2 -> opp draw 3; then Alms replaces -> (me draw 1, them draw 1), Riddler already applied!
        assert_eq!(engine.state().zones.list(ZoneLocation::Hand(me())).len(), 1);
        let expected_opp = if pick_riddler_first { 1 } else { 2 };
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(them())).len(),
            expected_opp,
            "pick_riddler_first={pick_riddler_first}"
        );
    }
}

#[test]
fn words_of_worship_and_wilding_consume_next_draw_once() {
    const WORSHIP: u32 = 9621;
    const WILDING: u32 = 9622;
    const DRAW2: u32 = 9623;
    static WORSHIP_AB: &[AbilityDef] = &[free(
        &[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::NextDrawReplace(NextDrawReplacement::GainLife(5)),
            duration: Duration::UntilEndOfTurn,
        }],
        None,
    )];
    static WILDING_AB: &[AbilityDef] = &[free(
        &[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::NextDrawReplace(NextDrawReplacement::CreateToken(&BEAR_TOKEN)),
            duration: Duration::UntilEndOfTurn,
        }],
        None,
    )];
    let worship = static_perm(WORSHIP, "Worship", TypeSet::ENCHANTMENT, WORSHIP_AB);
    let wilding = static_perm(WILDING, "Wilding", TypeSet::ENCHANTMENT, WILDING_AB);
    let draw2 = draw_n_spell(DRAW2, 2, PlayerRel::You);

    // Words of Worship: 1st draw -> +5 life, 2nd draw -> normal draw.
    let mut e1 = bench(
        1911,
        vec![worship, draw2],
        [
            Seat::with(&[WORSHIP]).holding(&[DRAW2]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let w_id = first(&e1, ZoneLocation::Battlefield, WORSHIP);
    activate(&mut e1, me(), w_id, 0);
    settle(&mut e1, me());
    let s = first(&e1, ZoneLocation::Hand(me()), DRAW2);
    cast(&mut e1, me(), s);
    settle(&mut e1, me());
    assert_eq!(e1.state().players[0].life, 25);
    assert_eq!(e1.state().zones.list(ZoneLocation::Hand(me())).len(), 1);

    // Words of Wilding: 1st draw -> 2/2 Bear token, 2nd draw -> normal draw.
    let mut e2 = bench(
        1912,
        vec![wilding, draw2],
        [Seat::with(&[WILDING]).holding(&[DRAW2]), Seat::default()],
    );
    to_main(&mut e2, me());
    let wi_id = first(&e2, ZoneLocation::Battlefield, WILDING);
    activate(&mut e2, me(), wi_id, 0);
    settle(&mut e2, me());
    let s = first(&e2, ZoneLocation::Hand(me()), DRAW2);
    cast(&mut e2, me(), s);
    settle(&mut e2, me());
    let bears = e2
        .state()
        .battlefield_seen()
        .filter(|id| e2.state().object(*id).is_some_and(|o| !o.is_card()))
        .count();
    assert_eq!(bears, 1);
    assert_eq!(e2.state().zones.list(ZoneLocation::Hand(me())).len(), 1);
}

// =========================================================================
// 2. Moved Replacements, Game Loss & Token Replacements
// =========================================================================

#[test]
fn obstinate_baloth_and_dodecapod_opponent_discard_vs_self_discard() {
    const DODECAPOD: u32 = 9624;
    const OPP_DISCARD: u32 = 9625;
    const SELF_DISCARD: u32 = 9626;
    static DODE_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::PutOntoBattlefieldOnOpponentDiscard {
            counter_kind: Some(CounterKind::P1P1),
            counter_count: 2,
        },
    )];
    static OPP_DISCARD_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DiscardCount {
        who: PlayerRel::Opponent,
        count: Amount::Fixed(1),
    }])];
    static SELF_DISCARD_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DiscardCount {
        who: PlayerRel::You,
        count: Amount::Fixed(1),
    }])];
    let dode = card(
        DODECAPOD,
        creature_face("Dodecapod", "{0}", 3, 3),
        KeywordSet::EMPTY,
        DODE_AB,
    );
    let opp_disc = card(
        OPP_DISCARD,
        face("MindRot", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        OPP_DISCARD_AB,
    );
    let self_disc = card(
        SELF_DISCARD,
        face("SelfDisc", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SELF_DISCARD_AB,
    );

    // Opponent causes discard -> Dodecapod enters battlefield with 2 +1/+1 counters.
    let mut e1 = bench(
        1913,
        vec![dode, opp_disc],
        [
            Seat::default().holding(&[OPP_DISCARD]),
            Seat::default().holding(&[DODECAPOD]),
        ],
    );
    to_main(&mut e1, me());
    let dode_id = first(&e1, ZoneLocation::Hand(them()), DODECAPOD);
    let s = first(&e1, ZoneLocation::Hand(me()), OPP_DISCARD);
    cast(&mut e1, me(), s);
    loop {
        match e1.pending().clone() {
            Pending::ChooseCards { player, .. } => {
                e1.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![dode_id],
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e1.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e1.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(e1.state().object(dode_id).unwrap().zone, Zone::Battlefield);
    assert_eq!(e1.state().object(dode_id).unwrap().controller, them());
    assert_eq!(
        e1.state()
            .object(dode_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );

    // Self-discard -> goes to graveyard normally.
    let mut e2 = bench(
        1914,
        vec![dode, self_disc],
        [
            Seat::default().holding(&[SELF_DISCARD, DODECAPOD]),
            Seat::default(),
        ],
    );
    to_main(&mut e2, me());
    let dode_id2 = first(&e2, ZoneLocation::Hand(me()), DODECAPOD);
    let s2 = first(&e2, ZoneLocation::Hand(me()), SELF_DISCARD);
    cast(&mut e2, me(), s2);
    loop {
        match e2.pending().clone() {
            Pending::ChooseCards { player, .. } => {
                e2.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![dode_id2],
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e2.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e2.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(e2.state().object(dode_id2).unwrap().zone, Zone::Graveyard);
}

#[test]
fn xantcha_gather_specimens_and_entry_replacement_ordering() {
    const XANTCHA: u32 = 9627;
    const GATHER: u32 = 9628;
    const PRIEST: u32 = 9629;
    const DONT_BLINK: u32 = 9630;
    const REANIMATE: u32 = 9631;
    const BEAR: u32 = 9633;
    const PRIMEVAL: u32 = 9634;

    let mut xantcha_face = creature_face("Xantcha", "{0}", 5, 5);
    xantcha_face.enter_modifiers = &[EnterModifier::EntersUnderChosenOpponentControl];
    let xantcha = card(XANTCHA, xantcha_face, KeywordSet::EMPTY, &[]);

    static GATHER_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::EnterUnderYourControl(&Filter::CREATURE),
            duration: Duration::UntilEndOfTurn,
        }])];
    static PRIEST_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceBattlefieldEntry {
            filter: &Filter::CREATURE,
            predicate: EntryPredicate::NotCast,
            destination: EntryInsteadZone::Exile,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static BLINK_RULE_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceBattlefieldEntry {
            filter: &Filter::CREATURE,
            predicate: EntryPredicate::FromExileOrCastFromExile,
            destination: EntryInsteadZone::ShuffleIntoOwnersLibrary,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static PRIMEVAL_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceBattlefieldEntry {
            filter: &Filter::This,
            predicate: EntryPredicate::NotCastOrNoManaSpent,
            destination: EntryInsteadZone::Exile,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static REANIMATE_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            owner_control: false,
            counters: None,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        ))),
    )];

    let gather = card(
        GATHER,
        face("Gather", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        GATHER_AB,
    );
    let priest = card(
        PRIEST,
        creature_face("Priest", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PRIEST_AB,
    );
    let dont_blink = static_perm(DONT_BLINK, "DontBlink", TypeSet::ENCHANTMENT, BLINK_RULE_AB);
    let primeval = card(
        PRIMEVAL,
        creature_face("Primeval", "{0}", 10, 10),
        KeywordSet::EMPTY,
        PRIMEVAL_AB,
    );
    let reanimate = card(
        REANIMATE,
        face("Reanimate", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        REANIMATE_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    // 1) Xantcha enters under chosen opponent's control (3-seat explicit choice).
    let mut e1 = bench(
        1915,
        vec![xantcha],
        [
            Seat::default().holding(&[XANTCHA]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let x_id = first(&e1, ZoneLocation::Hand(me()), XANTCHA);
    cast(&mut e1, me(), x_id);
    loop {
        match e1.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, me());
                assert!(options.contains(&PlayerId::new(2)));
                e1.apply(player, PlayerAction::ChoosePlayer(PlayerId::new(2)))
                    .unwrap();
            }
            Pending::Priority { player, .. }
                if e1.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e1.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(e1.state().object(x_id).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        e1.state().object(x_id).unwrap().controller,
        PlayerId::new(2)
    );

    // 2) Gather Specimens steals opponent's entering creature, not mine.
    let mut e2 = bench(
        1916,
        vec![gather, bear],
        [
            Seat::default().holding(&[GATHER]),
            Seat::default().holding(&[BEAR]),
        ],
    );
    to_main(&mut e2, me());
    let g_id = first(&e2, ZoneLocation::Hand(me()), GATHER);
    cast(&mut e2, me(), g_id);
    settle(&mut e2, me());
    advance_to_main_of(&mut e2, them());
    let b_id = first(&e2, ZoneLocation::Hand(them()), BEAR);
    // Cast Gather on opponent's turn instead (Gather is UntilEndOfTurn).
    let mut e2b = bench(
        1917,
        vec![gather, bear],
        [
            Seat::default().holding(&[GATHER]),
            Seat::default().holding(&[BEAR]),
        ],
    );
    advance_to_main_of(&mut e2b, them());
    let b_id2 = first(&e2b, ZoneLocation::Hand(them()), BEAR);
    cast(&mut e2b, them(), b_id2);
    // Pass from them to me, respond with Gather Specimens!
    e2b.apply(them(), PlayerAction::PassPriority).unwrap();
    let g_id2 = first(&e2b, ZoneLocation::Hand(me()), GATHER);
    cast(&mut e2b, me(), g_id2);
    settle(&mut e2b, them());
    assert_eq!(e2b.state().object(b_id2).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        e2b.state().object(b_id2).unwrap().controller,
        me(),
        "Gather Specimens stole opponent's creature"
    );
    let _ = b_id;

    // 3) Containment Priest exiles uncast creature; cast creature enters normally.
    let mut e3 = bench(
        1918,
        vec![priest, reanimate, bear],
        [
            Seat::with(&[PRIEST]).holding(&[REANIMATE, BEAR, BEAR]),
            Seat::default(),
        ],
    );
    let gy_bear = first(&e3, ZoneLocation::Hand(me()), BEAR);
    let _ = e3.state.move_object(
        gy_bear,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Setup,
    );
    to_main(&mut e3, me());
    let r_id = first(&e3, ZoneLocation::Hand(me()), REANIMATE);
    cast(&mut e3, me(), r_id);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bear],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e3, me());
    assert_eq!(
        e3.state().object(gy_bear).unwrap().zone,
        Zone::Exile,
        "uncast creature exiled by Containment Priest"
    );
    let hand_bear = first(&e3, ZoneLocation::Hand(me()), BEAR);
    cast(&mut e3, me(), hand_bear);
    settle(&mut e3, me());
    assert_eq!(
        e3.state().object(hand_bear).unwrap().zone,
        Zone::Battlefield,
        "cast creature enters normally"
    );

    // 4) Primeval Spawn cast for 0 mana is exiled by its own replacement.
    let mut e4 = bench(
        1919,
        vec![primeval],
        [Seat::default().holding(&[PRIMEVAL]), Seat::default()],
    );
    to_main(&mut e4, me());
    let p_id = first(&e4, ZoneLocation::Hand(me()), PRIMEVAL);
    cast(&mut e4, me(), p_id);
    settle(&mut e4, me());
    assert_eq!(
        e4.state().object(p_id).unwrap().zone,
        Zone::Exile,
        "Primeval Spawn cast for 0 mana exiled"
    );

    // 5) CR 616.1 ordering: Containment Priest (Exile) + Don't Blink (Library) both apply to creature entering from exile!
    for pick_dont_blink in [false, true] {
        let mut e5 = bench(
            1920,
            vec![priest, dont_blink, bear],
            [
                Seat::with(&[PRIEST, DONT_BLINK]).holding(&[BEAR]),
                Seat::default(),
            ],
        );
        to_main(&mut e5, me());
        let priest_id = first(&e5, ZoneLocation::Battlefield, PRIEST);
        let blink_id = first(&e5, ZoneLocation::Battlefield, DONT_BLINK);
        let ex_bear = first(&e5, ZoneLocation::Hand(me()), BEAR);
        let _ = e5.state.move_object(
            ex_bear,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        );
        // Trigger a direct battlefield entry from exile (uncast + from exile).
        e5.state
            .entries
            .begin_event(e5.state().journal.last_seq(), Some(me()));
        let _ = e5.state.move_object(
            ex_bear,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
        let pending = crate::entry::advance(&mut e5.state).expect("CR 616.1 choice offered");
        let Pending::ChooseCards {
            player, options, ..
        } = pending
        else {
            panic!("expected ChooseCards EntryReplacement, got {pending:?}");
        };
        assert_eq!(player, me());
        assert!(options.contains(&priest_id) && options.contains(&blink_id));
        let pick = if pick_dont_blink { blink_id } else { priest_id };
        crate::entry::answer(
            &mut e5.state,
            &PlayerAction::ChooseObjects {
                objects: vec![pick],
            },
        );
        assert!(crate::entry::advance(&mut e5.state).is_none());
        let expected_zone = if pick_dont_blink {
            Zone::Library
        } else {
            Zone::Exile
        };
        assert_eq!(
            e5.state().object(ex_bear).unwrap().zone,
            expected_zone,
            "pick_dont_blink={pick_dont_blink}"
        );
    }
}

#[test]
fn due_respect_and_displaced_dinosaurs_entry_modifiers() {
    const DUE: u32 = 9635;
    const DINO: u32 = 9636;
    const ART: u32 = 9637;
    static DUE_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::PermanentsEnterTapped(&Filter::Any),
        duration: Duration::UntilEndOfTurn,
    }])];
    static DINO_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::EnterAsAdditionalTypesAndPt {
            filter: &Filter::ARTIFACT,
            add_types: TypeSet::CREATURE,
            add_subtypes: &[creature_type::DINOSAUR],
            power: 7,
            toughness: 7,
        },
    )];
    let due = card(
        DUE,
        face("DueRespect", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DUE_AB,
    );
    let dino = card(
        DINO,
        creature_face("DisplacedDinos", "{0}", 7, 7),
        KeywordSet::EMPTY,
        DINO_AB,
    );
    let art = card(
        ART,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1921,
        vec![due, dino, art],
        [Seat::with(&[DINO]).holding(&[DUE, ART]), Seat::default()],
    );
    to_main(&mut engine, me());
    let d_id = first(&engine, ZoneLocation::Hand(me()), DUE);
    cast(&mut engine, me(), d_id);
    settle(&mut engine, me());

    let a_id = first(&engine, ZoneLocation::Hand(me()), ART);
    cast(&mut engine, me(), a_id);
    let mut asked_entry_order = false;
    loop {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                asked_entry_order = true;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. }
                if engine.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert!(
        asked_entry_order,
        "competing Due Respect + Displaced Dinosaurs offered CR 616.1 order"
    );

    let obj = engine.state().object(a_id).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "Due Respect tapped entering artifact"
    );
    let ch = obj.characteristics();
    assert!(ch.types.contains(TypeSet::CREATURE) && ch.types.contains(TypeSet::ARTIFACT));
    assert!(ch.subtypes.contains(creature_type::DINOSAUR));
    assert_eq!((ch.power, ch.toughness), (Some(7), Some(7)));
}

#[test]
fn graveyard_redirects_and_exile_riders_and_kumanos_blessing() {
    const WHEEL: u32 = 9638;
    const KALITAS: u32 = 9639;
    const SLIME: u32 = 9640;
    const NEXUS: u32 = 9641;
    const COSMIC: u32 = 9642;
    const KILL: u32 = 9643;
    const BEAR: u32 = 9644;
    const KUMANO: u32 = 9645;
    const PINGER: u32 = 9646;

    static WHEEL_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::RedirectGraveyard {
            filter: &Filter::Any,
            origin: None,
            owner: Some(PlayerRel::You),
            controller_lki: None,
            destination: GraveyardRedirectDest::BottomOfLibrary,
            reveal: true,
        },
    )];
    static KALITAS_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyardWithRider {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: Some(PlayerRel::Opponent),
            damaged_by: DamagedByScope::Any,
            link_to_source: false,
            rider: GraveyardExileRider::CreateToken {
                token: &ZOMBIE_TOKEN,
                count: 1,
            },
        },
    )];
    static SLIME_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyardWithRider {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: Some(PlayerRel::Opponent),
            damaged_by: DamagedByScope::Any,
            link_to_source: false,
            rider: GraveyardExileRider::PowerCountersOnSource(CounterKind::P1P1),
        },
    )];
    static NEXUS_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyardWithRider {
            filter: &Filter::This,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: None,
            damaged_by: DamagedByScope::Any,
            link_to_source: false,
            rider: GraveyardExileRider::ExtraTurn,
        },
    )];
    static COSMIC_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::ExileMatchingInsteadOfGraveyard {
                filter: &Filter::Any,
                origin: Some(ZoneRef::Battlefield),
                controller_lki: Some(PlayerRel::You),
                rider: Some(GraveyardExileRider::ReturnAtNextEndStep),
            },
            duration: Duration::UntilEndOfTurn,
        }])];
    static KILL_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )];
    static KUMANO_AB: &[AbilityDef] = &[
        baylee_cards_dsl::spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: None,
            damaged_by: DamagedByScope::AttachedCreatureThisTurn,
            link_to_source: false,
            counter: None,
        }),
    ];
    static PING_CREATURE: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let wheel = static_perm(WHEEL, "Wheel", TypeSet::ENCHANTMENT, WHEEL_AB);
    let kalitas = card(
        KALITAS,
        creature_face("Kalitas", "{0}", 3, 4),
        KeywordSet::EMPTY,
        KALITAS_AB,
    );
    let slime = card(
        SLIME,
        creature_face("Slime", "{0}", 1, 1),
        KeywordSet::EMPTY,
        SLIME_AB,
    );
    let nexus = static_perm(NEXUS, "Nexus", TypeSet::ARTIFACT, NEXUS_AB);
    let cosmic = card(
        COSMIC,
        face("Cosmic", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COSMIC_AB,
    );
    let kill = card(
        KILL,
        face("Kill", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        KILL_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 4, 4);
    let mut kumano_face = face("KumanosBlessing", "{0}", TypeSet::ENCHANTMENT);
    kumano_face.subtypes = &[baylee_core::generated::subtypes::enchantment::AURA];
    let kumano = card(KUMANO, kumano_face, KeywordSet::EMPTY, KUMANO_AB);
    let pinger = card(
        PINGER,
        creature_face("Pinger", "{0}", 1, 1),
        KeywordSet::EMPTY,
        PING_CREATURE,
    );

    // 1) Wheel: dying creature goes to bottom of library (revealed), not graveyard.
    let mut e1 = bench(
        1922,
        vec![wheel, bear, kill],
        [Seat::with(&[WHEEL, BEAR]).holding(&[KILL]), Seat::default()],
    );
    to_main(&mut e1, me());
    let b1 = first(&e1, ZoneLocation::Battlefield, BEAR);
    let k1 = first(&e1, ZoneLocation::Hand(me()), KILL);
    cast(&mut e1, me(), k1);
    if let Pending::ChooseTargets { player, .. } = e1.pending().clone() {
        e1.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![b1],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e1, me());
    assert_eq!(e1.state().object(b1).unwrap().zone, Zone::Library);
    assert_eq!(
        e1.state().object(k1).unwrap().zone,
        Zone::Library,
        "Wheel also bottomed resolving Kill spell"
    );
    let lib = e1.state().zones.list(ZoneLocation::Library(me()));
    assert_eq!(lib[0], k1, "resolving Kill went to bottom after Bear");
    assert_eq!(lib[1], b1, "destroyed Bear went to bottom of library");

    // 2) Kalitas: opponent's dying creature exiled + 2/2 Zombie created for me.
    let mut e2 = bench(
        1923,
        vec![kalitas, bear, kill],
        [Seat::with(&[KALITAS]).holding(&[KILL]), Seat::with(&[BEAR])],
    );
    to_main(&mut e2, me());
    let b2 = first(&e2, ZoneLocation::Battlefield, BEAR);
    let k2 = first(&e2, ZoneLocation::Hand(me()), KILL);
    cast(&mut e2, me(), k2);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![b2],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e2, me());
    assert_eq!(e2.state().object(b2).unwrap().zone, Zone::Exile);
    let zombies = e2
        .state()
        .battlefield_seen()
        .filter(|id| {
            e2.state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == me())
        })
        .count();
    assert_eq!(zombies, 1);

    // 3) Ravenous Slime: opponent's dying 4/4 exiled + 4 +1/+1 counters on Slime.
    let mut e3 = bench(
        1924,
        vec![slime, bear, kill],
        [Seat::with(&[SLIME]).holding(&[KILL]), Seat::with(&[BEAR])],
    );
    to_main(&mut e3, me());
    let sl = first(&e3, ZoneLocation::Battlefield, SLIME);
    let b3 = first(&e3, ZoneLocation::Battlefield, BEAR);
    let k3 = first(&e3, ZoneLocation::Hand(me()), KILL);
    cast(&mut e3, me(), k3);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![b3],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e3, me());
    assert_eq!(e3.state().object(b3).unwrap().zone, Zone::Exile);
    assert_eq!(
        e3.state()
            .object(sl)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );

    // 4) Ugin's Nexus: destroyed Nexus is exiled + grants extra turn.
    let mut e4 = bench(
        1925,
        vec![nexus, kill],
        [Seat::with(&[NEXUS]).holding(&[KILL]), Seat::default()],
    );
    to_main(&mut e4, me());
    let nx = first(&e4, ZoneLocation::Battlefield, NEXUS);
    let k4 = first(&e4, ZoneLocation::Hand(me()), KILL);
    cast(&mut e4, me(), k4);
    if let Pending::ChooseTargets { player, .. } = e4.pending().clone() {
        e4.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![nx],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e4, me());
    assert_eq!(e4.state().object(nx).unwrap().zone, Zone::Exile);
    assert_eq!(e4.state().extra_turns.front().copied(), Some(me()));

    // 5) Cosmic Intervention: dying permanent exiled, then returned at end step!
    let mut e5 = bench(
        1926,
        vec![cosmic, bear, kill],
        [
            Seat::with(&[BEAR]).holding(&[COSMIC, KILL]),
            Seat::default(),
        ],
    );
    to_main(&mut e5, me());
    let b5 = first(&e5, ZoneLocation::Battlefield, BEAR);
    let c5 = first(&e5, ZoneLocation::Hand(me()), COSMIC);
    cast(&mut e5, me(), c5);
    settle(&mut e5, me());
    let k5 = first(&e5, ZoneLocation::Hand(me()), KILL);
    cast(&mut e5, me(), k5);
    if let Pending::ChooseTargets { player, .. } = e5.pending().clone() {
        e5.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![b5],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e5, me());
    assert_eq!(e5.state().object(b5).unwrap().zone, Zone::Exile);
    advance_to_main_of(&mut e5, them());
    assert_eq!(
        e5.state().object(b5).unwrap().zone,
        Zone::Battlefield,
        "Cosmic Intervention returned Bear at end step"
    );

    // 6) Kumano's Blessing: creature damaged by enchanted creature is exiled on death.
    let mut e6 = bench(
        1927,
        vec![kumano, pinger, bear, kill],
        [
            Seat::with(&[PINGER]).holding(&[KUMANO, KILL]),
            Seat::with(&[BEAR, BEAR]),
        ],
    );
    to_main(&mut e6, me());
    let ku = first(&e6, ZoneLocation::Hand(me()), KUMANO);
    let pi = first(&e6, ZoneLocation::Battlefield, PINGER);
    cast(&mut e6, me(), ku);
    if let Pending::ChooseTargets { player, .. } = e6.pending().clone() {
        e6.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![pi],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e6, me());
    let opp_bears = objects(&e6, ZoneLocation::Battlefield, BEAR);
    let damaged_bear = opp_bears[0];
    let untouched_bear = opp_bears[1];
    // Ping damaged_bear with enchanted pinger.
    activate(&mut e6, me(), pi, 0);
    if let Pending::ChooseTargets { player, .. } = e6.pending().clone() {
        e6.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![damaged_bear],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e6, me());
    // Destroy damaged_bear -> exiled!
    let k6 = first(&e6, ZoneLocation::Hand(me()), KILL);
    cast(&mut e6, me(), k6);
    if let Pending::ChooseTargets { player, .. } = e6.pending().clone() {
        e6.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![damaged_bear],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e6, me());
    assert_eq!(e6.state().object(damaged_bear).unwrap().zone, Zone::Exile);
    // Untouched bear dies normally -> graveyard!
    crate::sba::destroy(&mut e6.state, untouched_bear);
    assert_eq!(
        e6.state().object(untouched_bear).unwrap().zone,
        Zone::Graveyard
    );
}

#[test]
fn stunning_reversal_replaces_loss_once_and_crafty_cutpurse_kaya_tokens() {
    const STUNNING: u32 = 9647;
    const LOSE_SPELL: u32 = 9648;
    const CUTPURSE: u32 = 9649;
    const KAYA: u32 = 9650;
    const MAKE_TOKENS: u32 = 9651;

    static STUN_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::ReplaceNextGameLoss {
            draw: 7,
            set_life: 1
        },
        duration: Duration::UntilEndOfTurn,
    }])];
    static LOSE_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::LoseGame])];
    static CUTPURSE_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::StealOpponentTokens,
            duration: Duration::UntilEndOfTurn,
        }])];
    static KAYA_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::MultiplyTokensCreated {
            who: PlayerRel::You,
            factor: 2
        },
        duration: Duration::UntilEndOfTurn,
    }])];
    static MAKE_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::CreateTokenN {
        token: &BEAR_TOKEN,
        amount: Amount::Fixed(2),
    }])];

    let stun = card(
        STUNNING,
        face("Stunning", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        STUN_AB,
    );
    let lose = card(
        LOSE_SPELL,
        face("Lose", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        LOSE_AB,
    );
    let cut = card(
        CUTPURSE,
        face("Cutpurse", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CUTPURSE_AB,
    );
    let kaya = card(
        KAYA,
        face("Kaya", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        KAYA_AB,
    );
    let make = card(
        MAKE_TOKENS,
        face("MakeTokens", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MAKE_AB,
    );

    // Stunning Reversal saves from first loss (life=1, draws 7), second loss eliminates!
    let mut e1 = bench(
        1928,
        vec![stun, lose],
        [
            Seat::default()
                .holding(&[STUNNING, LOSE_SPELL, LOSE_SPELL])
                .at(20),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let st = first(&e1, ZoneLocation::Hand(me()), STUNNING);
    cast(&mut e1, me(), st);
    settle(&mut e1, me());
    let l1 = first(&e1, ZoneLocation::Hand(me()), LOSE_SPELL);
    cast(&mut e1, me(), l1);
    settle(&mut e1, me());
    assert!(!e1.state().has_left(me()));
    assert_eq!(e1.state().players[0].life, 1);
    assert_eq!(e1.state().zones.list(ZoneLocation::Hand(me())).len(), 8); // 1 remaining LOSE + 7 drawn
    let l2 = first(&e1, ZoneLocation::Hand(me()), LOSE_SPELL);
    cast(&mut e1, me(), l2);
    walk_until(&mut e1, |e| matches!(e.pending(), Pending::GameOver { .. }));
    assert!(e1.state().has_left(me()), "second loss is not replaced");

    // Crafty Cutpurse + Kaya: opponent creates 2 tokens -> stolen under my control AND doubled to 4!
    let mut e2 = bench(
        1929,
        vec![cut, kaya, make],
        [
            Seat::default().holding(&[CUTPURSE, KAYA]),
            Seat::default().holding(&[MAKE_TOKENS]),
        ],
    );
    advance_to_main_of(&mut e2, them());
    let mk = first(&e2, ZoneLocation::Hand(them()), MAKE_TOKENS);
    cast(&mut e2, them(), mk);
    e2.apply(them(), PlayerAction::PassPriority).unwrap();
    let cp = first(&e2, ZoneLocation::Hand(me()), CUTPURSE);
    cast(&mut e2, me(), cp);
    e2.apply(me(), PlayerAction::PassPriority).unwrap();
    e2.apply(them(), PlayerAction::PassPriority).unwrap();
    // Now cast Kaya before MakeTokens resolves!
    e2.apply(them(), PlayerAction::PassPriority).unwrap();
    let ky = first(&e2, ZoneLocation::Hand(me()), KAYA);
    cast(&mut e2, me(), ky);
    settle(&mut e2, them());
    let my_tokens = e2
        .state()
        .battlefield_seen()
        .filter(|id| {
            e2.state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == me())
        })
        .count();
    let their_tokens = e2
        .state()
        .battlefield_seen()
        .filter(|id| {
            e2.state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == them())
        })
        .count();
    assert_eq!(
        my_tokens, 4,
        "stolen 2 tokens doubled to 4 under my control"
    );
    assert_eq!(their_tokens, 0);
}

// =========================================================================
// 3. Damage Prevention, Redirection, Scaling & Riders
// =========================================================================

#[test]
fn equal_treatment_set_to_and_add_damage_dynamic() {
    const EQUAL: u32 = 9652;
    const FIREPOWER: u32 = 9653;
    const BOLT5: u32 = 9654;

    static EQUAL_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::ReplaceDamage {
                sources: &Filter::Any,
                recipients: DamageRecipients::Any,
                combat: None,
                replacement: DamageAmountReplacement::SetTo(2),
            },
            duration: Duration::UntilEndOfTurn,
        }])];
    static FIRE_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::AddDamageDynamic {
            sources: &Filter::ControlledByYou,
            extra: Amount::CountersOnSource(CounterKind::Charge),
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::Any,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static BOLT5_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(5),
        target: TargetSpec::Player(PlayerRel::Opponent),
    }])];

    let equal = card(
        EQUAL,
        face("Equal", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        EQUAL_AB,
    );
    let fire = static_perm(FIREPOWER, "Firepower", TypeSet::ENCHANTMENT, FIRE_AB);
    let bolt5 = card(
        BOLT5,
        face("Bolt5", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT5_AB,
    );

    // Equal Treatment alone: 5 -> 2.
    let mut e1 = bench(
        1930,
        vec![equal, bolt5],
        [
            Seat::default().holding(&[EQUAL, BOLT5]),
            Seat::default().at(20),
        ],
    );
    to_main(&mut e1, me());
    let eq = first(&e1, ZoneLocation::Hand(me()), EQUAL);
    cast(&mut e1, me(), eq);
    settle(&mut e1, me());
    let b = first(&e1, ZoneLocation::Hand(me()), BOLT5);
    cast(&mut e1, me(), b);
    settle(&mut e1, me());
    assert_eq!(
        e1.state().players[1].life,
        18,
        "Equal Treatment replaced 5 with 2"
    );

    // AddDamageDynamic with 3 charge counters: 5 + 3 = 8.
    let mut e2 = bench(
        1931,
        vec![fire, bolt5],
        [
            Seat::with(&[FIREPOWER]).holding(&[BOLT5]),
            Seat::default().at(20),
        ],
    );
    to_main(&mut e2, me());
    let fp = first(&e2, ZoneLocation::Battlefield, FIREPOWER);
    crate::replacement::record_counters(&mut e2.state, fp, CounterKind::Charge, 3);
    let b = first(&e2, ZoneLocation::Hand(me()), BOLT5);
    cast(&mut e2, me(), b);
    settle(&mut e2, me());
    assert_eq!(
        e2.state().players[1].life,
        12,
        "5 + 3 dynamic bonus = 8 damage"
    );
}

#[test]
fn prevent_damage_removing_one_counter_and_unpreventable_cr615_12() {
    const KNIGHT: u32 = 9655;
    const BOLT4: u32 = 9656;
    const UNPREV4: u32 = 9657;

    static KNIGHT_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::PreventDamageRemovingOneCounter {
            kind: CounterKind::P1P1,
            require_counter: true,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static BOLT4_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static UNPREV4_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamageWithoutPrevention {
            amount: Amount::Fixed(4),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let knight = card(
        KNIGHT,
        creature_face("Knight", "{0}", 3, 3),
        KeywordSet::EMPTY,
        KNIGHT_AB,
    );
    let bolt4 = card(
        BOLT4,
        face("Bolt4", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT4_AB,
    );
    let unprev4 = card(
        UNPREV4,
        face("Unprev4", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        UNPREV4_AB,
    );

    let mut engine = bench(
        1932,
        vec![knight, bolt4, unprev4],
        [
            Seat::with(&[KNIGHT]).holding(&[BOLT4, UNPREV4]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let k = first(&engine, ZoneLocation::Battlefield, KNIGHT);
    crate::replacement::record_counters(&mut engine.state, k, CounterKind::P1P1, 3);

    // Preventable 4 damage -> prevented, removes ONLY 1 counter (3 -> 2), 0 damage marked.
    let b = first(&engine, ZoneLocation::Hand(me()), BOLT4);
    cast(&mut engine, me(), b);
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![k],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(k)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(engine.state().object(k).unwrap().damage, 0);

    // Unpreventable 4 damage -> CR 615.12: damage NOT prevented (4 marked), still removes 1 counter (2 -> 1)!
    let u = first(&engine, ZoneLocation::Hand(me()), UNPREV4);
    cast(&mut engine, me(), u);
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![k],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    // 3/3 with 1 +1/+1 counter is 4/4 with 4 damage -> dies to SBA (or check counters before/death).
    assert_eq!(
        engine.state().object(k).unwrap().zone,
        Zone::Graveyard,
        "unpreventable 4 damage killed 4/4 after counter removal"
    );
}

#[test]
fn palisade_giant_treacherous_link_and_temple_altisaur() {
    const PALISADE: u32 = 9658;
    const LINK: u32 = 9659;
    const ALTISAUR: u32 = 9660;
    const BEAR: u32 = 9661;
    const BOLT5_PLAYER: u32 = 9662;
    const BOLT5_CREATURE: u32 = 9663;

    static PALISADE_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::RedirectDamageToAffected {
            sources: &Filter::Any,
            recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::You),
            combat: None,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static LINK_AB: &[AbilityDef] = &[
        baylee_cards_dsl::spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::AttachedToBySource,
            modifier: Modifier::RedirectDamageToAffectedController,
            condition: None,
            source_zone: ZoneRef::Battlefield,
        }),
    ];
    static ALTISAUR_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::PreventAllButOneDamage {
            sources: &Filter::Any,
            recipients: DamageRecipients::Permanents(&Filter::YOUR_CREATURE),
            combat: None,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static BOLT_P_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(5),
        target: TargetSpec::Player(PlayerRel::You),
    }])];
    static BOLT_C_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(5),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let palisade = card(
        PALISADE,
        creature_face("Palisade", "{0}", 2, 7),
        KeywordSet::EMPTY,
        PALISADE_AB,
    );
    let mut link_face = face("TreacherousLink", "{0}", TypeSet::ENCHANTMENT);
    link_face.subtypes = &[baylee_core::generated::subtypes::enchantment::AURA];
    let link = card(LINK, link_face, KeywordSet::EMPTY, LINK_AB);
    let altisaur = card(
        ALTISAUR,
        creature_face("Altisaur", "{0}", 3, 4),
        KeywordSet::EMPTY,
        ALTISAUR_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 5, 5);
    let bolt_p = card(
        BOLT5_PLAYER,
        face("BoltP", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT_P_AB,
    );
    let bolt_c = card(
        BOLT5_CREATURE,
        face("BoltC", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT_C_AB,
    );

    // 1) Palisade Giant redirects player damage to itself.
    let mut e1 = bench(
        1933,
        vec![palisade, bolt_p],
        [
            Seat::with(&[PALISADE]).holding(&[BOLT5_PLAYER]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let pg = first(&e1, ZoneLocation::Battlefield, PALISADE);
    let bp = first(&e1, ZoneLocation::Hand(me()), BOLT5_PLAYER);
    cast(&mut e1, me(), bp);
    settle(&mut e1, me());
    assert_eq!(e1.state().players[0].life, 20);
    assert_eq!(e1.state().object(pg).unwrap().damage, 5);

    // 2) Treacherous Link redirects damage to enchanted creature -> its controller!
    let mut e2 = bench(
        1934,
        vec![link, bear, bolt_c],
        [
            Seat::default().holding(&[LINK, BOLT5_CREATURE]),
            Seat::with(&[BEAR]).at(20),
        ],
    );
    to_main(&mut e2, me());
    let lk = first(&e2, ZoneLocation::Hand(me()), LINK);
    let br = first(&e2, ZoneLocation::Battlefield, BEAR);
    cast(&mut e2, me(), lk);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![br],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e2, me());
    let bc = first(&e2, ZoneLocation::Hand(me()), BOLT5_CREATURE);
    cast(&mut e2, me(), bc);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![br],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e2, me());
    assert_eq!(e2.state().object(br).unwrap().damage, 0);
    assert_eq!(e2.state().players[1].life, 15);

    // 3) Temple Altisaur reduces 5 damage to 1!
    let mut e3 = bench(
        1935,
        vec![altisaur, bear, bolt_c],
        [
            Seat::with(&[ALTISAUR, BEAR]).holding(&[BOLT5_CREATURE]),
            Seat::default(),
        ],
    );
    to_main(&mut e3, me());
    let br3 = first(&e3, ZoneLocation::Battlefield, BEAR);
    let bc3 = first(&e3, ZoneLocation::Hand(me()), BOLT5_CREATURE);
    cast(&mut e3, me(), bc3);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![br3],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e3, me());
    assert_eq!(
        e3.state().object(br3).unwrap().damage,
        1,
        "PreventAllButOneDamage left 1 damage"
    );
}

#[test]
fn scoped_riders_vigor_nine_lives_swans_szadek_weeping_angel_comeuppance() {
    const VIGOR: u32 = 9664;
    const NINE_LIVES: u32 = 9665;
    const SWANS: u32 = 9666;
    const SZADEK: u32 = 9667;
    const ANGEL: u32 = 9668;
    const COMEUPPANCE: u32 = 9669;
    const SWEEP3: u32 = 9670;
    const BEAR: u32 = 9671;
    const BOLT3_CREATURE: u32 = 9672;
    const BOLT3_OPP: u32 = 9673;

    static VIGOR_RIDER: &[Effect; 1] = &[Effect::AddCounterDamagedObject {
        kind: CounterKind::P1P1,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }];
    static NINE_RIDER: &[Effect; 1] = &[Effect::AddCounterSource {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1),
    }];
    static SWANS_RIDER: &[Effect; 1] = &[Effect::DrawCardsFor {
        who: PlayerRel::ControllerOfEvent,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }];
    static SZADEK_OPS: &[Effect] = &[
        Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: Amount::ThisWay(ThisWay::DamagePrevented),
        },
        Effect::Mill {
            target: PlayerRel::DamagedPlayer,
            amount: Amount::ThisWay(ThisWay::DamagePrevented),
        },
    ];
    static ANGEL_RIDER: &[Effect; 1] = &[Effect::ShuffleDamagedObjectIntoLibrary];
    static COME_RIDER: &[Effect; 1] = &[Effect::DealDamageToEventSourceOrController {
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
        creature_source_takes_damage: true,
    }];

    static YOUR_OTHER_CREATURE: Filter = Filter::And(&[Filter::YOUR_CREATURE, Filter::Another]);
    static VIGOR_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Permanents(&YOUR_OTHER_CREATURE),
            combat: None,
            is_replacement: false,
            only_if_prevented: true,
            scope: DamageRiderScope::PerTarget,
            then: DamageRider::new(VIGOR_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static NINE_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Players(PlayerRel::You),
            combat: None,
            is_replacement: false,
            only_if_prevented: false,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(NINE_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static SWANS_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Permanents(&Filter::This),
            combat: None,
            is_replacement: false,
            only_if_prevented: true,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(SWANS_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static SZADEK_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::This,
            recipients: DamageRecipients::Players(PlayerRel::EachPlayer),
            combat: Some(true),
            is_replacement: true,
            only_if_prevented: true,
            scope: DamageRiderScope::PerTarget,
            then: DamageRider::sequence(SZADEK_OPS),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static ANGEL_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::This,
            recipients: DamageRecipients::Permanents(&Filter::CREATURE),
            combat: Some(true),
            is_replacement: true,
            only_if_prevented: true,
            scope: DamageRiderScope::PerTarget,
            then: DamageRider::new(ANGEL_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static COME_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::ControlledByOpponent,
            recipients: DamageRecipients::Players(PlayerRel::You),
            combat: None,
            is_replacement: false,
            only_if_prevented: true,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(COME_RIDER),
        },
        duration: Duration::UntilEndOfTurn,
    }])];
    static SWEEP3_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DealDamageEach {
        amount: Amount::Fixed(3),
        filter: &Filter::CREATURE,
    }])];
    static BOLT3_C_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static BOLT3_O_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::Player(PlayerRel::Opponent),
    }])];

    let vigor = card(
        VIGOR,
        creature_face("Vigor", "{0}", 6, 6),
        KeywordSet::EMPTY,
        VIGOR_AB,
    );
    let nine = static_perm(NINE_LIVES, "NineLives", TypeSet::ENCHANTMENT, NINE_AB);
    let swans = card(
        SWANS,
        creature_face("Swans", "{0}", 4, 3),
        KeywordSet::EMPTY,
        SWANS_AB,
    );
    let szadek = card(
        SZADEK,
        creature_face("Szadek", "{0}", 5, 5),
        KeywordSet::EMPTY,
        SZADEK_AB,
    );
    let angel = card(
        ANGEL,
        creature_face("WeepingAngel", "{0}", 3, 3),
        KeywordSet::EMPTY,
        ANGEL_AB,
    );
    let come = card(
        COMEUPPANCE,
        face("Comeuppance", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COME_AB,
    );
    let sweep3 = card(
        SWEEP3,
        face("Sweep3", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SWEEP3_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);
    let bolt3_c = card(
        BOLT3_CREATURE,
        face("Bolt3C", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT3_C_AB,
    );
    let bolt3_o = card(
        BOLT3_OPP,
        face("Bolt3O", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT3_O_AB,
    );

    // 1) Vigor: sweep 3 damages Vigor (3 marked) while 2 Bears take 0 damage and get 3 +1/+1 counters each!
    let mut e1 = bench(
        1936,
        vec![vigor, bear, sweep3],
        [
            Seat::with(&[VIGOR, BEAR, BEAR]).holding(&[SWEEP3]),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let vg = first(&e1, ZoneLocation::Battlefield, VIGOR);
    let bears = objects(&e1, ZoneLocation::Battlefield, BEAR);
    let sw = first(&e1, ZoneLocation::Hand(me()), SWEEP3);
    cast(&mut e1, me(), sw);
    settle(&mut e1, me());
    assert_eq!(e1.state().object(vg).unwrap().damage, 3);
    for b in bears {
        assert_eq!(e1.state().object(b).unwrap().damage, 0);
        assert_eq!(
            e1.state()
                .object(b)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            3
        );
    }

    // 2) Nine Lives: 2 unblocked attackers -> 2 Incarnation counters (PerSource)!
    let mut e2 = bench(
        1937,
        vec![nine, bear],
        [Seat::with(&[BEAR, BEAR]), Seat::with(&[NINE_LIVES]).at(20)],
    );
    to_declare_attackers(&mut e2, me());
    let atks = objects(&e2, ZoneLocation::Battlefield, BEAR);
    e2.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: atks
                .into_iter()
                .map(|id| (id, Defender::Player(them())))
                .collect(),
        },
    )
    .unwrap();
    walk_until(&mut e2, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    let nl = first(&e2, ZoneLocation::Battlefield, NINE_LIVES);
    assert_eq!(e2.state().players[1].life, 20);
    assert_eq!(
        e2.state()
            .object(nl)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2,
        "PerSource puts 2 counters for 2 attackers"
    );

    // 3) Swans of Bryn Argoll: opponent bolts Swans for 3 -> prevented, opponent draws 3!
    let mut e3 = bench(
        1938,
        vec![swans, bolt3_c],
        [
            Seat::with(&[SWANS]),
            Seat::default().holding(&[BOLT3_CREATURE]),
        ],
    );
    advance_to_main_of(&mut e3, them());
    let hand_before = e3.state().zones.list(ZoneLocation::Hand(them())).len();
    let sw_id = first(&e3, ZoneLocation::Battlefield, SWANS);
    let bc = first(&e3, ZoneLocation::Hand(them()), BOLT3_CREATURE);
    cast(&mut e3, them(), bc);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![sw_id],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e3, them());
    assert_eq!(e3.state().object(sw_id).unwrap().damage, 0);
    assert_eq!(
        e3.state().zones.list(ZoneLocation::Hand(them())).len(),
        hand_before - 1 + 3,
        "damaging source's controller drew 3"
    );

    // 4) Szadek (multi-op sequence): unblocked 5/5 puts 5 +1/+1 counters on Szadek AND mills defender 5!
    let mut e4 = bench(
        1939,
        vec![szadek],
        [Seat::with(&[SZADEK]), Seat::default().at(20)],
    );
    to_declare_attackers(&mut e4, me());
    let sz = first(&e4, ZoneLocation::Battlefield, SZADEK);
    e4.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(sz, Defender::Player(them()))],
        },
    )
    .unwrap();
    walk_until(&mut e4, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(e4.state().players[1].life, 20);
    assert_eq!(
        e4.state()
            .object(sz)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );
    assert_eq!(
        e4.state().zones.list(ZoneLocation::Graveyard(them())).len(),
        5
    );

    // 5) Weeping Angel: blocked 3/3 replaces combat damage to blocker with shuffling blocker into library!
    let mut e5 = bench(
        1940,
        vec![angel, bear],
        [Seat::with(&[ANGEL]), Seat::with(&[BEAR])],
    );
    to_declare_attackers(&mut e5, me());
    let wa = first(&e5, ZoneLocation::Battlefield, ANGEL);
    let blk = first(&e5, ZoneLocation::Battlefield, BEAR);
    e5.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(wa, Defender::Player(them()))],
        },
    )
    .unwrap();
    walk_until(
        &mut e5,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    e5.apply(
        them(),
        PlayerAction::DeclareBlockers {
            blockers: vec![(blk, wa)],
        },
    )
    .unwrap();
    walk_until(&mut e5, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        e5.state().object(blk).unwrap().zone,
        Zone::Library,
        "Weeping Angel shuffled blocker into library"
    );

    // 6) Comeuppance: creature attacker takes its own damage; noncreature burn spell hits its controller!
    let mut e6 = bench(
        1941,
        vec![come, bear, bolt3_o],
        [
            Seat::with(&[BEAR]).holding(&[BOLT3_OPP]).at(20),
            Seat::default().holding(&[COMEUPPANCE]).at(20),
        ],
    );
    to_declare_attackers(&mut e6, me());
    let atk = first(&e6, ZoneLocation::Battlefield, BEAR);
    e6.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(atk, Defender::Player(them()))],
        },
    )
    .unwrap();
    // Cast Comeuppance before combat damage!
    walk_until(
        &mut e6,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == them()),
    );
    let cm = first(&e6, ZoneLocation::Hand(them()), COMEUPPANCE);
    cast(&mut e6, them(), cm);
    walk_until(&mut e6, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        e6.state().players[1].life,
        20,
        "Comeuppance prevented combat damage"
    );
    assert_eq!(
        e6.state().object(atk).unwrap().zone,
        Zone::Graveyard,
        "attacking 2/2 took 2 reflected damage and died"
    );
    // Now cast noncreature Bolt3O at opponent -> Comeuppance reflects 3 damage to ME (spell controller)!
    let bo = first(&e6, ZoneLocation::Hand(me()), BOLT3_OPP);
    cast(&mut e6, me(), bo);
    settle(&mut e6, me());
    assert_eq!(e6.state().players[1].life, 20);
    assert_eq!(
        e6.state().players[0].life,
        17,
        "noncreature damage reflected to spell controller"
    );
}

#[test]
fn wolverine_heals_prior_damage_on_new_damage() {
    const WOLVERINE: u32 = 9674;
    const BOLT3: u32 = 9675;
    static WOLV_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::HealOtherDamageWhenDealtDamage,
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static BOLT3_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let wolv = card(
        WOLVERINE,
        creature_face("Wolverine", "{0}", 4, 4),
        KeywordSet::EMPTY,
        WOLV_AB,
    );
    let bolt3 = card(
        BOLT3,
        face("Bolt3", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT3_AB,
    );

    let mut engine = bench(
        1942,
        vec![wolv, bolt3],
        [
            Seat::with(&[WOLVERINE]).holding(&[BOLT3, BOLT3]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let w = first(&engine, ZoneLocation::Battlefield, WOLVERINE);
    for _ in 0..2 {
        let b = first(&engine, ZoneLocation::Hand(me()), BOLT3);
        cast(&mut engine, me(), b);
        if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![w],
                        players: vec![],
                    },
                )
                .unwrap();
        }
        settle(&mut engine, me());
    }
    assert_eq!(
        engine.state().object(w).unwrap().zone,
        Zone::Battlefield,
        "Wolverine healed first 3 before taking second 3"
    );
    assert_eq!(engine.state().object(w).unwrap().damage, 3);
}

#[test]
fn redirect_next_damage_to_outrider_and_harms_way_and_deflecting_palm_new_way_forward() {
    const OUTRIDER: u32 = 9676;
    const HARMS_WAY: u32 = 9677;
    const NEW_WAY: u32 = 9678;
    const BEAR: u32 = 9679;
    const PINGER3: u32 = 9680;

    static OUTRIDER_AB: &[AbilityDef] = &[free(
        &[Effect::RedirectNextDamageTo {
            from: TargetSpec::ThisObject,
            choose_source: None,
            amount: Amount::Fixed(1),
            to: TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )];
    static HARMS_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::RedirectNextDamageTo {
            from: TargetSpec::PlayerOrObject(&Filter::ControlledByYou),
            choose_source: Some(&Filter::Any),
            amount: Amount::Fixed(2),
            to: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];
    static NEW_WAY_OPS: &[Effect] = &[
        Effect::DealDamageToEventSourceOrController {
            amount: Amount::ThisWay(ThisWay::DamagePrevented),
            creature_source_takes_damage: false,
        },
        Effect::DrawCardsFor {
            who: PlayerRel::You,
            amount: Amount::ThisWay(ThisWay::DamagePrevented),
        },
    ];
    static NEW_WAY_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[
        Effect::PreventNextFromChosenSourceWithThen {
            target: TargetSpec::Player(PlayerRel::You),
            sources: &Filter::Any,
            combat_only: false,
            then: DamageRider::sequence(NEW_WAY_OPS),
        }
    ])];
    static PING3_AB: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }],
        Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];

    let outrider = card(
        OUTRIDER,
        creature_face("Outrider", "{0}", 2, 4),
        KeywordSet::EMPTY,
        OUTRIDER_AB,
    );
    let harms = card(
        HARMS_WAY,
        face("HarmsWay", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        HARMS_AB,
    );
    let new_way = card(
        NEW_WAY,
        face("NewWay", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NEW_WAY_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 4, 4);
    let pinger = card(
        PINGER3,
        creature_face("Pinger3", "{0}", 1, 1),
        KeywordSet::EMPTY,
        PING3_AB,
    );

    // 1) Outrider en-Kor redirects next 1 damage from self to Bear (3 -> 2 on Outrider, 1 on Bear).
    let mut e1 = bench(
        1943,
        vec![outrider, bear, pinger],
        [Seat::with(&[OUTRIDER, BEAR]), Seat::with(&[PINGER3])],
    );
    to_main(&mut e1, me());
    let out_id = first(&e1, ZoneLocation::Battlefield, OUTRIDER);
    let br_id = first(&e1, ZoneLocation::Battlefield, BEAR);
    let pg_id = first(&e1, ZoneLocation::Battlefield, PINGER3);
    let hash_no_shield = e1.state().snapshot_hash();
    let loop_no_shield = e1.state().loop_signature();
    activate(&mut e1, me(), out_id, 0);
    if let Pending::ChooseTargets { player, .. } = e1.pending().clone() {
        e1.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![br_id],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e1, me());
    assert_ne!(
        e1.state().snapshot_hash(),
        hash_no_shield,
        "RedirectNextTo shield must change snapshot_hash"
    );
    assert_ne!(
        e1.state().loop_signature(),
        loop_no_shield,
        "RedirectNextTo shield must change loop_signature"
    );

    // Opponent pings Outrider for 3 -> 1 redirected to Bear, 2 stays on Outrider!
    e1.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e1, them(), pg_id, 0);
    if let Pending::ChooseTargets { player, .. } = e1.pending().clone() {
        e1.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![out_id],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e1, me());
    assert_eq!(e1.state().object(out_id).unwrap().damage, 2);
    assert_eq!(e1.state().object(br_id).unwrap().damage, 1);

    // 2) Harm's Way: protects me AND my permanents, redirects 2 of 3 damage from chosen Pinger to opponent!
    let mut e2 = bench(
        1944,
        vec![harms, bear, pinger],
        [
            Seat::with(&[BEAR]).holding(&[HARMS_WAY]).at(20),
            Seat::with(&[PINGER3]).at(20),
        ],
    );
    to_main(&mut e2, me());
    let br2 = first(&e2, ZoneLocation::Battlefield, BEAR);
    let pg2 = first(&e2, ZoneLocation::Battlefield, PINGER3);
    let hw = first(&e2, ZoneLocation::Hand(me()), HARMS_WAY);
    cast(&mut e2, me(), hw);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    }
    // Pass to let Harm's Way resolve -> asks ChooseDamageSource!
    loop {
        match e2.pending().clone() {
            Pending::ChooseDamageSource {
                player,
                choice,
                options,
            } => {
                let pg_ref = options
                    .into_iter()
                    .find(|r| r.object == pg2)
                    .expect("pinger offered");
                e2.apply(
                    player,
                    PlayerAction::ChooseDamageSource {
                        choice,
                        source: pg_ref,
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e2.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e2.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    // Opponent pings my Bear for 3 -> 2 redirected to opponent (life 20 -> 18), 1 stays on Bear!
    e2.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e2, them(), pg2, 0);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![br2],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e2, me());
    assert_eq!(e2.state().object(br2).unwrap().damage, 1);
    assert_eq!(e2.state().players[1].life, 18);

    // 3) New Way Forward: prevents next 3 damage from chosen source, deals 3 to source's controller, draws 3!
    let mut e3 = bench(
        1945,
        vec![new_way, pinger],
        [
            Seat::default().holding(&[NEW_WAY]).at(20),
            Seat::with(&[PINGER3]).at(20),
        ],
    );
    to_main(&mut e3, me());
    let pg3 = first(&e3, ZoneLocation::Battlefield, PINGER3);
    let nw = first(&e3, ZoneLocation::Hand(me()), NEW_WAY);
    cast(&mut e3, me(), nw);
    loop {
        match e3.pending().clone() {
            Pending::ChooseDamageSource {
                player,
                choice,
                options,
            } => {
                let pg_ref = options
                    .into_iter()
                    .find(|r| r.object == pg3)
                    .expect("pinger offered");
                e3.apply(
                    player,
                    PlayerAction::ChooseDamageSource {
                        choice,
                        source: pg_ref,
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e3.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e3.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    // First ping: prevented -> opponent takes 3, I draw 3, my life stays 20.
    e3.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e3, them(), pg3, 0);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    }
    settle(&mut e3, me());
    assert_eq!(e3.state().players[0].life, 20);
    assert_eq!(e3.state().players[1].life, 17);
    assert_eq!(e3.state().zones.list(ZoneLocation::Hand(me())).len(), 3);

    // Second ping: shield was consumed -> I take 3!
    e3.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e3, them(), pg3, 0);
    if let Pending::ChooseTargets { player, .. } = e3.pending().clone() {
        e3.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    }
    settle(&mut e3, me());
    assert_eq!(
        e3.state().players[0].life,
        17,
        "shield was consumed after first ping"
    );
}

// =========================================================================
// 4. Hash Completeness & Sensitivity (Tags 580..=586)
// =========================================================================

#[test]
fn hash_sensitivity_for_new_replacement_and_prevention_states() {
    const BEAR: u32 = 9690;
    const HARMS_WAY: u32 = 9691;
    const NEW_WAY: u32 = 9692;
    const PINGER: u32 = 9693;

    static HARMS_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::RedirectNextDamageTo {
            from: TargetSpec::PlayerOrObject(&Filter::ControlledByYou),
            choose_source: Some(&Filter::Any),
            amount: Amount::Fixed(2),
            to: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];
    static NEW_WAY_OPS: &[Effect] = &[Effect::DrawCardsFor {
        who: PlayerRel::You,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }];
    static NEW_WAY_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[
        Effect::PreventNextFromChosenSourceWithThen {
            target: TargetSpec::Player(PlayerRel::You),
            sources: &Filter::Any,
            combat_only: false,
            then: DamageRider::sequence(NEW_WAY_OPS),
        }
    ])];
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);
    let harms = card(
        HARMS_WAY,
        face("Harms", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        HARMS_AB,
    );
    let new_way = card(
        NEW_WAY,
        face("NewWay", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NEW_WAY_AB,
    );
    let pinger = plain_creature(PINGER, "Pinger", "{0}", 1, 1);

    // 1) Shielded::PlayerOrControlledPermanents vs Shielded::Player and RedirectNextTo remaining (tag 584).
    let mut e = bench(
        1990,
        vec![bear, pinger],
        [Seat::with(&[BEAR]), Seat::with(&[PINGER])],
    );
    to_main(&mut e, me());
    let h_base = e.state().snapshot_hash();
    let l_base = e.state().loop_signature();
    e.state.shields.push(crate::prevention::Shield {
        protects: crate::prevention::Shielded::PlayerOrControlledPermanents(me()),
        kind: crate::prevention::ShieldKind::RedirectNextTo {
            remaining: 2,
            to: crate::event::DamageTarget::Player(them()),
            to_version: None,
            source: None,
        },
        controller: me(),
    });
    let h_s2 = e.state().snapshot_hash();
    let l_s2 = e.state().loop_signature();
    assert_ne!(h_s2, h_base);
    assert_ne!(l_s2, l_base);
    e.state.shields[0].kind = crate::prevention::ShieldKind::RedirectNextTo {
        remaining: 1,
        to: crate::event::DamageTarget::Player(them()),
        to_version: None,
        source: None,
    };
    assert_ne!(
        e.state().snapshot_hash(),
        h_s2,
        "remaining 2 vs 1 must differ"
    );
    assert_ne!(
        e.state().loop_signature(),
        l_s2,
        "remaining 2 vs 1 loop_sig must differ"
    );
    e.state.shields[0].protects = crate::prevention::Shielded::Player(me());
    assert_ne!(
        e.state().snapshot_hash(),
        h_s2,
        "Player vs PlayerOrControlledPermanents must differ"
    );

    // 2) Pending AwaitingOp::RedirectNextToChosenSource vs ShieldNextWithThenFromChosenSource (tag 586).
    let mut ea = bench(
        1991,
        vec![harms, pinger],
        [Seat::default().holding(&[HARMS_WAY]), Seat::with(&[PINGER])],
    );
    to_main(&mut ea, me());
    let h_before = ea.state().snapshot_hash();
    let hw = first(&ea, ZoneLocation::Hand(me()), HARMS_WAY);
    cast(&mut ea, me(), hw);
    if let Pending::ChooseTargets { player, .. } = ea.pending().clone() {
        ea.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    }
    walk_until(&mut ea, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    assert_ne!(ea.state().snapshot_hash(), h_before);
    let fp_a = ea
        .resolution
        .as_ref()
        .unwrap()
        .awaiting
        .as_ref()
        .unwrap()
        .search_fingerprint();
    assert_ne!(
        fp_a, 0,
        "RedirectNextToChosenSource must produce nonzero search_fingerprint"
    );

    let mut eb = bench(
        1992,
        vec![new_way, pinger],
        [Seat::default().holding(&[NEW_WAY]), Seat::with(&[PINGER])],
    );
    to_main(&mut eb, me());
    let nw = first(&eb, ZoneLocation::Hand(me()), NEW_WAY);
    cast(&mut eb, me(), nw);
    walk_until(&mut eb, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let fp_b = eb
        .resolution
        .as_ref()
        .unwrap()
        .awaiting
        .as_ref()
        .unwrap()
        .search_fingerprint();
    assert_ne!(
        fp_b, 0,
        "ShieldNextWithThenFromChosenSource must produce nonzero search_fingerprint"
    );
    assert_ne!(fp_a, fp_b);

    // 3) Tag 580: FrontDraw::{multi_settled, multi_applied, multi_pending}.
    let mut e580 = bench(
        1993,
        vec![bear, pinger],
        [Seat::with(&[BEAR]), Seat::with(&[PINGER])],
    );
    to_main(&mut e580, me());
    let bear_id = first(&e580, ZoneLocation::Battlefield, BEAR);
    let h580_0 = e580.state().snapshot_hash();
    let l580_0 = e580.state().loop_signature();
    e580.state.draw_front.multi_settled = true;
    let h580_1 = e580.state().snapshot_hash();
    let l580_1 = e580.state().loop_signature();
    assert_ne!(
        h580_1, h580_0,
        "multi_settled must change snapshot_hash (tag 580)"
    );
    assert_ne!(
        l580_1, l580_0,
        "multi_settled must change loop_signature (tag 580)"
    );
    e580.state.draw_front.multi_applied.push(bear_id);
    let h580_2 = e580.state().snapshot_hash();
    let l580_2 = e580.state().loop_signature();
    assert_ne!(
        h580_2, h580_1,
        "multi_applied must change snapshot_hash (tag 580)"
    );
    assert_ne!(
        l580_2, l580_1,
        "multi_applied must change loop_signature (tag 580)"
    );
    e580.state
        .draw_front
        .multi_pending
        .push((them(), vec![bear_id]));
    assert_ne!(
        e580.state().snapshot_hash(),
        h580_2,
        "multi_pending must change snapshot_hash (tag 580)"
    );
    assert_ne!(
        e580.state().loop_signature(),
        l580_2,
        "multi_pending must change loop_signature (tag 580)"
    );

    // 4) Tag 581: PerTurnExtraDraws gated & canonical seat order.
    let mut e581 = bench(1994, vec![bear], [Seat::with(&[BEAR]), Seat::default()]);
    to_main(&mut e581, me());
    let h581_0 = e581.state().snapshot_hash();
    let l581_0 = e581.state().loop_signature();
    e581.state.per_turn.extra_draw_occurred.tracked = true;
    assert_eq!(
        e581.state().snapshot_hash(),
        h581_0,
        "empty PerTurnExtraDraws must stay gated (tag 581)"
    );
    e581.state.per_turn.extra_draw_occurred.mark(them());
    let h581_1 = e581.state().snapshot_hash();
    let l581_1 = e581.state().loop_signature();
    assert_ne!(
        h581_1, h581_0,
        "PerTurnExtraDraws must change snapshot_hash (tag 581)"
    );
    assert_ne!(
        l581_1, l581_0,
        "PerTurnExtraDraws must change loop_signature (tag 581)"
    );
    e581.state.per_turn.extra_draw_occurred.mark(me());
    let h581_both_a = e581.state().snapshot_hash();
    let l581_both_a = e581.state().loop_signature();
    let fp581_a = crate::state::structural_fingerprint(&e581.state.per_turn.extra_draw_occurred);
    e581.state.per_turn.extra_draw_occurred.seats.clear();
    e581.state.per_turn.extra_draw_occurred.mark(me());
    e581.state.per_turn.extra_draw_occurred.mark(them());
    assert_eq!(
        e581.state().snapshot_hash(),
        h581_both_a,
        "PerTurnExtraDraws snapshot_hash must be canonical across seat order"
    );
    assert_eq!(
        e581.state().loop_signature(),
        l581_both_a,
        "PerTurnExtraDraws loop_signature must be canonical across seat order"
    );
    assert_eq!(
        crate::state::structural_fingerprint(&e581.state.per_turn.extra_draw_occurred),
        fp581_a,
        "PerTurnExtraDraws::hash must be canonical across seat order"
    );

    // 5) Tag 583: entry::Attempt new fields on suspended CR 616.1b choice.
    const XANTCHA_H: u32 = 9562;
    const GATHER_H: u32 = 9563;
    static XANTCHA_H_MODS: &[EnterModifier] = &[EnterModifier::EntersUnderChosenOpponentControl];
    static GATHER_H_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::EnterUnderYourControl(&Filter::CREATURE),
            duration: Duration::UntilEndOfTurn,
        }])];
    let mut xh_face = creature_face("XantchaH", "{0}", 5, 5);
    xh_face.enter_modifiers = XANTCHA_H_MODS;
    let xh = card(XANTCHA_H, xh_face, KeywordSet::EMPTY, &[]);
    let gh = card(
        GATHER_H,
        face("GatherH", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        GATHER_H_AB,
    );
    let mut e583 = bench(
        1995,
        vec![xh, gh],
        [
            Seat::default().holding(&[XANTCHA_H]),
            Seat::default().holding(&[GATHER_H]),
            Seat::default(),
        ],
    );
    to_main(&mut e583, me());
    let xh_id = first(&e583, ZoneLocation::Hand(me()), XANTCHA_H);
    cast(&mut e583, me(), xh_id);
    e583.apply(me(), PlayerAction::PassPriority).unwrap();
    let gh_id = first(&e583, ZoneLocation::Hand(them()), GATHER_H);
    cast(&mut e583, them(), gh_id);
    walk_until(&mut e583, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::EntryReplacement,
                ..
            }
        )
    });
    let h583_before = e583.state().snapshot_hash();
    let entries_fp_before = crate::state::structural_fingerprint(&e583.state.entries);
    let Pending::ChooseCards {
        player, options, ..
    } = e583.pending().clone()
    else {
        panic!("expected EntryReplacement choice");
    };
    let gather_opt = *options.iter().find(|&&id| id != xh_id).unwrap();
    e583.apply(
        player,
        PlayerAction::ChooseObjects {
            objects: vec![gather_opt],
        },
    )
    .unwrap();
    assert!(
        matches!(e583.pending(), Pending::ChoosePlayer { player: p, .. } if *p == them()),
        "Gather Specimens transfers entry controller to P1 who now chooses Xantcha opponent"
    );
    assert_ne!(
        e583.state().snapshot_hash(),
        h583_before,
        "entry::Attempt applied_effects/controller_override/applied_own_control must change snapshot_hash (tag 583)"
    );
    assert_ne!(
        crate::state::structural_fingerprint(&e583.state.entries),
        entries_fp_before,
        "entry::Attempt Hash impl must change (tag 583)"
    );

    // 6) Tag 585: SubjectContext::{damaged_object, event_source_version} fingerprint sensitivity.
    let mut sc = crate::resolve::SubjectContext::default();
    let sc_fp0 = sc.fingerprint();
    sc.damaged_object = Some(ObjectId::new(42, 0));
    let sc_fp1 = sc.fingerprint();
    assert_ne!(
        sc_fp1, sc_fp0,
        "SubjectContext::damaged_object must affect fingerprint (tag 585)"
    );
    sc.event_source_version = Some(7);
    let sc_fp2 = sc.fingerprint();
    assert_ne!(
        sc_fp2, sc_fp1,
        "SubjectContext::event_source_version must affect fingerprint (tag 585)"
    );
}

#[test]
fn comeuppance_lki_sacrificed_creature_deals_no_damage_to_controller() {
    const COME: u32 = 9570;
    const FANATIC: u32 = 9571;
    const PINGER: u32 = 9572;
    const BOLT: u32 = 9573;

    static COME_RIDER: &[Effect; 1] = &[Effect::DealDamageToEventSourceOrController {
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
        creature_source_takes_damage: true,
    }];
    static COME_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::ControlledByOpponent,
            recipients: DamageRecipients::Players(PlayerRel::You),
            combat: None,
            is_replacement: false,
            only_if_prevented: true,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(COME_RIDER),
        },
        duration: Duration::UntilEndOfTurn,
    }])];
    static SAC_PARTS: &[CostPart] = &[CostPart::SacrificeSelf];
    static FANATIC_AB: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: SAC_PARTS,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }],
        None,
    )];
    static PINGER_AB: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }],
        None,
    )];
    static BOLT_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])];

    let come = card(
        COME,
        face("Comeuppance", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COME_AB,
    );
    let fanatic = card(
        FANATIC,
        creature_face("Fanatic", "{0}", 1, 1),
        KeywordSet::EMPTY,
        FANATIC_AB,
    );
    let pinger = card(
        PINGER,
        creature_face("Pinger", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PINGER_AB,
    );
    let bolt = card(
        BOLT,
        face("Bolt", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BOLT_AB,
    );

    let mut e = bench(
        1996,
        vec![come, fanatic, pinger, bolt],
        [
            Seat::default().holding(&[COME]),
            Seat::with(&[FANATIC, PINGER]).holding(&[BOLT]),
        ],
    );
    to_main(&mut e, me());
    let come_id = first(&e, ZoneLocation::Hand(me()), COME);
    cast(&mut e, me(), come_id);
    settle(&mut e, me());

    // Pass priority to opponent so opponent activates Fanatic (sacrificing itself).
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    let fanatic_id = first(&e, ZoneLocation::Battlefield, FANATIC);
    e.apply(
        them(),
        PlayerAction::ActivateAbility {
            source: fanatic_id,
            ability_index: 0,
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert_eq!(
        e.state().object(fanatic_id).unwrap().zone,
        Zone::Graveyard,
        "Fanatic was sacrificed to pay cost"
    );
    assert_eq!(
        e.state().players[0].life,
        20,
        "Comeuppance prevented Fanatic damage"
    );
    assert_eq!(
        e.state().players[1].life,
        20,
        "CR 608.2h: sacrificed creature source must NOT fall through to deal damage to its controller"
    );

    // Noncreature spell (Bolt) still deals reflected damage to opponent.
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    let bolt_id = first(&e, ZoneLocation::Hand(them()), BOLT);
    cast(&mut e, them(), bolt_id);
    settle(&mut e, me());
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(
        e.state().players[1].life,
        18,
        "noncreature spell reflects damage to its controller"
    );

    // Creature source still on the battlefield takes the reflected damage (killing 2/2 Pinger).
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    let pinger_id = first(&e, ZoneLocation::Battlefield, PINGER);
    e.apply(
        them(),
        PlayerAction::ActivateAbility {
            source: pinger_id,
            ability_index: 0,
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(
        e.state().players[1].life,
        18,
        "opponent takes no extra damage"
    );
    assert_eq!(
        e.state().object(pinger_id).unwrap().zone,
        Zone::Graveyard,
        "living creature source took 2 reflected damage and died"
    );
}

#[test]
fn simultaneous_damage_chosen_source_shield_protects_player_and_permanents() {
    const SHIELD_SPELL: u32 = 9574;
    const QUAKER: u32 = 9575;
    const BEAR: u32 = 9576;

    static GAIN_RIDER: &[Effect; 1] = &[Effect::GainLife {
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }];
    static SHIELD_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[
        Effect::PreventNextFromChosenSourceWithThen {
            target: TargetSpec::PlayerOrObject(&Filter::ControlledByYou),
            sources: &Filter::Any,
            combat_only: false,
            then: DamageRider::new(GAIN_RIDER),
        }
    ])];
    static QUAKER_AB: &[AbilityDef] = &[free(
        &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(3),
                filter: &Filter::CREATURE,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ],
        None,
    )];

    let shield_spell = card(
        SHIELD_SPELL,
        face("ShieldAll", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SHIELD_AB,
    );
    let quaker = card(
        QUAKER,
        creature_face("Quaker", "{0}", 4, 4),
        KeywordSet::EMPTY,
        QUAKER_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    let mut e = bench(
        1997,
        vec![shield_spell, quaker, bear],
        [
            Seat::with(&[BEAR]).holding(&[SHIELD_SPELL]),
            Seat::with(&[QUAKER]),
        ],
    );
    to_main(&mut e, me());
    let quaker_id = first(&e, ZoneLocation::Battlefield, QUAKER);
    let bear_id = first(&e, ZoneLocation::Battlefield, BEAR);
    let shield_id = first(&e, ZoneLocation::Hand(me()), SHIELD_SPELL);
    cast(&mut e, me(), shield_id);
    loop {
        match e.pending().clone() {
            Pending::ChooseDamageSource {
                player,
                choice,
                options,
            } => {
                let q_ref = options
                    .into_iter()
                    .find(|r| r.object == quaker_id)
                    .expect("Quaker offered");
                e.apply(
                    player,
                    PlayerAction::ChooseDamageSource {
                        choice,
                        source: q_ref,
                    },
                )
                .unwrap();
            }
            Pending::Priority { player, .. }
                if e.state().zones.stack_is_empty() && player == me() =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    // First simultaneous activation: deals 3 to each creature + 3 to each player.
    // Shield must stay active across BOTH `me()` and `bear_id`, preventing 6 total!
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(
        them(),
        PlayerAction::ActivateAbility {
            source: quaker_id,
            ability_index: 0,
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert_eq!(
        e.state().object(bear_id).unwrap().zone,
        Zone::Battlefield,
        "simultaneous shield must protect controlled permanent in addition to player"
    );
    assert_eq!(
        e.state().players[0].life,
        26,
        "3 to player + 3 to bear prevented simultaneously => +6 life"
    );
    assert_eq!(
        e.state().players[1].life,
        17,
        "opponent still took 3 from their own Quaker"
    );

    // Second activation: shield was consumed at end of first event, so Bear dies and me() takes 3.
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(
        them(),
        PlayerAction::ActivateAbility {
            source: quaker_id,
            ability_index: 0,
        },
    )
    .unwrap();
    settle(&mut e, me());
    assert_eq!(
        e.state().object(bear_id).unwrap().zone,
        Zone::Graveyard,
        "shield must be consumed after the first simultaneous event"
    );
    assert_eq!(e.state().players[0].life, 23);
}

#[test]
fn control_changing_entry_replacements_apply_before_non_control_cr_616_1b() {
    const GATHER: u32 = 9577;
    const KISMET: u32 = 9578;
    const XANTCHA: u32 = 9579;
    const BEAR: u32 = 9580;

    static GATHER_AB: &[AbilityDef] =
        &[baylee_cards_dsl::spell!(&[Effect::AddGlobalTempModifier {
            filter: &Filter::Any,
            modifier: Modifier::EnterUnderYourControl(&Filter::CREATURE),
            duration: Duration::UntilEndOfTurn,
        }])];
    static KISMET_FILTER: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);
    static KISMET_AB: &[AbilityDef] = &[AbilityDef::Replacement(ReplacementRule::EnterTapped(
        &KISMET_FILTER,
    ))];
    static XANTCHA_MODS: &[EnterModifier] = &[EnterModifier::EntersUnderChosenOpponentControl];

    let gather = card(
        GATHER,
        face("Gather", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        GATHER_AB,
    );
    let kismet = card(
        KISMET,
        face("Kismet", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        KISMET_AB,
    );
    let mut x_face = creature_face("Xantcha", "{0}", 5, 5);
    x_face.enter_modifiers = XANTCHA_MODS;
    let xantcha = card(XANTCHA, x_face, KeywordSet::EMPTY, &[]);
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    // Case A: Opponent controls Gather Specimens + Kismet (taps opponent's creatures).
    // Under CR 616.1b, Gather Specimens applies BEFORE Kismet; once Bear enters under
    // opponent's control, opponent's Kismet no longer applies -> enters UNTAPPED!
    let mut ea = bench(
        1998,
        vec![gather, kismet, bear],
        [
            Seat::default().holding(&[BEAR]),
            Seat::with(&[KISMET]).holding(&[GATHER]),
        ],
    );
    to_main(&mut ea, me());
    let bear_a = first(&ea, ZoneLocation::Hand(me()), BEAR);
    cast(&mut ea, me(), bear_a);
    ea.apply(me(), PlayerAction::PassPriority).unwrap();
    let gather_a = first(&ea, ZoneLocation::Hand(them()), GATHER);
    cast(&mut ea, them(), gather_a);
    settle(&mut ea, me());
    let obj_a = ea.state().object(bear_a).unwrap();
    assert_eq!(obj_a.zone, Zone::Battlefield);
    assert_eq!(obj_a.controller, them());
    assert!(
        !obj_a.status.contains(crate::object::Status::TAPPED),
        "CR 616.1b: Gather Specimens must apply before opponent's Kismet so Bear enters untapped"
    );

    // Case B: I control Kismet, opponent controls Gather Specimens.
    // Gather Specimens applies first -> Bear enters under opponent's control -> MY Kismet taps it!
    let mut eb = bench(
        1999,
        vec![gather, kismet, bear],
        [
            Seat::with(&[KISMET]).holding(&[BEAR]),
            Seat::default().holding(&[GATHER]),
        ],
    );
    to_main(&mut eb, me());
    let bear_b = first(&eb, ZoneLocation::Hand(me()), BEAR);
    cast(&mut eb, me(), bear_b);
    eb.apply(me(), PlayerAction::PassPriority).unwrap();
    let gather_b = first(&eb, ZoneLocation::Hand(them()), GATHER);
    cast(&mut eb, them(), gather_b);
    settle(&mut eb, me());
    let obj_b = eb.state().object(bear_b).unwrap();
    assert_eq!(obj_b.zone, Zone::Battlefield);
    assert_eq!(obj_b.controller, them());
    assert!(
        obj_b.status.contains(crate::object::Status::TAPPED),
        "CR 616.1b: after Gather Specimens moves control to opponent, my Kismet must tap Bear"
    );

    // Case C: I control Kismet and cast Xantcha -> Xantcha enters under opponent's control TAPPED!
    let mut ec = bench(
        2000,
        vec![kismet, xantcha],
        [Seat::with(&[KISMET]).holding(&[XANTCHA]), Seat::default()],
    );
    to_main(&mut ec, me());
    let x_c = first(&ec, ZoneLocation::Hand(me()), XANTCHA);
    cast(&mut ec, me(), x_c);
    settle(&mut ec, me());
    let obj_c = ec.state().object(x_c).unwrap();
    assert_eq!(obj_c.zone, Zone::Battlefield);
    assert_eq!(obj_c.controller, them());
    assert!(
        obj_c.status.contains(crate::object::Status::TAPPED),
        "CR 616.1b: Xantcha control change applies first so my Kismet taps it"
    );

    // Case D: I cast Xantcha while opponent controls BOTH Gather Specimens AND Kismet.
    // The CR 616.1b choice MUST contain ONLY Xantcha and Gather Specimens (never Kismet).
    let mut ed = bench(
        2001,
        vec![gather, kismet, xantcha],
        [
            Seat::default().holding(&[XANTCHA]),
            Seat::with(&[KISMET]).holding(&[GATHER]),
        ],
    );
    to_main(&mut ed, me());
    let kismet_d = first(&ed, ZoneLocation::Battlefield, KISMET);
    let x_d = first(&ed, ZoneLocation::Hand(me()), XANTCHA);
    cast(&mut ed, me(), x_d);
    ed.apply(me(), PlayerAction::PassPriority).unwrap();
    let gather_d = first(&ed, ZoneLocation::Hand(them()), GATHER);
    cast(&mut ed, them(), gather_d);
    walk_until(&mut ed, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::EntryReplacement,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player, options, ..
    } = ed.pending().clone()
    else {
        panic!("expected EntryReplacement choice");
    };
    assert_eq!(player, me());
    assert_eq!(
        options.len(),
        2,
        "only control-changing tier must be offered"
    );
    assert!(options.contains(&x_d));
    assert!(
        !options.contains(&kismet_d),
        "Kismet must NOT be offered in CR 616.1b tier"
    );
    ed.apply(player, PlayerAction::ChooseObjects { objects: vec![x_d] })
        .unwrap();
    settle(&mut ed, me());
    let obj_d = ed.state().object(x_d).unwrap();
    assert_eq!(obj_d.controller, them());
    assert!(
        !obj_d.status.contains(crate::object::Status::TAPPED),
        "after Xantcha moves to opponent, opponent's Kismet does not tap it"
    );
}

#[test]
fn unpreventable_damage_fires_unconditional_rider_only_cr_615_12() {
    const NINE: u32 = 9581;
    const SWANS: u32 = 9582;
    const UNPREV_P: u32 = 9583;
    const UNPREV_C: u32 = 9584;

    static NINE_RIDER: &[Effect; 1] = &[Effect::AddCounterSource {
        kind: CounterKind::Charge,
        amount: Amount::Fixed(1),
    }];
    static NINE_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Players(PlayerRel::You),
            combat: None,
            is_replacement: false,
            only_if_prevented: false,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(NINE_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static SWANS_RIDER: &[Effect; 1] = &[Effect::DrawCardsFor {
        who: PlayerRel::You,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }];
    static SWANS_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceOrPreventDamageScopedWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Permanents(&Filter::This),
            combat: None,
            is_replacement: false,
            only_if_prevented: true,
            scope: DamageRiderScope::PerSource,
            then: DamageRider::new(SWANS_RIDER),
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static UNPREV_P_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[
        Effect::DealDamageWithoutPrevention {
            amount: Amount::Fixed(3),
            target: TargetSpec::Player(PlayerRel::You),
        }
    ])];
    static UNPREV_C_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::DealDamageWithoutPrevention {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let nine = card(
        NINE,
        face("NineLives", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        NINE_AB,
    );
    let swans = card(
        SWANS,
        creature_face("Swans", "{0}", 4, 4),
        KeywordSet::EMPTY,
        SWANS_AB,
    );
    let unprev_p = card(
        UNPREV_P,
        face("UnprevPlayer", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        UNPREV_P_AB,
    );
    let unprev_c = card(
        UNPREV_C,
        face("UnprevCreature", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        UNPREV_C_AB,
    );

    // 1) Nine Lives (`only_if_prevented: false`): unpreventable damage still deals 3,
    // AND CR 615.12 still adds the counter!
    let mut e1 = bench(
        2002,
        vec![nine, unprev_p],
        [Seat::with(&[NINE]).holding(&[UNPREV_P]), Seat::default()],
    );
    to_main(&mut e1, me());
    let nine_id = first(&e1, ZoneLocation::Battlefield, NINE);
    let up_id = first(&e1, ZoneLocation::Hand(me()), UNPREV_P);
    cast(&mut e1, me(), up_id);
    settle(&mut e1, me());
    assert_eq!(
        e1.state().players[0].life,
        17,
        "unpreventable damage still dealt"
    );
    assert_eq!(
        e1.state()
            .object(nine_id)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1,
        "CR 615.12: unconditional prevention rider must still fire when damage can't be prevented"
    );

    // 2) Swans (`only_if_prevented: true`): unpreventable damage deals 3, rider does NOT draw!
    let mut e2 = bench(
        2003,
        vec![swans, unprev_c],
        [Seat::with(&[SWANS]).holding(&[UNPREV_C]), Seat::default()],
    );
    to_main(&mut e2, me());
    let swans_id = first(&e2, ZoneLocation::Battlefield, SWANS);
    let hand_before = e2.state().zones.list(ZoneLocation::Hand(me())).len();
    let uc_id = first(&e2, ZoneLocation::Hand(me()), UNPREV_C);
    cast(&mut e2, me(), uc_id);
    if let Pending::ChooseTargets { player, .. } = e2.pending().clone() {
        e2.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![swans_id],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e2, me());
    assert_eq!(
        e2.state().object(swans_id).unwrap().damage,
        3,
        "unpreventable damage dealt"
    );
    assert_eq!(
        e2.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1,
        "only_if_prevented=true rider must NOT draw when prevented == 0"
    );
}

#[test]
fn permanent_spell_copy_exiled_by_not_cast_entry_replacement_cr_707_10f() {
    const PRIEST: u32 = 9585;
    const COPIER: u32 = 9586;
    const BEAR: u32 = 9587;

    static PRIEST_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReplaceBattlefieldEntry {
            filter: &Filter::CREATURE,
            predicate: EntryPredicate::NotCast,
            destination: EntryInsteadZone::Exile,
        },
        condition: None,
        source_zone: ZoneRef::Battlefield,
    })];
    static COPIER_AB: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::CREATURE))),
    )];

    let priest = card(
        PRIEST,
        creature_face("Priest", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PRIEST_AB,
    );
    let copier = card(
        COPIER,
        face("DoubleMajor", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COPIER_AB,
    );
    let bear = plain_creature(BEAR, "Bear", "{0}", 2, 2);

    let mut e = bench(
        2004,
        vec![priest, copier, bear],
        [
            Seat::with(&[PRIEST]).holding(&[BEAR, COPIER]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());
    let priest_id = first(&e, ZoneLocation::Battlefield, PRIEST);
    let bear_id = first(&e, ZoneLocation::Hand(me()), BEAR);
    let copier_id = first(&e, ZoneLocation::Hand(me()), COPIER);
    cast(&mut e, me(), bear_id);
    // Hold priority and copy the Bear creature spell on the stack!
    cast(&mut e, me(), copier_id);
    if let Pending::ChooseTargets { player, .. } = e.pending().clone() {
        e.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .unwrap();
    }
    settle(&mut e, me());

    // Original cast Bear entered the battlefield normally.
    assert_eq!(e.state().object(bear_id).unwrap().zone, Zone::Battlefield);
    // The permanent spell copy was NOT cast (CR 707.10f) -> exiled by Priest, never on battlefield!
    let bears_on_bf: Vec<_> = e
        .state()
        .battlefield_seen()
        .filter(|&id| {
            id != priest_id
                && e.state()
                    .object(id)
                    .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
        })
        .collect();
    assert_eq!(
        bears_on_bf,
        vec![bear_id],
        "CR 707.10f: copied creature spell must be exiled by NotCast replacement while original cast Bear enters"
    );
}
