//! Graveyard casting family (CR 702.34a non-mana flashback, CR 702.84a unearth,
//! CR 702.127a aftermath, CR 702.128a embalm, CR 702.129a eternalize,
//! CR 702.133a jump-start, CR 702.141a encore) and face-down / morph /
//! megamorph / bestow casting & turn-face-up (CR 702.37, CR 702.103, CR 708).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{AnswerFault, CastModeKind, TURN_FACE_UP};
use crate::turn::{Phase, Step};
use baylee_cards_dsl::prelude::{embalm, encore, eternalize, triggered, unearth};
use baylee_cards_dsl::{
    Amount, CostPart, CounterKind, Filter, Layer, Modifier, PlayerRel, StaticAbility, TargetSpec,
    Trigger, ZoneRef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{creature, enchantment};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const SAC_FLASHBACK_SORCERY: u32 = 8100;
const FODDER_BEAR: u32 = 8101;
const JUMP_START_BOLT: u32 = 8102;
const COUNTER_PROBE: u32 = 8104;
const AFTERMATH_SPLIT: u32 = 8105;
const UNEARTH_RAIDER: u32 = 8106;
const BOUNCE_PROBE: u32 = 8107;
const ENCORE_WARRIOR: u32 = 8108;
const EMBALM_CLERIC: u32 = 8109;
const ETERNALIZE_MAGE: u32 = 8110;
const MEGAMORPH_BEAST: u32 = 8111;
const BOUNCE_MORPH_DRAKE: u32 = 8112;
const BESTOW_NYMPH: u32 = 8113;
const CREATURE_LOCK_PILLAR: u32 = 8114;
const DESTROY_PROBE: u32 = 8115;

fn split_card(index: u32, left: FaceDef, right: FaceDef) -> &'static CardDef {
    assert_ne!(index, forest());
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test-split",
        scryfall_id: "test-split",
        faces: Box::leak(Box::new([left, right])),
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn move_to_graveyard(engine: &mut Bench, card_id: ObjectId, owner: PlayerId) {
    let _ = engine.state.move_object(
        card_id,
        ZoneLocation::Graveyard(owner),
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    refresh_offer(engine);
}

fn add_mana(engine: &mut Bench, seat: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[seat.get() as usize]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn in_first_main(engine: &Bench, seat: PlayerId) -> bool {
    holds_priority_in(engine, seat, Step::Main) && engine.state().turn.phase == Phase::FirstMain
}

fn in_second_main(engine: &Bench, seat: PlayerId) -> bool {
    holds_priority_in(engine, seat, Step::Main) && engine.state().turn.phase == Phase::SecondMain
}

#[test]
fn flashback_with_nonmana_sacrifice_cost_gates_offer_and_exiles_on_resolution() {
    static SAC_CREATURE: &[CostPart] = &[CostPart::Sacrifice(&Filter::CREATURE)];
    static DRAW_ONE: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let cards = vec![
        card(
            SAC_FLASHBACK_SORCERY,
            FaceDef {
                flashback: Some(ManaCost::ZERO),
                flashback_parts: SAC_CREATURE,
                ..face("Therapy Echo", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            DRAW_ONE,
        ),
        card(
            FODDER_BEAR,
            creature_face("Grizzly", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1,
        cards,
        [
            Seat::default().holding(&[SAC_FLASHBACK_SORCERY, FODDER_BEAR]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let spell = the(&engine, ZoneLocation::Hand(me()), SAC_FLASHBACK_SORCERY);
    let bear = the(&engine, ZoneLocation::Hand(me()), FODDER_BEAR);
    move_to_graveyard(&mut engine, spell, me());

    // Negative: no creature to sacrifice -> uncastable and unpayable from graveyard.
    let (_, legal_empty) = offer(&engine);
    assert!(
        !legal_empty.castable.contains(&spell) && !legal_empty.payable.contains(&spell),
        "non-mana flashback cost with no creature on board must not be offered"
    );
    assert!(
        !legal_empty.cast_ways.iter().any(|(id, _)| *id == spell),
        "cast_ways must omit unpayable non-mana flashback"
    );

    // Cast Bear onto me()'s battlefield.
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear })
        .expect("cast bear");
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    let (_, legal_with_bear) = offer(&engine);
    assert!(
        legal_with_bear.castable.contains(&spell),
        "flashback becomes castable once sacrifice fodder exists"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell })
        .expect("start flashback cast");
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { options, .. } if options.contains(&bear))
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .expect("sacrifice bear for flashback");

    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "flashback spell resolved and drew a card"
    );
    assert_eq!(
        engine.state().object(spell).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.34a: flashed-back spell is exiled on resolution"
    );
}

#[test]
fn jump_start_requires_discard_from_hand_and_exiles_on_resolution_or_counter() {
    static BOLT_OPP: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::Opponent,
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static COUNTER_TARGET: &[Effect] = &[Effect::CounterTargetSpellOrAbility];

    let cards = vec![
        card(
            JUMP_START_BOLT,
            FaceDef {
                jump_start: true,
                ..face("Spark Jump", "{1}{R}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            BOLT_OPP,
        ),
        card(
            COUNTER_PROBE,
            face("Null Tower", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            const {
                &[free(
                    COUNTER_TARGET,
                    Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
                )]
            },
        ),
    ];

    let mut engine = bench(
        2,
        cards,
        [
            Seat::default().holding(&[JUMP_START_BOLT, JUMP_START_BOLT]),
            Seat::with(&[COUNTER_PROBE]),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let bolts = objects(&engine, ZoneLocation::Hand(me()), JUMP_START_BOLT);
    let (bolt1, bolt2) = (bolts[0], bolts[1]);
    move_to_graveyard(&mut engine, bolt1, me());
    move_to_graveyard(&mut engine, bolt2, me());
    add_mana(&mut engine, me(), ManaColor::Red, 4);

    // Negative: hand is empty -> jump-start cannot pay its discard cost.
    let (_, legal_empty_hand) = offer(&engine);
    assert!(
        !legal_empty_hand.castable.contains(&bolt1) && !legal_empty_hand.payable.contains(&bolt1),
        "CR 702.133a: jump-start requires a card in hand to discard"
    );

    // Move two cards from library into hand.
    let fodder1 = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let _ = engine.state.move_object(
        fodder1,
        ZoneLocation::Hand(me()),
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let fodder2 = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let _ = engine.state.move_object(
        fodder2,
        ZoneLocation::Hand(me()),
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    refresh_offer(&mut engine);

    let (_, legal_ready) = offer(&engine);
    assert!(legal_ready.castable.contains(&bolt1));
    assert!(
        legal_ready
            .cast_ways
            .iter()
            .any(|(id, w)| *id == bolt1 && w.kind == CastModeKind::JumpStart),
        "jump-start route must be advertised in cast_ways"
    );

    // 1) Cast bolt1 via jump-start and let it resolve.
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt1 })
        .expect("cast jump-start bolt1");
    assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder1],
            },
        )
        .expect("discard fodder1");
    assert_eq!(
        engine.state().object(fodder1).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(life(&engine, them()), 18);
    assert_eq!(
        engine.state().object(bolt1).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.133a: resolved jump-start spell is exiled"
    );

    // 2) Cast bolt2 via jump-start and counter it -> still exiled!
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt2 })
        .expect("cast jump-start bolt2");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder2],
            },
        )
        .expect("discard fodder2");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass to opponent");
    let probe = the(&engine, ZoneLocation::Battlefield, COUNTER_PROBE);
    engine
        .apply(
            them(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("counter jump-start spell");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![bolt2],
                    players: vec![],
                },
            )
            .expect("target bolt2");
    }
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(
        life(&engine, them()),
        18,
        "countered bolt2 did not deal damage"
    );
    assert_eq!(
        engine.state().object(bolt2).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.133a: countered jump-start spell is exiled any time it leaves the stack"
    );
}

#[test]
fn aftermath_split_card_combines_cmc_blocks_hand_cast_and_casts_from_graveyard_to_exile() {
    static GAIN_THREE: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static DRAIN_OPP: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::LoseLife {
            amount: Amount::Fixed(3),
            target: PlayerRel::Opponent,
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let split = split_card(
        AFTERMATH_SPLIT,
        FaceDef {
            abilities: GAIN_THREE,
            ..face("Dawnward", "{1}{W}", TypeSet::SORCERY)
        },
        FaceDef {
            castable_from_hand: false,
            aftermath: true,
            abilities: DRAIN_OPP,
            ..face("Duskward", "{2}{B}", TypeSet::INSTANT)
        },
    );
    assert!(
        split.is_split_spell_card(),
        "aftermath card is a split spell card"
    );

    let mut engine = bench(
        3,
        vec![split],
        [Seat::default().holding(&[AFTERMATH_SPLIT]), Seat::default()],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let card_id = the(&engine, ZoneLocation::Hand(me()), AFTERMATH_SPLIT);
    assert_eq!(
        engine
            .state()
            .object(card_id)
            .unwrap()
            .characteristics()
            .mana_value(),
        5,
        "CR 709.4b: split card with aftermath has combined mana value (2 + 3 = 5) in hand"
    );

    add_mana(&mut engine, me(), ManaColor::White, 2);
    add_mana(&mut engine, me(), ManaColor::Black, 3);
    let (_, legal_hand) = offer(&engine);
    assert!(legal_hand.castable.contains(&card_id));
    assert!(
        !legal_hand
            .cast_ways
            .iter()
            .any(|(id, w)| *id == card_id && w.kind == CastModeKind::Face(1)),
        "CR 702.127a: aftermath half cannot be cast from hand"
    );

    // Cast front half from hand -> resolves to graveyard.
    engine
        .apply(me(), PlayerAction::CastSpell { card: card_id })
        .expect("cast front half");
    assert_eq!(
        engine
            .state()
            .object(card_id)
            .unwrap()
            .characteristics()
            .mana_value(),
        2,
        "CR 709.3b: on the stack only the cast half's characteristics exist"
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(life(&engine, me()), 23);
    assert_eq!(
        engine.state().object(card_id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        engine
            .state()
            .object(card_id)
            .unwrap()
            .characteristics()
            .mana_value(),
        5,
        "in graveyard, combined mana value is 5 again"
    );

    // Cast aftermath half (Instant) from graveyard!
    let (_, legal_grave) = offer(&engine);
    assert!(legal_grave.castable.contains(&card_id));
    assert!(
        legal_grave
            .cast_ways
            .iter()
            .any(|(id, w)| *id == card_id && w.kind == CastModeKind::Face(1)),
        "CR 702.127a: aftermath half is castable from graveyard"
    );
    assert!(
        !legal_grave
            .cast_ways
            .iter()
            .any(|(id, w)| *id == card_id && w.kind == CastModeKind::Normal),
        "front half without flashback is not castable from graveyard"
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: card_id })
        .expect("cast aftermath half from graveyard");
    assert_eq!(
        engine.state().object(card_id).unwrap().face_index,
        1,
        "aftermath puts face 1 on the stack"
    );
    assert_eq!(
        engine
            .state()
            .object(card_id)
            .unwrap()
            .characteristics()
            .mana_value(),
        3
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(life(&engine, them()), 17);
    let exiled = engine.state().object(card_id).unwrap();
    assert_eq!(
        exiled.zone,
        crate::zone::Zone::Exile,
        "CR 702.127a: aftermath spell is exiled when leaving the stack"
    );
    assert_eq!(
        exiled.face_index, 0,
        "split card off the stack rests on face 0 with combined characteristics"
    );
    assert_eq!(exiled.characteristics().mana_value(), 5);
}

#[test]
fn unearth_sorcery_speed_haste_end_step_exile_and_replacement_exile_on_bounce() {
    static UNEARTH_AB: &[AbilityDef] = &[unearth!("{1}{B}")];
    static BOUNCE_TARGET: &[Effect] = &[Effect::ReturnToHand {
        target: TargetSpec::Object(&Filter::CREATURE),
    }];

    let cards = vec![
        card(
            UNEARTH_RAIDER,
            creature_face("Ash Raider", "{2}{B}", 3, 1),
            KeywordSet::EMPTY,
            UNEARTH_AB,
        ),
        card(
            BOUNCE_PROBE,
            face("Aether Rod", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            const {
                &[free(
                    BOUNCE_TARGET,
                    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
                )]
            },
        ),
    ];

    let mut engine = bench(
        4,
        cards,
        [
            Seat::with(&[BOUNCE_PROBE]).holding(&[UNEARTH_RAIDER, UNEARTH_RAIDER]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let raiders = objects(&engine, ZoneLocation::Hand(me()), UNEARTH_RAIDER);
    let (r1, r2) = (raiders[0], raiders[1]);
    move_to_graveyard(&mut engine, r1, me());
    move_to_graveyard(&mut engine, r2, me());

    // Negative: pass to combat -> unearth is sorcery-speed only!
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::CombatBegin)
    });
    add_mana(&mut engine, me(), ManaColor::Black, 2);
    let (_, legal_combat) = offer(&engine);
    assert!(
        !legal_combat.abilities.contains(&(r1, 0)),
        "CR 702.84a: unearth may be activated only as a sorcery"
    );

    // Walk to SecondMain -> activate unearth on r1, then advance to End step -> exiled!
    walk_until(&mut engine, |e| in_second_main(e, me()));
    add_mana(&mut engine, me(), ManaColor::Black, 2);
    let (_, legal_main) = offer(&engine);
    assert!(legal_main.abilities.contains(&(r1, 0)));
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: r1,
                ability_index: 0,
            },
        )
        .expect("activate unearth on r1");
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    let r1_obj = engine.state().object(r1).unwrap();
    assert_eq!(r1_obj.zone, crate::zone::Zone::Battlefield);
    assert!(
        r1_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.84a: unearthed permanent gains haste"
    );

    // Advance past End step -> delayed trigger exiles r1!
    walk_until(&mut engine, |e| e.state().turn.active == them());
    assert_eq!(
        engine.state().object(r1).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.84a: unearthed permanent is exiled at the beginning of the next end step"
    );

    // Next turn of me(): unearth r2, then bounce it to hand -> exiled instead!
    walk_until(&mut engine, |e| in_first_main(e, me()));
    add_mana(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: r2,
                ability_index: 0,
            },
        )
        .expect("activate unearth on r2");
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(
        engine.state().object(r2).unwrap().zone,
        crate::zone::Zone::Battlefield
    );

    let rod = the(&engine, ZoneLocation::Battlefield, BOUNCE_PROBE);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: rod,
                ability_index: 0,
            },
        )
        .expect("activate bounce rod on r2");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![r2],
                    players: vec![],
                },
            )
            .expect("target r2");
    }
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(
        engine.state().object(r2).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.84a: if unearthed permanent would leave battlefield for anywhere else, exile it instead"
    );
}

#[test]
fn encore_creates_hasty_tokens_per_opponent_forced_to_attack_and_embalm_eternalize_tokens() {
    static ENCORE_AB: &[AbilityDef] = &[encore!("{2}{U}")];
    static EMBALM_AB: &[AbilityDef] = &[embalm!("{1}{W}")];
    static ETERNALIZE_AB: &[AbilityDef] = &[eternalize!("{2}{U}")];

    let cards = vec![
        card(
            ENCORE_WARRIOR,
            creature_face("Tideblade", "{1}{U}", 2, 2),
            KeywordSet::EMPTY,
            ENCORE_AB,
        ),
        card(
            EMBALM_CLERIC,
            FaceDef {
                subtypes: &[creature::HUMAN],
                ..creature_face("Sun Initiate", "{1}{G}", 2, 1)
            },
            KeywordSet::EMPTY,
            EMBALM_AB,
        ),
        card(
            ETERNALIZE_MAGE,
            FaceDef {
                subtypes: &[creature::WIZARD],
                ..creature_face("Lapis Adept", "{1}{U}", 1, 3)
            },
            KeywordSet::EMPTY,
            ETERNALIZE_AB,
        ),
    ];

    let opp1 = PlayerId::new(1);
    let opp2 = PlayerId::new(2);
    let mut engine = bench(
        5,
        cards,
        [
            Seat::default().holding(&[ENCORE_WARRIOR, EMBALM_CLERIC, ETERNALIZE_MAGE]),
            Seat::default(),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let warrior = the(&engine, ZoneLocation::Hand(me()), ENCORE_WARRIOR);
    let cleric = the(&engine, ZoneLocation::Hand(me()), EMBALM_CLERIC);
    let mage = the(&engine, ZoneLocation::Hand(me()), ETERNALIZE_MAGE);
    move_to_graveyard(&mut engine, warrior, me());
    move_to_graveyard(&mut engine, cleric, me());
    move_to_graveyard(&mut engine, mage, me());

    add_mana(&mut engine, me(), ManaColor::Blue, 6);
    add_mana(&mut engine, me(), ManaColor::White, 2);

    // 1) Activate Embalm and Eternalize first, verify token characteristics!
    let sun_name = engine.state.names.intern("Sun Initiate");
    let lapis_name = engine.state.names.intern("Lapis Adept");
    let tideblade_name = engine.state.names.intern("Tideblade");

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: cleric,
                ability_index: 0,
            },
        )
        .expect("activate embalm");
    assert_eq!(
        engine.state().object(cleric).unwrap().zone,
        crate::zone::Zone::Exile
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: mage,
                ability_index: 0,
            },
        )
        .expect("activate eternalize");
    assert_eq!(
        engine.state().object(mage).unwrap().zone,
        crate::zone::Zone::Exile
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    let embalmed = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| engine.state().object(*id).unwrap().characteristics().name == sun_name)
        .expect("embalmed token");
    let emb_chars = engine.state().object(embalmed).unwrap().characteristics();
    assert_eq!(emb_chars.colors, ColorSet::of(Color::White));
    assert_eq!(emb_chars.mana_value(), 0);
    assert!(emb_chars.subtypes.contains(creature::ZOMBIE));
    assert!(emb_chars.subtypes.contains(creature::HUMAN));
    assert_eq!((emb_chars.power, emb_chars.toughness), (Some(2), Some(1)));

    let eternalized = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| engine.state().object(*id).unwrap().characteristics().name == lapis_name)
        .expect("eternalized token");
    let et_chars = engine
        .state()
        .object(eternalized)
        .unwrap()
        .characteristics();
    assert_eq!(et_chars.colors, ColorSet::of(Color::Black));
    assert_eq!(et_chars.mana_value(), 0);
    assert!(et_chars.subtypes.contains(creature::ZOMBIE));
    assert_eq!((et_chars.power, et_chars.toughness), (Some(4), Some(4)));

    // 2) Activate Encore -> exiles warrior as cost, creates 2 hasty tokens forced to attack opp1 & opp2!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: warrior,
                ability_index: 0,
            },
        )
        .expect("activate encore");
    assert_eq!(
        engine.state().object(warrior).unwrap().zone,
        crate::zone::Zone::Exile,
        "CR 702.141a: encore exiles the card from graveyard as part of its cost"
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    let encore_tokens: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).unwrap().characteristics().name == tideblade_name)
        .collect();
    assert_eq!(encore_tokens.len(), 2, "one encore token copy per opponent");

    // Advance to DeclareAttackers.
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        required, limits, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseAttackers");
    };
    assert_eq!(required.len(), 2);
    assert!(encore_tokens.iter().all(|t| required.contains(t)));

    // Refuses empty attack declaration!
    let err = engine
        .apply(me(), PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap_err();
    assert!(matches!(
        err,
        EngineError::IllegalAction(msg) if msg == AnswerFault::MustAttack.reason()
    ));

    // Declare each token attacking its designated opponent from `limits`.
    let assigned: Vec<(ObjectId, Defender)> = limits
        .iter()
        .map(|lim| {
            assert_eq!(lim.defenders.len(), 1);
            (lim.creature, lim.defenders[0])
        })
        .collect();
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: assigned,
            },
        )
        .expect("declare required per-opponent encore attacks");

    walk_until(&mut engine, |e| in_second_main(e, me()));
    assert_eq!(life(&engine, opp1), 18);
    assert_eq!(life(&engine, opp2), 18);

    // Advance past End step -> both encore tokens are sacrificed!
    walk_until(&mut engine, |e| e.state().turn.active == opp1);
    let remaining_encore = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| engine.state().object(**id).unwrap().characteristics().name == tideblade_name)
        .count();
    assert_eq!(
        remaining_encore, 0,
        "CR 702.141a: encore tokens are sacrificed at the beginning of the next end step"
    );
}

#[test]
fn megamorph_and_nonmana_morph_turn_face_up_with_counter_and_preserve_hidden_info() {
    static TURNED_UP_GAIN: &[AbilityDef] = &[triggered!(
        Trigger::TurnedFaceUp,
        &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }]
    )];
    static BOUNCE_LAND: &[CostPart] = &[CostPart::ReturnToHand(&Filter::LAND)];

    let cards = vec![
        card(
            MEGAMORPH_BEAST,
            FaceDef {
                megamorph: Some(ManaCost::parse("{2}{G}")),
                ..creature_face("Hidden Baloth", "{4}{G}", 3, 3)
            },
            KeywordSet::EMPTY,
            TURNED_UP_GAIN,
        ),
        card(
            BOUNCE_MORPH_DRAKE,
            FaceDef {
                morph: Some(ManaCost::ZERO),
                morph_parts: BOUNCE_LAND,
                ..creature_face("Tidal Whelp", "{4}{U}", 4, 4)
            },
            KeywordSet::FLYING,
            &[],
        ),
    ];

    let mut engine = bench(
        6,
        cards,
        [
            Seat::default().holding(&[MEGAMORPH_BEAST, BOUNCE_MORPH_DRAKE, forest()]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let nameless = engine.state.names.intern("");
    let baloth = the(&engine, ZoneLocation::Hand(me()), MEGAMORPH_BEAST);
    let drake = the(&engine, ZoneLocation::Hand(me()), BOUNCE_MORPH_DRAKE);
    let my_forest = the(&engine, ZoneLocation::Hand(me()), forest());

    // Cast Baloth face down for {3}.
    add_mana(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: baloth })
        .expect("cast megamorph face down");
    let stack_obj = engine.state().object(baloth).unwrap();
    assert!(stack_obj.status.contains(crate::object::Status::FACE_DOWN));
    assert_eq!(stack_obj.characteristics().name, nameless);
    assert!(stack_obj.characteristics().colors.is_empty());
    assert_eq!(stack_obj.characteristics().mana_value(), 0);
    assert!(
        stack_obj.printed_abilities(&engine.lookup).is_empty(),
        "CR 702.37c: morph/megamorph face-down spells do NOT have disguise's ward {{2}}"
    );
    assert_eq!(
        (
            stack_obj.characteristics().power,
            stack_obj.characteristics().toughness
        ),
        (Some(2), Some(2))
    );

    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    let bf_obj = engine.state().object(baloth).unwrap();
    assert!(bf_obj.status.contains(crate::object::Status::FACE_DOWN));
    assert!(
        bf_obj.printed_abilities(&engine.lookup).is_empty(),
        "CR 708.2: face-down megamorph permanent has no abilities"
    );

    // Turn Baloth face up for {2}{G}: does NOT use stack, gets +1/+1 counter (4/4), fires TurnedFaceUp!
    add_mana(&mut engine, me(), ManaColor::Green, 3);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: baloth,
                ability_index: TURN_FACE_UP,
            },
        )
        .expect("turn megamorph face up");
    let up_obj = engine.state().object(baloth).unwrap();
    assert!(
        !up_obj.status.contains(crate::object::Status::FACE_DOWN),
        "CR 702.37e: turning face up happens immediately without using the stack"
    );
    assert_eq!(up_obj.counters.get(CounterKind::P1P1), 1);
    assert_eq!(
        (
            up_obj.characteristics().power,
            up_obj.characteristics().toughness
        ),
        (Some(4), Some(4)),
        "CR 702.37b: megamorph permanent gets a +1/+1 counter as it turns face up"
    );
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(
        life(&engine, me()),
        24,
        "TurnedFaceUp trigger gained 4 life"
    );

    // Cast Drake face down for {3}, verify non-mana morph_parts (ReturnToHand(&Filter::LAND)).
    add_mana(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: drake })
        .expect("cast drake face down");
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    // Negative: no land on battlefield -> TURN_FACE_UP is NOT offered!
    let (_, legal_no_land) = offer(&engine);
    assert!(
        !legal_no_land.abilities.contains(&(drake, TURN_FACE_UP)),
        "non-mana morph cost cannot be activated without a land to return"
    );

    // Play a Forest onto battlefield -> TURN_FACE_UP becomes legal!
    engine
        .apply(me(), PlayerAction::PlayLand { card: my_forest })
        .expect("play forest");

    let (_, legal_with_land) = offer(&engine);
    assert!(legal_with_land.abilities.contains(&(drake, TURN_FACE_UP)));
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: drake,
                ability_index: TURN_FACE_UP,
            },
        )
        .expect("activate non-mana morph turn-face-up");
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { options, .. } if options.contains(&my_forest))
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .expect("return forest to hand for morph cost");

    assert_eq!(
        engine.state().object(my_forest).unwrap().zone,
        crate::zone::Zone::Hand
    );
    let up_drake = engine.state().object(drake).unwrap();
    assert!(!up_drake.status.contains(crate::object::Status::FACE_DOWN));
    assert_eq!(
        (
            up_drake.characteristics().power,
            up_drake.characteristics().toughness
        ),
        (Some(4), Some(4))
    );
}

#[test]
fn bestow_casts_as_aura_bypasses_creature_lock_attaches_and_survives_host_death() {
    static BUFF_ENCHANTED: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::Enchanted,
        layer: Layer::PtModify,
        modifier: Modifier::ModifyPT(2, 2),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static LOCK_CREATURES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::Any,
        layer: Layer::Ability,
        modifier: Modifier::PlayersCantCast {
            who: PlayerRel::EachPlayer,
            filter: &Filter::CREATURE,
        },
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    static DESTROY_CREATURE: &[Effect] = &[Effect::Destroy {
        target: TargetSpec::Object(&Filter::CREATURE),
        no_regen: false,
    }];

    let cards = vec![
        card(
            BESTOW_NYMPH,
            FaceDef {
                types: TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
                subtypes: &[creature::NYMPH],
                bestow: Some(Cost {
                    mana: ManaCost::parse("{3}{W}"),
                    parts: &[],
                }),
                ..creature_face("Graceful Nymph", "{1}{W}", 2, 2)
            },
            KeywordSet::FLYING,
            BUFF_ENCHANTED,
        ),
        card(
            FODDER_BEAR,
            creature_face("Host Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CREATURE_LOCK_PILLAR,
            face("Silent Pillar", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            LOCK_CREATURES,
        ),
        card(
            DESTROY_PROBE,
            face("Slayer Orb", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            const {
                &[free(
                    DESTROY_CREATURE,
                    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
                )]
            },
        ),
    ];

    let mut engine = bench(
        7,
        cards,
        [
            Seat::with(&[FODDER_BEAR, CREATURE_LOCK_PILLAR, DESTROY_PROBE])
                .holding(&[BESTOW_NYMPH]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let nymph = the(&engine, ZoneLocation::Hand(me()), BESTOW_NYMPH);
    let bear = the(&engine, ZoneLocation::Battlefield, FODDER_BEAR);
    add_mana(&mut engine, me(), ManaColor::White, 4);

    // Under "Players can't cast creature spells", Normal is locked out, Bestow is legal!
    let (_, legal) = offer(&engine);
    assert!(
        !legal
            .cast_ways
            .iter()
            .any(|(id, w)| *id == nymph && w.kind == CastModeKind::Normal),
        "Normal creature cast must be blocked by PlayersCantCast(CREATURE)"
    );
    assert!(
        legal
            .cast_ways
            .iter()
            .any(|(id, w)| *id == nymph && w.kind == CastModeKind::Bestow),
        "CR 702.103b: Bestow spell is an Enchantment — Aura, bypassing creature cast lock"
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: nymph })
        .expect("cast nymph bestowed");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .expect("target bear");
    }

    let on_stack = engine.state().object(nymph).unwrap().characteristics();
    assert_eq!(on_stack.types, TypeSet::ENCHANTMENT);
    assert!(on_stack.subtypes.contains(enchantment::AURA));
    assert!(!on_stack.subtypes.contains(creature::NYMPH));
    assert_eq!((on_stack.power, on_stack.toughness), (None, None));

    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    // Resolved as Aura attached to Bear!
    let attached_nymph = engine.state().object(nymph).unwrap();
    assert_eq!(attached_nymph.attached_to, Some(bear));
    assert_eq!(attached_nymph.characteristics().types, TypeSet::ENCHANTMENT);
    let buffed_bear = engine.state().object(bear).unwrap().characteristics();
    assert_eq!(
        (buffed_bear.power, buffed_bear.toughness),
        (Some(4), Some(4))
    );

    // Destroy Bear -> CR 702.103f SBA unattaches Nymph without dying; it becomes a 2/2 Creature!
    let orb = the(&engine, ZoneLocation::Battlefield, DESTROY_PROBE);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: orb,
                ability_index: 0,
            },
        )
        .expect("destroy bear");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .expect("target bear");
    }
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    let freestanding = engine.state().object(nymph).unwrap();
    assert_eq!(
        freestanding.zone,
        crate::zone::Zone::Battlefield,
        "CR 702.103f: bestowed Aura stays on battlefield when host leaves"
    );
    assert_eq!(freestanding.attached_to, None);
    let chars = freestanding.characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::ENCHANTMENT));
    assert!(chars.subtypes.contains(creature::NYMPH));
    assert!(!chars.subtypes.contains(enchantment::AURA));
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)));
}

#[test]
fn bestow_resolves_as_creature_when_target_becomes_illegal_in_response() {
    static DESTROY_CREATURE: &[Effect] = &[Effect::Destroy {
        target: TargetSpec::Object(&Filter::CREATURE),
        no_regen: false,
    }];

    let cards = vec![
        card(
            BESTOW_NYMPH,
            FaceDef {
                types: TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
                subtypes: &[creature::NYMPH],
                bestow: Some(Cost {
                    mana: ManaCost::parse("{3}{W}"),
                    parts: &[],
                }),
                ..creature_face("Graceful Nymph", "{1}{W}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            FODDER_BEAR,
            creature_face("Host Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            DESTROY_PROBE,
            face("Slayer Orb", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            const {
                &[free(
                    DESTROY_CREATURE,
                    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
                )]
            },
        ),
    ];

    let mut engine = bench(
        8,
        cards,
        [
            Seat::with(&[FODDER_BEAR, DESTROY_PROBE]).holding(&[BESTOW_NYMPH]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| in_first_main(e, me()));

    let nymph = the(&engine, ZoneLocation::Hand(me()), BESTOW_NYMPH);
    let bear = the(&engine, ZoneLocation::Battlefield, FODDER_BEAR);
    add_mana(&mut engine, me(), ManaColor::White, 4);

    engine
        .apply(me(), PlayerAction::CastSpell { card: nymph })
        .expect("start cast");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let idx = options
            .iter()
            .position(|o| o.kind == CastModeKind::Bestow)
            .expect("bestow option");
        engine
            .apply(me(), PlayerAction::ChooseMode(idx))
            .expect("choose bestow");
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .expect("target bear");
    }

    // In response to Bestow on the stack, destroy Bear!
    let orb = the(&engine, ZoneLocation::Battlefield, DESTROY_PROBE);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: orb,
                ability_index: 0,
            },
        )
        .expect("destroy bear in response");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .expect("target bear");
    }

    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    let resolved = engine.state().object(nymph).unwrap();
    assert_eq!(
        resolved.zone,
        crate::zone::Zone::Battlefield,
        "CR 702.103e: bestow spell with illegal target resolves as a creature spell instead of fizzling"
    );
    assert_eq!(resolved.attached_to, None);
    assert!(resolved.characteristics().types.contains(TypeSet::CREATURE));
    assert_eq!(
        (
            resolved.characteristics().power,
            resolved.characteristics().toughness
        ),
        (Some(2), Some(2))
    );
}
