//! Wave 11a engine rule & bugfix tests:
//! - Split-card per-face targetability (`CR 709.3a`, `CR 601.2c` — *Spite // Malice*):
//!   an unpointable front face on an empty stack neither hides a legal, affordable
//!   second face nor falsely offers the card when only the front face's mana is floating.
//! - `Sand Golem` opponent-discard delayed graveyard return (`CR 400.7`, `CR 400.7e`):
//!   `Trigger::DiscardedThisByOpponent` records the graveyard arrival version (`V + 1`),
//!   `RegisterDelayedTrigger` binds that incarnation, and `ReturnSourceFromGraveyard`
//!   returns it tapped with a `+1/+1` counter at the next end step while fizzling if
//!   the card leaves the graveyard first.
//! - `Plow Under` simultaneous library placement (`CR 401.4`):
//!   `Effect::PutTargetOnTopOfLibrary` and `Effect::PutTargetOnBottomOfLibrary` ask
//!   each owner of 2+ simultaneously placed cards to choose their order via
//!   `Pending::Arrange`, and ask nothing when at most 1 card goes to each library.
//! - `Midnight Ritual` actual-exile counting & simultaneous `ExileTargetsCreateTokens`
//!   (`CR 608.2b`): `ThisWay::Exiled` counts only cards/permanents actually moved to
//!   exile, and `ExileTargetsCreateTokens` exiles all targets before creating tokens.
//! - Counter, token, and draw replacements + `Effect::RepeatProcess`:
//!   `ModifyCounterPlacement`, `ModifyPlayerCounterPlacement`, `AdditionalTokenCreation`,
//!   `SubstituteTokenCreation`, `ManufactorTokenCreation`, `ReplaceDrawWithDraws`,
//!   `OpponentsExtraDrawsYouDraw`, `SkipDrawInstead`, `WinOnEmptyLibraryDraw`,
//!   `ReplaceDrawLookTop`, `ReplaceDrawExileTop`, and `Effect::RepeatProcess`.

use super::*;
use crate::choice::{ArrangePlace, ArrangePrompt, CastModeKind, ChoicePrompt, YesNoPrompt};
use baylee_cards_dsl::prelude::{spell, triggered};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, CounterReplaceOp, DelayedStep, DelayedTriggerWhen, Effect,
    Filter, PlayerRel, ReplacementRule, TargetReq, TargetSpec, ThisWay, TokenDef, Trigger,
    TriggerRemember,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::artifact::{CLUE, FOOD, TREASURE};
use baylee_core::generated::subtypes::creature::{ANGEL, SQUIRREL, ZOMBIE};
use baylee_core::mana::ManaColor;

const SPITE_MALICE_SPLIT: u32 = 9400;
const GREEN_BEAR: u32 = 9401;
const SAND_GOLEM_SYNTH: u32 = 9402;
const OPP_DISCARD_RIG: u32 = 9403;
const TUCK_TOP_TWO: u32 = 9404;
const TUCK_BOTTOM_TWO: u32 = 9405;
const LAND_A: u32 = 9406;
const LAND_B: u32 = 9407;
const EXILE_SWAP_RIG: u32 = 9408;
const TOKEN_DOUBLER: u32 = 9409;
const MIDNIGHT_RITUAL_RIG: u32 = 9410;
const BOUNCE_RIG: u32 = 9411;
const SCALES_PERM: u32 = 9412;
const VORINCLEX_PERM: u32 = 9413;
const BEETLE_PERM: u32 = 9414;
const COUNTER_PROBE: u32 = 9415;
const TOOK_PERM: u32 = 9416;
const CHATTER_PERM: u32 = 9417;
const VISITATION_PERM: u32 = 9418;
const MANUFACTOR_PERM: u32 = 9419;
const TOKEN_RIG: u32 = 9420;
const REFLECTION_A: u32 = 9421;
const REFLECTION_B: u32 = 9422;
const THIEF_PERM: u32 = 9423;
const LAB_MAN_PERM: u32 = 9424;
const LICH_PERM: u32 = 9425;
const UBA_PERM: u32 = 9426;
const FAMILIAR_PERM: u32 = 9427;
const DRAW_RIG: u32 = 9428;
const REPEAT_RIG: u32 = 9429;

static NONCREATURE_SPELL: Filter = Filter::And(&[Filter::Any, Filter::Not(&Filter::CREATURE)]);
static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);
static GRAVEYARD_CREATURE: Filter = Filter::CREATURE;
static NON_ANGEL_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasSubtype(ANGEL))]);

static ZOMBIE_2_2: TokenDef = TokenDef {
    name: "Zombie",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[ZOMBIE],
    power: Some(2),
    toughness: Some(2),
    ..TokenDef::DEFAULT
};

static SQUIRREL_1_1: TokenDef = TokenDef {
    name: "Squirrel",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[SQUIRREL],
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

static ANGEL_4_4: TokenDef = TokenDef {
    name: "Angel",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[ANGEL],
    keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    power: Some(4),
    toughness: Some(4),
    ..TokenDef::DEFAULT
};

static TEST_CLUE: TokenDef = TokenDef {
    name: "Clue",
    types: TypeSet::ARTIFACT,
    subtypes: &[CLUE],
    ..TokenDef::DEFAULT
};

static TEST_FOOD: TokenDef = TokenDef {
    name: "Food",
    types: TypeSet::ARTIFACT,
    subtypes: &[FOOD],
    ..TokenDef::DEFAULT
};

static TEST_TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    types: TypeSet::ARTIFACT,
    subtypes: &[TREASURE],
    ..TokenDef::DEFAULT
};

fn refresh_offer(engine: &mut Bench) {
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
    refresh_offer(engine);
}

fn split_spite_malice() -> &'static CardDef {
    static SPITE_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&NONCREATURE_SPELL))),
    )];
    static MALICE_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&NONBLACK_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONBLACK_CREATURE))),
    )];
    let left = FaceDef {
        abilities: SPITE_ABILITIES,
        ..face("Spite", "{3}{U}", TypeSet::INSTANT)
    };
    let right = FaceDef {
        abilities: MALICE_ABILITIES,
        ..face("Malice", "{3}{B}", TypeSet::INSTANT)
    };
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(SPITE_MALICE_SPLIT),
        oracle_id: "test-spite-malice",
        scryfall_id: "test-spite-malice",
        faces: Box::leak(Box::new([left, right])),
        color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

#[test]
fn split_card_back_face_castable_when_front_face_has_no_legal_target() {
    let bear = card(
        GREEN_BEAR,
        creature_face("Green Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let split = split_spite_malice();

    let mut engine = bench(
        1101,
        vec![split, bear],
        [
            Seat::default().holding(&[SPITE_MALICE_SPLIT]),
            Seat::with(&[GREEN_BEAR]),
        ],
    );
    to_main(&mut engine, me());
    let split_id = the(&engine, ZoneLocation::Hand(me()), SPITE_MALICE_SPLIT);
    let bear_id = the(&engine, ZoneLocation::Battlefield, GREEN_BEAR);

    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    let (_, legal_blue_only) = offer(&engine);
    assert!(
        !legal_blue_only.castable.contains(&split_id),
        "face 0 has no legal spell target on an empty stack, so {{3}}{{U}} cannot cast Malice"
    );
    assert!(!legal_blue_only.payable.contains(&split_id));

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    let (_, legal_with_black) = offer(&engine);
    assert!(
        legal_with_black.castable.contains(&split_id),
        "Malice has a legal creature target and {{3}}{{B}} floating"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: split_id })
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let pos = options
            .iter()
            .position(|m| m.kind == CastModeKind::Face(1))
            .expect("Malice face offered");
        engine.apply(me(), PlayerAction::ChooseMode(pos)).unwrap();
    }
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
        engine.state().object(bear_id).unwrap().zone,
        Zone::Graveyard
    );

    let mut empty_engine = bench(
        1102,
        vec![split, bear],
        [
            Seat::with(&[super::synthetic::forest()]).holding(&[SPITE_MALICE_SPLIT]),
            Seat::default(),
        ],
    );
    to_main(&mut empty_engine, me());
    let split_empty = the(&empty_engine, ZoneLocation::Hand(me()), SPITE_MALICE_SPLIT);
    add_pool(&mut empty_engine, me(), ManaColor::Blue, 4);
    add_pool(&mut empty_engine, me(), ManaColor::Black, 4);
    let (_, legal_empty) = offer(&empty_engine);
    assert!(!legal_empty.castable.contains(&split_empty));
    assert!(!legal_empty.payable.contains(&split_empty));
}

fn sand_golem_cards() -> Vec<&'static CardDef> {
    static GOLEM_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::DiscardedThisByOpponent,
        &[Effect::RegisterDelayedTrigger {
            when: DelayedTriggerWhen::AtStep {
                step: DelayedStep::EndStep,
                whose: Some(PlayerRel::EachPlayer),
                this_turn_only: false,
                recurring: false,
            },
            condition: None,
            targets: None,
            remember: TriggerRemember::None,
            effects: &[Effect::ReturnSourceFromGraveyard {
                face: None,
                tapped: true,
                owner_control: false,
                counters: Some((CounterKind::P1P1, 1)),
            }],
        }],
    )];
    static DISCARD_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::EachOpponent,
            count: 1,
        }],
        None,
    )];
    vec![
        card(
            SAND_GOLEM_SYNTH,
            creature_face("Sand Golem", "{5}", 3, 3),
            KeywordSet::EMPTY,
            GOLEM_ABILITIES,
        ),
        card(
            OPP_DISCARD_RIG,
            face("Opp Discarder", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DISCARD_ABILITIES,
        ),
    ]
}

#[test]
fn sand_golem_opponent_discard_returns_at_next_end_step() {
    let mut engine = bench(
        1103,
        sand_golem_cards(),
        [
            Seat::default().holding(&[SAND_GOLEM_SYNTH]),
            Seat::with(&[OPP_DISCARD_RIG]),
        ],
    );
    to_main(&mut engine, them());
    let golem_id = the(&engine, ZoneLocation::Hand(me()), SAND_GOLEM_SYNTH);
    let rig_id = the(&engine, ZoneLocation::Battlefield, OPP_DISCARD_RIG);
    activate(&mut engine, them(), rig_id, 0);
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![golem_id],
            },
        )
        .unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state().object(golem_id).unwrap().zone,
        Zone::Graveyard
    );

    walk_until(&mut engine, |e| {
        e.state().object(golem_id).unwrap().zone == Zone::Battlefield
    });
    assert_eq!(engine.state().turn.active, them());
    let returned = engine.state().object(golem_id).unwrap();
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(returned.controller, me());
    assert!(returned.status.contains(crate::object::Status::TAPPED));
    assert_eq!(returned.counters.get(CounterKind::P1P1), 1);
    assert_eq!(returned.characteristics().power, Some(4));
}

#[test]
fn sand_golem_delayed_trigger_fizzles_after_zone_change_cr_400_7() {
    let mut fizzle = bench(
        1104,
        sand_golem_cards(),
        [
            Seat::default().holding(&[SAND_GOLEM_SYNTH]),
            Seat::with(&[OPP_DISCARD_RIG]),
        ],
    );
    to_main(&mut fizzle, them());
    let g2 = the(&fizzle, ZoneLocation::Hand(me()), SAND_GOLEM_SYNTH);
    let r2 = the(&fizzle, ZoneLocation::Battlefield, OPP_DISCARD_RIG);
    activate(&mut fizzle, them(), r2, 0);
    fizzle.apply(them(), PlayerAction::PassPriority).unwrap();
    fizzle.apply(me(), PlayerAction::PassPriority).unwrap();
    fizzle
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![g2] })
        .unwrap();
    settle(&mut fizzle, them());
    let _ = fizzle.state.move_object(
        g2,
        ZoneLocation::Exile(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let _ = fizzle.state.move_object(
        g2,
        ZoneLocation::Graveyard(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    walk_until(&mut fizzle, |e| {
        e.state().turn.active == me() && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        fizzle.state().object(g2).unwrap().zone,
        Zone::Graveyard,
        "new graveyard incarnation must not be returned by stale delayed trigger"
    );
}

fn library_tuck_cards() -> Vec<&'static CardDef> {
    static TOP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::PutTargetOnTopOfLibrary],
        Some(TargetReq::exactly(TargetSpec::Object(&Filter::LAND), 2)),
    )];
    static BOTTOM_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::PutTargetOnBottomOfLibrary],
        Some(TargetReq::exactly(TargetSpec::Object(&Filter::LAND), 2)),
    )];
    vec![
        card(
            TUCK_TOP_TWO,
            face("Plow Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TOP_ABILITIES,
        ),
        card(
            TUCK_BOTTOM_TWO,
            face("Bottom Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BOTTOM_ABILITIES,
        ),
        card(LAND_A, land_face("Land A"), KeywordSet::EMPTY, &[]),
        card(LAND_B, land_face("Land B"), KeywordSet::EMPTY, &[]),
    ]
}

#[test]
fn put_targets_on_top_asks_owner_order_for_multiple_same_owner_cards() {
    let mut engine = bench(
        1105,
        library_tuck_cards(),
        [
            Seat::with(&[TUCK_TOP_TWO, LAND_A]),
            Seat::with(&[LAND_B, LAND_A]),
        ],
    );
    to_main(&mut engine, me());
    let rig = the(&engine, ZoneLocation::Battlefield, TUCK_TOP_TWO);
    let their_lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.owner == them() && o.characteristics().types.contains(TypeSet::LAND)
            })
        })
        .collect();
    assert_eq!(their_lands.len(), 2);
    activate(&mut engine, me(), rig, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: their_lands.clone(),
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::Arrange {
        player,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Pending::Arrange for owner of 2 lands, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, them());
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(piles[0].place, ArrangePlace::LibraryTop);
    engine
        .apply(
            them(),
            PlayerAction::Arrange {
                piles: vec![vec![their_lands[0], their_lands[1]]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let lib = engine.state().zones.list(ZoneLocation::Library(them()));
    assert_eq!(lib[lib.len() - 1], their_lands[0]);
    assert_eq!(lib[lib.len() - 2], their_lands[1]);
}

#[test]
fn put_targets_on_bottom_skips_arrange_for_distinct_owners() {
    let mut split_owners = bench(
        1106,
        library_tuck_cards(),
        [
            Seat::with(&[TUCK_BOTTOM_TWO, LAND_A]),
            Seat::with(&[LAND_B]),
        ],
    );
    to_main(&mut split_owners, me());
    let brig = the(&split_owners, ZoneLocation::Battlefield, TUCK_BOTTOM_TWO);
    let my_land = the(&split_owners, ZoneLocation::Battlefield, LAND_A);
    let opp_land = the(&split_owners, ZoneLocation::Battlefield, LAND_B);
    activate(&mut split_owners, me(), brig, 0);
    split_owners
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_land, opp_land],
            },
        )
        .unwrap();
    settle(&mut split_owners, me());
    assert_eq!(
        split_owners.state().zones.list(ZoneLocation::Library(me()))[0],
        my_land
    );
    assert_eq!(
        split_owners
            .state()
            .zones
            .list(ZoneLocation::Library(them()))[0],
        opp_land
    );
}

fn exile_token_cards() -> Vec<&'static CardDef> {
    static DOUBLER_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleTokenCreation {
            controller_filter: &Filter::ControlledByYou,
        },
    )];
    static SWAP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::ExileTargetsCreateTokens { token: &ZOMBIE_2_2 }],
        Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
    )];
    static BOUNCE_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::bounce(TargetSpec::Object(&Filter::CREATURE))],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static RITUAL_ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::Exile {
                target: TargetSpec::CardInGraveyard(&GRAVEYARD_CREATURE, PlayerRel::You),
            },
            Effect::CreateTokenN {
                token: &ZOMBIE_2_2,
                amount: Amount::ThisWay(ThisWay::Exiled(&Filter::CREATURE)),
            },
        ],
        Some(TargetReq::up_to(
            TargetSpec::CardInGraveyard(&GRAVEYARD_CREATURE, PlayerRel::You),
            2,
        )),
    )];
    vec![
        card(
            TOKEN_DOUBLER,
            creature_face("Doubler Body", "{3}", 2, 2),
            KeywordSet::EMPTY,
            DOUBLER_ABILITIES,
        ),
        card(
            GREEN_BEAR,
            creature_face("Bear", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            EXILE_SWAP_RIG,
            face("Swap Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SWAP_ABILITIES,
        ),
        card(
            BOUNCE_RIG,
            face("Bounce Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BOUNCE_ABILITIES,
        ),
        card(
            MIDNIGHT_RITUAL_RIG,
            face("Ritual Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            RITUAL_ABILITIES,
        ),
    ]
}

fn count_subtype_for(engine: &Bench, who: PlayerId, sub: baylee_core::ids::SubtypeId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == who && o.characteristics().subtypes.contains(sub))
        })
        .count()
}

#[test]
fn exile_targets_create_tokens_exiles_all_before_creating_tokens() {
    let mut engine = bench(
        1107,
        exile_token_cards(),
        [
            Seat::with(&[EXILE_SWAP_RIG, BOUNCE_RIG]),
            Seat::with(&[GREEN_BEAR, TOKEN_DOUBLER]),
        ],
    );
    to_main(&mut engine, me());
    let swap_id = the(&engine, ZoneLocation::Battlefield, EXILE_SWAP_RIG);
    let bounce_id = the(&engine, ZoneLocation::Battlefield, BOUNCE_RIG);
    let bear_id = the(&engine, ZoneLocation::Battlefield, GREEN_BEAR);
    let doubler_id = the(&engine, ZoneLocation::Battlefield, TOKEN_DOUBLER);

    activate(&mut engine, me(), swap_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id, doubler_id],
            },
        )
        .unwrap();
    activate(&mut engine, me(), bounce_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(count_subtype_for(&engine, them(), ZOMBIE), 1);
}

#[test]
fn this_way_exiled_counts_only_cards_actually_moved_to_exile() {
    let mut engine = bench(
        1117,
        exile_token_cards(),
        [
            Seat::with(&[MIDNIGHT_RITUAL_RIG]).holding(&[GREEN_BEAR, GREEN_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let ritual_id = the(&engine, ZoneLocation::Battlefield, MIDNIGHT_RITUAL_RIG);
    let hand_bears = objects(&engine, ZoneLocation::Hand(me()), GREEN_BEAR);
    assert_eq!(hand_bears.len(), 2);
    let (g1, g2) = (hand_bears[0], hand_bears[1]);
    for &id in &[g1, g2] {
        let _ = engine.state.move_object(
            id,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    refresh_offer(&mut engine);
    activate(&mut engine, me(), ritual_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![g1, g2],
            },
        )
        .unwrap();
    let _ = engine.state.move_object(
        g2,
        ZoneLocation::Hand(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    settle(&mut engine, me());
    assert_eq!(count_subtype_for(&engine, me(), ZOMBIE), 1);
}

fn counter_replacement_cards() -> Vec<&'static CardDef> {
    static SCALES_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::YOUR_CREATURE,
            kind: Some(CounterKind::P1P1),
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Add(1),
        },
    )];
    static VORINCLEX_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Replacement(ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::Any,
            kind: None,
            placer: Some(PlayerRel::EachOpponent),
            effect_only: false,
            op: CounterReplaceOp::HalveFloor,
        }),
        AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement {
            recipient: PlayerRel::You,
            kind: Some(CounterKind::Energy),
            placer: Some(PlayerRel::You),
            op: CounterReplaceOp::Add(1),
        }),
        AbilityDef::Replacement(ReplacementRule::ModifyPlayerCounterPlacement {
            recipient: PlayerRel::You,
            kind: Some(CounterKind::Poison),
            placer: Some(PlayerRel::EachOpponent),
            op: CounterReplaceOp::HalveFloor,
        }),
    ];
    static BEETLE_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::YOUR_CREATURE,
            kind: Some(CounterKind::M1M1),
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Sub(u16::MAX),
        },
    )];
    vec![
        card(
            SCALES_PERM,
            face("Scales", "{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SCALES_ABILITIES,
        ),
        card(
            VORINCLEX_PERM,
            face("Vorinclex", "{6}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            VORINCLEX_ABILITIES,
        ),
        card(
            BEETLE_PERM,
            face("Beetle", "{1}{B}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            BEETLE_ABILITIES,
        ),
        card(
            COUNTER_PROBE,
            creature_face("Probe", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

#[test]
fn counter_placement_replacements_modify_permanent_and_player_counters() {
    let mut engine = bench(
        1108,
        counter_replacement_cards(),
        [
            Seat::with(&[SCALES_PERM, VORINCLEX_PERM, BEETLE_PERM, COUNTER_PROBE]),
            Seat::with(&[COUNTER_PROBE]),
        ],
    );
    to_main(&mut engine, me());
    let find_probe = |e: &Bench, who| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|&id| {
                e.state().object(id).is_some_and(|o| {
                    o.controller == who && o.card.is_some_and(|c| c.index.get() == COUNTER_PROBE)
                })
            })
            .unwrap()
    };
    let my_probe = find_probe(&engine, me());
    let opp_probe = find_probe(&engine, them());

    crate::replacement::put_counters_by(&mut engine.state, me(), my_probe, CounterKind::P1P1, 1);
    crate::replacement::put_counters_by(&mut engine.state, them(), my_probe, CounterKind::M1M1, 3);
    assert_eq!(
        engine
            .state()
            .object(my_probe)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        engine
            .state()
            .object(my_probe)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        0
    );

    crate::replacement::put_counters_by(&mut engine.state, them(), opp_probe, CounterKind::P1P1, 3);
    crate::replacement::put_counters_by(
        &mut engine.state,
        them(),
        opp_probe,
        CounterKind::Charge,
        1,
    );
    assert_eq!(
        engine
            .state()
            .object(opp_probe)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(opp_probe)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );

    engine
        .state
        .put_player_counters_by(me(), me(), CounterKind::Energy, 2);
    engine
        .state
        .put_player_counters_by(them(), me(), CounterKind::Poison, 1);
    engine
        .state
        .put_player_counters_by(them(), me(), CounterKind::Poison, 3);
    assert_eq!(engine.state().players[0].energy, 3);
    assert_eq!(engine.state().players[0].poison, 1);
}

fn token_replacement_cards() -> Vec<&'static CardDef> {
    static TOOK_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::AdditionalTokenCreation {
            controller_filter: &Filter::ControlledByYou,
            token_filter: &Filter::Any,
            bonus_token: &TEST_FOOD,
            match_count: false,
            condition: None,
        },
    )];
    static CHATTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::AdditionalTokenCreation {
            controller_filter: &Filter::ControlledByYou,
            token_filter: &Filter::Any,
            bonus_token: &SQUIRREL_1_1,
            match_count: true,
            condition: None,
        },
    )];
    static VISIT_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::SubstituteTokenCreation {
            controller_filter: &Filter::ControlledByYou,
            token_filter: &NON_ANGEL_CREATURE,
            replacement: &ANGEL_4_4,
            condition: None,
        },
    )];
    static MANU_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ManufactorTokenCreation {
            controller_filter: &Filter::ControlledByYou,
            clue: &TEST_CLUE,
            food: &TEST_FOOD,
            treasure: &TEST_TREASURE,
        },
    )];
    static RIG_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::CreateTokenModified {
                token: &SQUIRREL_1_1,
                amount: Amount::Fixed(2),
                tapped: true,
                haste: false,
            }],
            None,
        ),
        free(
            &[Effect::CreateTokenN {
                token: &SQUIRREL_1_1,
                amount: Amount::Fixed(0),
            }],
            None,
        ),
    ];
    vec![
        card(
            TOOK_PERM,
            face("Took", "{2}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            TOOK_ABILITIES,
        ),
        card(
            CHATTER_PERM,
            face("Chatter", "{2}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            CHATTER_ABILITIES,
        ),
        card(
            VISITATION_PERM,
            face("Visitation", "{4}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            VISIT_ABILITIES,
        ),
        card(
            MANUFACTOR_PERM,
            face("Manufactor", "{3}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            MANU_ABILITIES,
        ),
        card(
            TOKEN_RIG,
            face("Token Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            RIG_ABILITIES,
        ),
    ]
}

#[test]
fn token_creation_replacements_compose_additional_substitute_and_manufactor_once_per_event() {
    let mut engine = bench(
        1109,
        token_replacement_cards(),
        [
            Seat::with(&[TOOK_PERM, MANUFACTOR_PERM, TOKEN_RIG]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let rig_id = the(&engine, ZoneLocation::Battlefield, TOKEN_RIG);
    activate(&mut engine, me(), rig_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        3
    );

    activate(&mut engine, me(), rig_id, 0);
    settle(&mut engine, me());
    assert_eq!(count_subtype_for(&engine, me(), SQUIRREL), 2);
    assert_eq!(count_subtype_for(&engine, me(), CLUE), 1);
    assert_eq!(count_subtype_for(&engine, me(), FOOD), 1);
    assert_eq!(count_subtype_for(&engine, me(), TREASURE), 1);

    let mut engine2 = bench(
        1110,
        token_replacement_cards(),
        [
            Seat::with(&[CHATTER_PERM, VISITATION_PERM, TOKEN_RIG]),
            Seat::default(),
        ],
    );
    to_main(&mut engine2, me());
    let rig2 = the(&engine2, ZoneLocation::Battlefield, TOKEN_RIG);
    activate(&mut engine2, me(), rig2, 0);
    settle(&mut engine2, me());
    assert_eq!(count_subtype_for(&engine2, me(), ANGEL), 4);
    assert_eq!(count_subtype_for(&engine2, me(), SQUIRREL), 0);
}

fn draw_replacement_cards() -> Vec<&'static CardDef> {
    static REFL_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawWithDraws {
            who: PlayerRel::You,
            skip_first_in_draw_step: false,
            condition: None,
            count: 2,
        },
    )];
    static THIEF_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::OpponentsExtraDrawsYouDraw,
    )];
    static LAB_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::WinOnEmptyLibraryDraw,
    )];
    static LICH_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawLookTop {
            count: 3,
            rest_to_graveyard: true,
        },
    )];
    static UBA_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ReplaceDrawExileTop {
            who: PlayerRel::EachPlayer,
            face_down: false,
            may_play_this_turn: true,
        },
    )];
    static FAMILIAR_ABILITIES: &[AbilityDef] =
        &[AbilityDef::Replacement(ReplacementRule::SkipDrawInstead {
            who: PlayerRel::You,
            optional: true,
            empty_library_only: false,
        })];
    static DRAW_ABILITIES: &[AbilityDef] = &[
        free(&[Effect::draw(1)], None),
        free(&[Effect::draw(2)], None),
    ];
    vec![
        card(
            REFLECTION_A,
            face("Refl A", "{6}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            REFL_ABILITIES,
        ),
        card(
            REFLECTION_B,
            face("Refl B", "{6}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            REFL_ABILITIES,
        ),
        card(
            THIEF_PERM,
            face("Thief", "{4}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            THIEF_ABILITIES,
        ),
        card(
            LAB_MAN_PERM,
            face("Lab Man", "{3}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            LAB_ABILITIES,
        ),
        card(
            LICH_PERM,
            face("Lich", "{5}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            LICH_ABILITIES,
        ),
        card(
            UBA_PERM,
            face("Uba Mask", "{4}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            UBA_ABILITIES,
        ),
        card(
            FAMILIAR_PERM,
            face("Familiar", "{1}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            FAMILIAR_ABILITIES,
        ),
        card(
            DRAW_RIG,
            face("Draw Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DRAW_ABILITIES,
        ),
    ]
}

#[test]
fn draw_replacements_handle_thought_reflection_notion_thief_and_lab_maniac() {
    let mut e1 = bench(
        1111,
        draw_replacement_cards(),
        [
            Seat::with(&[REFLECTION_A, REFLECTION_B, DRAW_RIG]),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let before = e1.state().zones.list(ZoneLocation::Hand(me())).len();
    let d1 = the(&e1, ZoneLocation::Battlefield, DRAW_RIG);
    activate(&mut e1, me(), d1, 0);
    e1.apply(me(), PlayerAction::PassPriority).unwrap();
    e1.apply(them(), PlayerAction::PassPriority).unwrap();
    let first_refl = match e1.pending() {
        Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::DrawReplacement,
            ..
        } => {
            assert_eq!(*player, me());
            assert_eq!(options.len(), 2);
            options[0]
        }
        other => panic!("expected DrawReplacement choice, got {other:?}"),
    };
    e1.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![first_refl],
        },
    )
    .unwrap();
    settle(&mut e1, me());
    assert_eq!(
        e1.state().zones.list(ZoneLocation::Hand(me())).len(),
        before + 4
    );

    let mut e2 = bench(
        1112,
        draw_replacement_cards(),
        [
            Seat::with(&[THIEF_PERM]),
            Seat::with(&[THIEF_PERM, DRAW_RIG]),
        ],
    );
    to_main(&mut e2, me());
    let p1_before = e2.state().zones.list(ZoneLocation::Hand(them())).len();
    let d2 = the(&e2, ZoneLocation::Battlefield, DRAW_RIG);
    e2.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut e2, them(), d2, 0);
    settle(&mut e2, me());
    assert_eq!(
        e2.state().zones.list(ZoneLocation::Hand(them())).len(),
        p1_before + 1
    );

    let mut e3 = bench(
        1113,
        draw_replacement_cards(),
        [Seat::with(&[LAB_MAN_PERM, DRAW_RIG]), Seat::default()],
    );
    to_main(&mut e3, me());
    while e3.state().zones.list(ZoneLocation::Library(me())).len() > 1 {
        e3.state.zones.list_mut(ZoneLocation::Library(me())).pop();
    }
    let d3 = the(&e3, ZoneLocation::Battlefield, DRAW_RIG);
    activate(&mut e3, me(), d3, 1);
    e3.apply(me(), PlayerAction::PassPriority).unwrap();
    e3.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        e3.game_result().map(|r| (r.winner, r.reason)),
        Some((
            Some(crate::win::Victor::Player(me())),
            crate::win::EndReason::EffectWin
        ))
    );
}

#[test]
fn draw_replacements_handle_underrealm_lich_uba_mask_and_obstinate_familiar() {
    let mut e1 = bench(
        1114,
        draw_replacement_cards(),
        [Seat::with(&[LICH_PERM, DRAW_RIG]), Seat::default()],
    );
    to_main(&mut e1, me());
    let hand_b4 = e1.state().zones.list(ZoneLocation::Hand(me())).len();
    let d1 = the(&e1, ZoneLocation::Battlefield, DRAW_RIG);
    activate(&mut e1, me(), d1, 0);
    e1.apply(me(), PlayerAction::PassPriority).unwrap();
    e1.apply(them(), PlayerAction::PassPriority).unwrap();
    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::PutIntoHand,
        ..
    } = e1.pending().clone()
    else {
        panic!("expected Lich PutIntoHand choice, got {:?}", e1.pending());
    };
    assert_eq!(options.len(), 3);
    e1.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![options[1]],
        },
    )
    .unwrap();
    settle(&mut e1, me());
    assert_eq!(
        e1.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_b4 + 1
    );
    assert_eq!(
        e1.state().zones.list(ZoneLocation::Graveyard(me())).len(),
        2
    );

    let mut e2 = bench(
        1116,
        draw_replacement_cards(),
        [
            Seat::with(&[UBA_PERM, FAMILIAR_PERM, DRAW_RIG]),
            Seat::default(),
        ],
    );
    to_main(&mut e2, me());
    let d2 = the(&e2, ZoneLocation::Battlefield, DRAW_RIG);
    activate(&mut e2, me(), d2, 0);
    e2.apply(me(), PlayerAction::PassPriority).unwrap();
    e2.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        e2.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    e2.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut e2, me());
    assert!(e2.state().zones.list(ZoneLocation::Exile(me())).is_empty());

    activate(&mut e2, me(), d2, 0);
    e2.apply(me(), PlayerAction::PassPriority).unwrap();
    e2.apply(them(), PlayerAction::PassPriority).unwrap();
    e2.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut e2, me());
    assert_eq!(e2.state().zones.list(ZoneLocation::Exile(me())).len(), 1);
    assert_eq!(e2.state().play_permissions.len(), 1);
}

fn repeat_process_cards() -> Vec<&'static CardDef> {
    static REPEAT_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::RepeatProcess {
                effects: &[
                    Effect::DiscardRandom {
                        who: PlayerRel::You,
                        count: Amount::Fixed(1),
                    },
                    Effect::IfCondition {
                        condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Discarded), 1),
                        then: &[Effect::LoseLife {
                            target: PlayerRel::EachOpponent,
                            amount: Amount::Fixed(2),
                        }],
                        otherwise: &[],
                    },
                ],
                max_times: Some(Amount::Fixed(5)),
                pre_condition: None,
                post_condition: Some(Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Discarded),
                    1,
                )),
                optional: true,
                optional_before_first: false,
                per_pass_this_way: true,
            }],
            None,
        ),
        free(
            &[Effect::RepeatProcess {
                effects: &[Effect::Mill {
                    target: PlayerRel::You,
                    amount: Amount::Fixed(1),
                }],
                max_times: Some(Amount::Fixed(10)),
                pre_condition: None,
                post_condition: Some(Condition::Not(&Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Milled(&Filter::Any)),
                    2,
                ))),
                optional: false,
                optional_before_first: false,
                per_pass_this_way: false,
            }],
            None,
        ),
    ];
    vec![
        card(
            REPEAT_RIG,
            face("Repeat Rig", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            REPEAT_ABILITIES,
        ),
        card(
            GREEN_BEAR,
            creature_face("Bear", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

#[test]
fn repeat_process_handles_optional_bounded_and_per_pass_this_way_loops() {
    let mut engine = bench(
        1115,
        repeat_process_cards(),
        [
            Seat::with(&[REPEAT_RIG]).holding(&[GREEN_BEAR, GREEN_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let rig_id = the(&engine, ZoneLocation::Battlefield, REPEAT_RIG);

    activate(&mut engine, me(), rig_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    assert_eq!(life(&engine, them()), 18);
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();

    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    assert_eq!(life(&engine, them()), 16);
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();

    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 16);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty()
    );

    let gy_before = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(me()))
        .len();
    activate(&mut engine, me(), rig_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .len(),
        gy_before + 2
    );
}
