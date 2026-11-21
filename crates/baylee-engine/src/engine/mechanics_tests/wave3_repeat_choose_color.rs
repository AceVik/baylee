//! Rule tests for Engine Wave 3:
//! 1. Resolution-chosen color (`Effect::ChooseColor` + `Filter::ChosenColor` +
//!    `Effect::GrantProtectionFromChosenColorUntilEot`) across protection
//!    (CR 702.16b–f, CR 611.2c, CR 704.5m–n), `DestroyAll`, `ReturnAllToHand`,
//!    hand reveal + discard (`Persecute` / `Addle`), and per-player counts.
//! 2. `Effect::RepeatEach` across `RepeatDomain::{EachPlayer, EachOpponent,
//!    EachMatchingPermanent, Times}` in APNAP order (CR 101.4) and deterministic
//!    permanent order, including interactive `Pending` choices inside the loop
//!    body and nested `RepeatEach` preserving outer event bindings.
//! 3. `Effect::ChooseCard` (battlefield & graveyard `1` / `up_to` N into
//!    `Remembered` or moving to hand/battlefield/library),
//!    `Effect::PeekAndReveal` (private look / public reveal of top N, selecting
//!    `min..=max` matching `Filter` into hand/battlefield, placing rest into
//!    graveyard/bottom, and running `if_picked`), and
//!    `Effect::RevealFromHand` / `Effect::RevealTop` (CR 701.20a–b).
#![allow(clippy::too_many_lines)] // Positive and negative rule checks sit side by side per mechanic.

use super::*;
use crate::event::GameEvent;
use baylee_cards_dsl::{
    Amount, ChooseMove, ChooseZone, CostPart, Filter, Modifier, PeekPickDest, PeekRestDest,
    PlayerRel, RepeatDomain, TargetSpec, ZoneSel, spell, static_ability,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::mana::ManaColor;

const KNIGHT_A: u32 = 99_800;
const KNIGHT_LATE: u32 = 99_801;
const RED_AURA: u32 = 99_802;
const GREEN_AURA: u32 = 99_803;
const RED_EQUIP: u32 = 99_804;
const BLESSING_SPELL: u32 = 99_805;
const RED_PING_PROBE: u32 = 99_806;
const RED_SWEEP_PROBE: u32 = 99_807;

const GREEVIL_SAC_SWEEPER: u32 = 99_810;
const RED_ENCHANTMENT: u32 = 99_811;
const BLUE_ENCHANTMENT: u32 = 99_812;
const RED_BEAR: u32 = 99_813;
const BLUE_BEAR: u32 = 99_814;
const WASH_OUT_SPELL: u32 = 99_815;
const SEARING_RAYS_SPELL: u32 = 99_816;

const PERSECUTE_SPELL: u32 = 99_820;
const ADDLE_SPELL: u32 = 99_821;
const BLACK_CARD_1: u32 = 99_822;
const BLACK_CARD_2: u32 = 99_823;
const WHITE_CARD: u32 = 99_824;

const APNAP_REPEATER: u32 = 99_830;
const OPP_REPEATER: u32 = 99_831;
const INTERACTIVE_PERM_REPEATER: u32 = 99_832;
const CREATURE_2_2: u32 = 99_833;
const CREATURE_4_4: u32 = 99_834;
const TIMES_REPEATER: u32 = 99_835;

const SHROUD_TITAN_5_5: u32 = 99_840;
const CHOOSE_REMEMBER_SPELL: u32 = 99_841;
const CHOOSE_GY_REANIMATOR: u32 = 99_842;

const PEEK_COMPANY_SPELL: u32 = 99_850;
const SPELL_MISS_1: u32 = 99_851;
const SPELL_MISS_2: u32 = 99_852;
const SPELL_MISS_3: u32 = 99_853;

const REVEAL_HAND_SPELL: u32 = 99_860;
const REVEAL_TOP_PROBE: u32 = 99_861;
const FOUR_MV_BEAST: u32 = 99_862;

fn colored_face(name: &'static str, cost: &'static str, types: TypeSet, color: Color) -> FaceDef {
    FaceDef {
        color_indicator: ColorSet::of(color),
        ..face(name, cost, types)
    }
}

fn colored_creature_face(
    name: &'static str,
    cost: &'static str,
    power: i16,
    toughness: i16,
    color: Color,
) -> FaceDef {
    FaceDef {
        color_indicator: ColorSet::of(color),
        ..creature_face(name, cost, power, toughness)
    }
}

fn cast_from_hand(engine: &mut Bench, seat: PlayerId, idx: u32) -> ObjectId {
    let found = objects(engine, ZoneLocation::Hand(seat), idx);
    assert!(!found.is_empty(), "expected card {idx} in {seat:?}'s hand");
    let id = found[0];
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {idx} ({id:?}) to be castable"
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast succeeds");
    id
}

fn pass_until_choose_color(engine: &mut Bench, chooser: PlayerId, pick: ManaColor) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, chooser);
                assert!(options.contains(&pick));
                engine
                    .apply(chooser, PlayerAction::ChooseColor(pick))
                    .unwrap();
                return;
            }
            other => panic!("unexpected pending before ChooseColor: {other:?}"),
        }
    }
    panic!("never reached Pending::ChooseColor");
}

/// Deliverable 1a (`Effect::ChooseColor` + `Effect::GrantProtectionFromChosenColorUntilEot`):
/// - Unattaches same-color Aura to graveyard (`CR 702.16c`, `CR 704.5m`) while
///   leaving off-color Aura attached,
/// - Unattaches same-color Equipment while keeping it on the battlefield
///   (`CR 702.16d`, `CR 704.5n`),
/// - Locks the affected set at resolution (`CR 611.2c`) so a creature entering
///   later this turn is NOT protected,
/// - Forbids targeting by sources of the chosen color (`CR 702.16b`) and
///   prevents damage from sources of the chosen color (`CR 702.16e`).
#[test]
fn grant_protection_from_chosen_color_unattaches_aura_and_equipment_and_locks_set() {
    static AURA_SUBS: &[baylee_core::ids::SubtypeId] = &[subtypes::enchantment::AURA];
    static EQUIP_SUBS: &[baylee_core::ids::SubtypeId] = &[subtypes::artifact::EQUIPMENT];
    static AURA_ABILITIES: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
    ];
    static EQUIP_ABILITIES: &[AbilityDef] = &[
        paid(
            Cost::FREE,
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
    ];
    static BLESSING_ABILITIES: &[AbilityDef] = &[spell!(&[
        Effect::ChooseColor,
        Effect::GrantProtectionFromChosenColorUntilEot {
            filter: &Filter::YOUR_CREATURE,
        },
    ])];
    static RED_PING_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static RED_SWEEP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &Filter::CREATURE,
        }],
        None,
    )];

    let mut red_aura_face = colored_face("Red Ward Aura", "{0}", TypeSet::ENCHANTMENT, Color::Red);
    red_aura_face.subtypes = AURA_SUBS;
    let mut green_aura_face =
        colored_face("Green Ward Aura", "{0}", TypeSet::ENCHANTMENT, Color::Green);
    green_aura_face.subtypes = AURA_SUBS;
    let mut red_equip_face = colored_face("Red Blade", "{0}", TypeSet::ARTIFACT, Color::Red);
    red_equip_face.subtypes = EQUIP_SUBS;

    let cards = vec![
        card(
            KNIGHT_A,
            creature_face("Knight A", "{0}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            KNIGHT_LATE,
            creature_face("Knight Late", "{0}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(RED_AURA, red_aura_face, KeywordSet::EMPTY, AURA_ABILITIES),
        card(
            GREEN_AURA,
            green_aura_face,
            KeywordSet::EMPTY,
            AURA_ABILITIES,
        ),
        card(
            RED_EQUIP,
            red_equip_face,
            KeywordSet::EMPTY,
            EQUIP_ABILITIES,
        ),
        card(
            BLESSING_SPELL,
            face("Blessing", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLESSING_ABILITIES,
        ),
        card(
            RED_PING_PROBE,
            colored_face("Red Pinger", "{0}", TypeSet::ARTIFACT, Color::Red),
            KeywordSet::EMPTY,
            RED_PING_ABILITIES,
        ),
        card(
            RED_SWEEP_PROBE,
            colored_face("Red Sweeper", "{0}", TypeSet::ARTIFACT, Color::Red),
            KeywordSet::EMPTY,
            RED_SWEEP_ABILITIES,
        ),
    ];

    let mut engine = bench(
        9_301,
        cards,
        [
            Seat::with(&[KNIGHT_A, RED_EQUIP, RED_PING_PROBE, RED_SWEEP_PROBE]).holding(&[
                RED_AURA,
                GREEN_AURA,
                BLESSING_SPELL,
                KNIGHT_LATE,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let knight = the(&engine, ZoneLocation::Battlefield, KNIGHT_A);
    let equip = the(&engine, ZoneLocation::Battlefield, RED_EQUIP);

    // Enchant Knight A with Red Aura and Green Aura, and attach Red Equipment.
    for aura_idx in [RED_AURA, GREEN_AURA] {
        cast_from_hand(&mut engine, me(), aura_idx);
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseObjects {
                        objects: vec![knight],
                    },
                )
                .unwrap();
        }
        settle(&mut engine, me());
    }
    activate(&mut engine, me(), equip, 0);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![knight],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());

    let green_aura = the(&engine, ZoneLocation::Battlefield, GREEN_AURA);
    assert_eq!(
        engine.state().object(equip).unwrap().attached_to,
        Some(knight)
    );
    assert_eq!(
        engine.state().object(green_aura).unwrap().attached_to,
        Some(knight)
    );

    // Cast Blessing and choose Red on resolution.
    cast_from_hand(&mut engine, me(), BLESSING_SPELL);
    pass_until_choose_color(&mut engine, me(), ManaColor::Red);
    settle(&mut engine, me());

    // Red Aura went to graveyard (CR 702.16c / 704.5m); Green Aura stayed attached!
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RED_AURA).len(),
        0
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), RED_AURA).len(),
        1
    );
    assert_eq!(
        engine.state().object(green_aura).unwrap().attached_to,
        Some(knight)
    );
    // Red Equipment stayed on the battlefield unattached (CR 702.16d / 704.5n)!
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RED_EQUIP).len(),
        1
    );
    assert_eq!(engine.state().object(equip).unwrap().attached_to, None);

    // Cast Knight Late AFTER Blessing resolved: CR 611.2c locks the set at resolution,
    // so Knight Late does NOT gain protection from Red.
    cast_from_hand(&mut engine, me(), KNIGHT_LATE);
    settle(&mut engine, me());
    let late_knight = the(&engine, ZoneLocation::Battlefield, KNIGHT_LATE);

    // Red Ping Probe can target Knight Late, NOT Knight A (CR 702.16b).
    let pinger = the(&engine, ZoneLocation::Battlefield, RED_PING_PROBE);
    activate(&mut engine, me(), pinger, 0);
    if let Pending::ChooseTargets { options: opts, .. } = engine.pending().clone() {
        assert!(
            !opts.contains(&knight),
            "protected Knight A must not be a legal target for Red source"
        );
        assert!(opts.contains(&late_knight));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![late_knight],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(engine.state().object(knight).unwrap().damage, 0);
    assert_eq!(engine.state().object(late_knight).unwrap().damage, 1);

    // Non-targeted Red sweep prevents damage to Knight A (CR 702.16e) while hitting Knight Late.
    let sweeper = the(&engine, ZoneLocation::Battlefield, RED_SWEEP_PROBE);
    activate(&mut engine, me(), sweeper, 0);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(knight).unwrap().damage, 0);
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), KNIGHT_LATE).len(),
        1,
        "unprotected 2/3 Knight Late took 1 + 2 = 3 lethal damage"
    );
}

/// Deliverable 1b (`Filter::ChosenColor` across `DestroyAll`, `ReturnAllToHand`,
/// and per-player `DealDamage` counts):
/// - Sacrificed source (`Root Greevil`) still resolves `ChooseColor` + `DestroyAll`
///   destroying only enchantments of the chosen color,
/// - `Wash Out` bounces only permanents of the chosen color across both seats,
/// - `Searing Rays` deals damage to each player equal to the number of creatures
///   of the chosen color that player controls.
#[test]
fn chosen_color_filter_drives_destroy_all_bounce_sweep_and_counts() {
    static GREEVIL_FILTER: Filter = Filter::And(&[Filter::ENCHANTMENT, Filter::ChosenColor]);
    static RAYS_FILTER: Filter = Filter::And(&[Filter::CREATURE, Filter::ChosenColor]);

    static GREEVIL_ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::SacrificeSelf],
        },
        &[
            Effect::ChooseColor,
            Effect::DestroyAll {
                filter: &GREEVIL_FILTER,
                no_regen: false,
            },
        ],
        None,
    )];
    static WASH_OUT_ABILITIES: &[AbilityDef] = &[spell!(&[
        Effect::ChooseColor,
        Effect::ReturnAllToHand {
            filter: &Filter::ChosenColor,
            opponents_only: false,
        },
    ])];
    static RAYS_ABILITIES: &[AbilityDef] = &[spell!(&[
        Effect::ChooseColor,
        Effect::DealDamage {
            amount: Amount::ControlledCount {
                who: PlayerRel::Recipient,
                filter: &RAYS_FILTER,
            },
            target: TargetSpec::Player(PlayerRel::EachPlayer),
        },
    ])];

    let cards = vec![
        card(
            GREEVIL_SAC_SWEEPER,
            colored_creature_face("Greevil", "{0}", 1, 1, Color::Green),
            KeywordSet::EMPTY,
            GREEVIL_ABILITIES,
        ),
        card(
            RED_ENCHANTMENT,
            colored_face("Red Shrine", "{0}", TypeSet::ENCHANTMENT, Color::Red),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BLUE_ENCHANTMENT,
            colored_face("Blue Shrine", "{0}", TypeSet::ENCHANTMENT, Color::Blue),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            RED_BEAR,
            colored_creature_face("Red Bear", "{0}", 2, 2, Color::Red),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BLUE_BEAR,
            colored_creature_face("Blue Bear", "{0}", 2, 2, Color::Blue),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            WASH_OUT_SPELL,
            face("Wash Out Probe", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            WASH_OUT_ABILITIES,
        ),
        card(
            SEARING_RAYS_SPELL,
            face("Searing Rays Probe", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            RAYS_ABILITIES,
        ),
    ];

    let mut engine = bench(
        9_302,
        cards,
        [
            Seat::with(&[GREEVIL_SAC_SWEEPER, RED_ENCHANTMENT, RED_BEAR, BLUE_BEAR])
                .holding(&[SEARING_RAYS_SPELL, WASH_OUT_SPELL])
                .at(20),
            Seat::with(&[RED_ENCHANTMENT, BLUE_ENCHANTMENT, RED_BEAR, RED_BEAR]).at(20),
        ],
    );
    to_main(&mut engine, me());

    // 1. Activate Greevil (sacrificing itself) and choose Red: both Red enchantments
    // die, while Blue enchantment and all Red/Blue creatures survive!
    let greevil = the(&engine, ZoneLocation::Battlefield, GREEVIL_SAC_SWEEPER);
    activate(&mut engine, me(), greevil, 0);
    pass_until_choose_color(&mut engine, me(), ManaColor::Red);
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RED_ENCHANTMENT).len(),
        0
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, BLUE_ENCHANTMENT).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RED_BEAR).len(),
        3
    );

    // 2. Cast Searing Rays and choose Red: me() controls 1 Red Bear (takes 1),
    // them() controls 2 Red Bears (takes 2).
    cast_from_hand(&mut engine, me(), SEARING_RAYS_SPELL);
    pass_until_choose_color(&mut engine, me(), ManaColor::Red);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 19);
    assert_eq!(life(&engine, them()), 18);

    // 3. Cast Wash Out and choose Blue: Blue Bear (me) and Blue Enchantment (them)
    // bounce to their owners' hands; all 3 Red Bears stay on the battlefield!
    cast_from_hand(&mut engine, me(), WASH_OUT_SPELL);
    pass_until_choose_color(&mut engine, me(), ManaColor::Blue);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, BLUE_BEAR).len(),
        0
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, BLUE_ENCHANTMENT).len(),
        0
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), BLUE_BEAR).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(them()), BLUE_ENCHANTMENT).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RED_BEAR).len(),
        3
    );
}

/// Deliverable 1c (`Filter::ChosenColor` across `Persecute` & `Addle` hand discard):
/// - `Persecute`: target player reveals their hand and discards all cards of the
///   chosen color,
/// - `Addle`: target player reveals their hand and controller chooses 1 card of
///   the chosen color to discard.
#[test]
fn chosen_color_reveal_hand_and_discard_all_or_chosen_card() {
    static PERSECUTE_ABILITIES: &[AbilityDef] = &[spell!(
        &[
            Effect::ChooseColor,
            Effect::RevealHand {
                who: PlayerRel::Chosen,
            },
            Effect::DiscardMatching {
                who: PlayerRel::Chosen,
                filter: &Filter::ChosenColor,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    static ADDLE_ABILITIES: &[AbilityDef] = &[spell!(
        &[
            Effect::ChooseColor,
            Effect::RevealHandDiscard {
                filter: &Filter::ChosenColor,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];

    let cards = vec![
        card(
            PERSECUTE_SPELL,
            face("Persecute Probe", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            PERSECUTE_ABILITIES,
        ),
        card(
            ADDLE_SPELL,
            face("Addle Probe", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            ADDLE_ABILITIES,
        ),
        card(
            BLACK_CARD_1,
            colored_face("Black Card 1", "{1}{B}", TypeSet::SORCERY, Color::Black),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BLACK_CARD_2,
            colored_creature_face("Black Card 2", "{2}{B}", 2, 2, Color::Black),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            WHITE_CARD,
            colored_face("White Card", "{1}{W}", TypeSet::INSTANT, Color::White),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        9_303,
        cards,
        [
            Seat::default().holding(&[PERSECUTE_SPELL, ADDLE_SPELL]),
            Seat::default().holding(&[BLACK_CARD_1, BLACK_CARD_2, WHITE_CARD]),
        ],
    );
    to_main(&mut engine, me());

    // Cast Persecute targeting them(), choosing Black.
    cast_from_hand(&mut engine, me(), PERSECUTE_SPELL);
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
    }
    pass_until_choose_color(&mut engine, me(), ManaColor::Black);
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(them()), BLACK_CARD_1).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(them()), BLACK_CARD_2).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(them()), WHITE_CARD).len(),
        1,
        "White card must remain in hand when Black was chosen"
    );
    assert!(
        engine
            .journal()
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::Revealed { player, .. } if player == them())),
        "target player's hand must be publicly revealed"
    );

    // Cast Addle targeting them(), choosing White: controller selects the White card.
    cast_from_hand(&mut engine, me(), ADDLE_SPELL);
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
    }
    pass_until_choose_color(&mut engine, me(), ManaColor::White);
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, me());
        assert_eq!(options.len(), 1);
        engine
            .apply(me(), PlayerAction::ChooseObjects { objects: options })
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(them()), WHITE_CARD).len(),
        1
    );
}

/// Deliverable 2a (`Effect::RepeatEach` APNAP ordering on opponent's turn +
/// `EachOpponent` filtering):
/// - On `them()`'s turn (`active == them()`), `me()` activates `RepeatEach` with
///   `RepeatDomain::EachPlayer`: `them()` (active player) loses life BEFORE `me()`
///   (non-active player) per CR 101.4!
/// - `RepeatDomain::EachOpponent` affects only opponents of `me()`.
#[test]
fn repeat_each_iterates_players_in_apnap_order_even_on_opponents_turn() {
    static APNAP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RepeatEach {
            domain: RepeatDomain::EachPlayer,
            effects: &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::EventPlayer,
            }],
        }],
        None,
    )];
    static OPP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RepeatEach {
            domain: RepeatDomain::EachOpponent,
            effects: &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::EventPlayer,
            }],
        }],
        None,
    )];

    let cards = vec![
        card(
            APNAP_REPEATER,
            face("APNAP Repeater", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            APNAP_ABILITIES,
        ),
        card(
            OPP_REPEATER,
            face("Opp Repeater", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            OPP_ABILITIES,
        ),
    ];

    let mut engine = bench(
        9_304,
        cards,
        [
            Seat::with(&[APNAP_REPEATER, OPP_REPEATER]).at(20),
            Seat::default().at(20),
        ],
    );
    // Advance to `them()`'s main phase, then pass once so `me()` holds priority
    // during `them()`'s turn (`active == them()`).
    to_main(&mut engine, them());
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().turn.active, them());

    let before_journal = engine.journal().entries().len();
    let apnap = the(&engine, ZoneLocation::Battlefield, APNAP_REPEATER);
    activate(&mut engine, me(), apnap, 0);
    walk_until(&mut engine, |e| {
        e.state().zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });

    assert_eq!(life(&engine, them()), 18);
    assert_eq!(life(&engine, me()), 18);

    let life_losses: Vec<PlayerId> = engine.journal().entries()[before_journal..]
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::LifeChanged {
                player, old, new, ..
            } if new < old => Some(player),
            _ => None,
        })
        .collect();
    assert_eq!(
        life_losses,
        vec![them(), me()],
        "CR 101.4 APNAP: active player (them) must execute first, then non-active (me)"
    );

    // Now test `EachOpponent` on `me()`'s turn: only `them()` loses 3 life.
    to_main(&mut engine, me());
    let opp_rep = the(&engine, ZoneLocation::Battlefield, OPP_REPEATER);
    activate(&mut engine, me(), opp_rep, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 18);
    assert_eq!(life(&engine, them()), 15);
}

/// Deliverable 2b (`Effect::RepeatEach` with interactive `Pending` choices inside
/// `EachMatchingPermanent` + `Times(Amount)` + nested preservation):
/// - Iterates matching permanents in deterministic order, binding `EventObject`
///   (`Amount::EventPower`) on each pass while pausing on `Pending::YesNo` inside
///   the loop body!
/// - `RepeatDomain::Times` runs exact count (and `0` when count is `0`).
#[test]
fn repeat_each_supports_interactive_choices_and_bounded_times() {
    static PERM_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RepeatEach {
            domain: RepeatDomain::EachMatchingPermanent(&Filter::YOUR_CREATURE),
            effects: &[Effect::MayDo {
                effects: &[Effect::GainLife {
                    amount: Amount::EventPower,
                }],
            }],
        }],
        None,
    )];
    static TIMES_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RepeatEach {
            domain: RepeatDomain::Times(Amount::CountOf {
                filter: &Filter::YOUR_CREATURE,
                zone: ZoneSel::Battlefield,
            }),
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
        }],
        None,
    )];

    let cards = vec![
        card(
            INTERACTIVE_PERM_REPEATER,
            face("Perm Repeater", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PERM_ABILITIES,
        ),
        card(
            TIMES_REPEATER,
            face("Times Repeater", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TIMES_ABILITIES,
        ),
        card(
            CREATURE_2_2,
            creature_face("Bear 2/2", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CREATURE_4_4,
            creature_face("Beast 4/4", "{0}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        9_305,
        cards,
        [
            Seat::with(&[
                INTERACTIVE_PERM_REPEATER,
                TIMES_REPEATER,
                CREATURE_2_2,
                CREATURE_4_4,
            ])
            .at(20),
            Seat::default().at(20),
        ],
    );
    to_main(&mut engine, me());

    let perm_rep = the(
        &engine,
        ZoneLocation::Battlefield,
        INTERACTIVE_PERM_REPEATER,
    );
    activate(&mut engine, me(), perm_rep, 0);

    // Pass priority to start resolving: iteration 1 (2/2) asks `Pending::YesNo` -> accept (`+2`).
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::YesNo { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or YesNo, got {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert!(matches!(engine.pending(), Pending::YesNo { .. }));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();

    // Iteration 2 (4/4) immediately asks `Pending::YesNo` -> decline (`+0`), then accept on a second activation!
    assert!(matches!(engine.pending(), Pending::YesNo { .. }));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);

    // Activate again and accept both (`+2 + +4 = +6` -> `28`).
    activate(&mut engine, me(), perm_rep, 0);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::YesNo { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or YesNo");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 28);

    // `Times(CountOf(YOUR_CREATURE))` = 2 iterations * 2 life = +4 -> `32`.
    let times_rep = the(&engine, ZoneLocation::Battlefield, TIMES_REPEATER);
    activate(&mut engine, me(), times_rep, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 32);
}

/// Deliverable 3a (`Effect::ChooseCard`):
/// - Untargeted battlefield choice selects a Shroud creature (proving CR 115.10a
///   non-targeting!) into `Remembered` (`EventObject`) so a subsequent outer
///   `GainLife { amount: Amount::EventPower }` gains `5` life.
/// - Untargeted graveyard choice (`min: 0, max: Fixed(2)`) moves up to 2 chosen
///   creatures from graveyard onto the battlefield tapped.
#[test]
fn choose_card_remembers_battlefield_choice_and_reanimates_up_to_n_from_graveyard() {
    static REMEMBER_ABILITIES: &[AbilityDef] = &[spell!(&[
        Effect::ChooseCard {
            chooser: PlayerRel::You,
            zone: ChooseZone::Battlefield,
            filter: &Filter::YOUR_CREATURE,
            min: 1,
            max: Amount::Fixed(1),
            movement: ChooseMove::None,
            then: &[],
        },
        Effect::GainLife {
            amount: Amount::EventPower,
        },
    ])];
    static REANIMATE_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::ChooseCard {
        chooser: PlayerRel::You,
        zone: ChooseZone::Graveyard(PlayerRel::You),
        filter: &Filter::CREATURE,
        min: 0,
        max: Amount::Fixed(2),
        movement: ChooseMove::Battlefield { tapped: true },
        then: &[],
    }])];

    let cards = vec![
        card(
            SHROUD_TITAN_5_5,
            creature_face("Shroud Titan", "{0}", 5, 5),
            KeywordSet::SHROUD,
            &[],
        ),
        card(
            CREATURE_2_2,
            creature_face("Bear 2/2", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CREATURE_4_4,
            creature_face("Beast 4/4", "{0}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CHOOSE_REMEMBER_SPELL,
            face("Choose Remember", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REMEMBER_ABILITIES,
        ),
        card(
            CHOOSE_GY_REANIMATOR,
            face("Choose Graveyard", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REANIMATE_ABILITIES,
        ),
    ];

    let mut engine = bench(
        9_306,
        cards,
        [
            Seat::with(&[SHROUD_TITAN_5_5, CREATURE_2_2])
                .holding(&[CHOOSE_REMEMBER_SPELL, CHOOSE_GY_REANIMATOR])
                .at(20),
            Seat::default().at(20),
        ],
    );
    to_main(&mut engine, me());

    let titan = the(&engine, ZoneLocation::Battlefield, SHROUD_TITAN_5_5);
    let bear = the(&engine, ZoneLocation::Battlefield, CREATURE_2_2);

    // 1. Cast Choose Remember: both Shroud Titan and Bear are offered (`ChooseCards`, not targets!).
    cast_from_hand(&mut engine, me(), CHOOSE_REMEMBER_SPELL);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or ChooseCards");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards");
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&titan) && options.contains(&bear));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![titan],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 25);

    // Put both creatures into graveyard, then cast Choose Graveyard (`up to 2` -> battlefield tapped).
    for id in [titan, bear] {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Graveyard(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    cast_from_hand(&mut engine, me(), CHOOSE_GY_REANIMATOR);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or ChooseCards");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected graveyard ChooseCards");
    };
    assert_eq!((min, max), (0, 2));
    assert_eq!(options.len(), 2);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    settle(&mut engine, me());

    let titan_bf = the(&engine, ZoneLocation::Battlefield, SHROUD_TITAN_5_5);
    let bear_bf = the(&engine, ZoneLocation::Battlefield, CREATURE_2_2);
    assert!(
        engine
            .state()
            .object(titan_bf)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        engine
            .state()
            .object(bear_bf)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

/// Deliverable 3b (`Effect::PeekAndReveal`):
/// Privately looks at top 5 cards (`Engine::inspected_cards` shows all 5 to `me()`),
/// offers up to 2 matching creatures, reveals ONLY the chosen creatures (`CR 701.20a`),
/// puts them onto the battlefield, puts the unchosen 3 into the graveyard, and
/// executes `if_picked`.
#[test]
fn peek_and_reveal_inspects_privately_reveals_chosen_and_places_rest() {
    static COMPANY_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::PeekAndReveal {
        library: PlayerRel::You,
        count: Amount::Fixed(5),
        reveal_all: false,
        reveal_chosen: true,
        filter: &Filter::CREATURE,
        min_pick: 0,
        max_pick: 2,
        picked: PeekPickDest::Battlefield { tapped: false },
        rest: PeekRestDest::Graveyard,
        if_picked: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
    }])];

    let cards = vec![
        card(
            PEEK_COMPANY_SPELL,
            face("Peek Company", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COMPANY_ABILITIES,
        ),
        card(
            CREATURE_2_2,
            creature_face("Bear 2/2", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CREATURE_4_4,
            creature_face("Beast 4/4", "{0}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SPELL_MISS_1,
            face("Miss 1", "{1}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SPELL_MISS_2,
            face("Miss 2", "{1}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SPELL_MISS_3,
            face("Miss 3", "{1}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        9_307,
        cards,
        [
            Seat::default()
                .holding(&[
                    PEEK_COMPANY_SPELL,
                    CREATURE_2_2,
                    SPELL_MISS_1,
                    CREATURE_4_4,
                    SPELL_MISS_2,
                    SPELL_MISS_3,
                ])
                .at(20),
            Seat::default().at(20),
        ],
    );
    to_main(&mut engine, me());

    // Move the 5 test cards from hand onto the top of `me()`'s library in known order.
    let mut stacked = Vec::new();
    for idx in [
        SPELL_MISS_3,
        CREATURE_4_4,
        SPELL_MISS_2,
        CREATURE_2_2,
        SPELL_MISS_1,
    ] {
        let id = the(&engine, ZoneLocation::Hand(me()), idx);
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Library(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        stacked.push(id);
    }

    let before_journal = engine.journal().entries().len();
    cast_from_hand(&mut engine, me(), PEEK_COMPANY_SPELL);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or ChooseCards");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    // All 5 top cards are privately visible via `inspected_cards()`, while only the
    // 2 creatures are selectable options!
    assert_eq!(engine.inspected_cards().len(), 5);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards");
    };
    assert_eq!((min, max), (0, 2));
    assert_eq!(options.len(), 2);

    let chosen = options.clone();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, CREATURE_2_2).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, CREATURE_4_4).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), SPELL_MISS_1).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), SPELL_MISS_2).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), SPELL_MISS_3).len(),
        1
    );
    assert_eq!(life(&engine, me()), 23, "`if_picked` gained 3 life");

    // Only the 2 chosen creatures were publicly revealed (never the 3 misses!).
    let revealed: Vec<ObjectId> = engine.journal().entries()[before_journal..]
        .iter()
        .filter_map(|e| match &e.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(revealed.len(), 2);
}

/// Deliverable 3c (`Effect::RevealFromHand` & `Effect::RevealTop`, CR 701.20a–b):
/// - `RevealFromHand` reveals a chosen matching card in place (leaving it in hand)
///   and binds `EventObject` (`Amount::EventManaValue`) for `if_revealed`;
///   declining (`&[]`) skips `if_revealed`.
/// - `RevealTop` reveals the top card of the library in place and branches to
///   `if_matches` or `otherwise`.
#[test]
fn reveal_from_hand_and_reveal_top_reveal_in_place_and_branch() {
    static HAND_REVEAL_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::RevealFromHand {
        who: PlayerRel::You,
        filter: &Filter::CREATURE,
        min: 0,
        max: Amount::Fixed(1),
        if_revealed: &[Effect::GainLife {
            amount: Amount::EventManaValue,
        }],
    }])];
    static TOP_REVEAL_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RevealTop {
            player: PlayerRel::You,
            count: Amount::Fixed(1),
            filter: &Filter::LAND,
            if_matches: &[Effect::GainLife {
                amount: Amount::Fixed(5),
            }],
            otherwise: &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
        }],
        None,
    )];

    let cards = vec![
        card(
            REVEAL_HAND_SPELL,
            face("Reveal Hand Probe", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            HAND_REVEAL_ABILITIES,
        ),
        card(
            REVEAL_TOP_PROBE,
            face("Reveal Top Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TOP_REVEAL_ABILITIES,
        ),
        card(
            FOUR_MV_BEAST,
            creature_face("Four MV Beast", "{2}{G}{G}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SPELL_MISS_1,
            face("Nonland Top", "{1}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        9_308,
        cards,
        [
            Seat::with(&[REVEAL_TOP_PROBE])
                .holding(&[
                    REVEAL_HAND_SPELL,
                    REVEAL_HAND_SPELL,
                    FOUR_MV_BEAST,
                    SPELL_MISS_1,
                ])
                .at(20),
            Seat::default().at(20),
        ],
    );
    to_main(&mut engine, me());

    let beast = the(&engine, ZoneLocation::Hand(me()), FOUR_MV_BEAST);

    // 1. Cast Reveal Hand Probe and decline (`objects: vec![]`) -> 0 life gained.
    cast_from_hand(&mut engine, me(), REVEAL_HAND_SPELL);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or ChooseCards");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20);

    // 2. Cast second Reveal Hand Probe and reveal Four MV Beast -> stays in hand
    // (CR 701.20b) and gains 4 life (`Amount::EventManaValue`)!
    cast_from_hand(&mut engine, me(), REVEAL_HAND_SPELL);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority or ChooseCards");
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![beast],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), FOUR_MV_BEAST).len(),
        1,
        "CR 701.20b: revealing a card does not cause it to leave its zone"
    );

    // 3. Library currently has Forests on top -> `RevealTop` hits `if_matches` (`+5` -> `29`).
    let probe = the(&engine, ZoneLocation::Battlefield, REVEAL_TOP_PROBE);
    activate(&mut engine, me(), probe, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 29);

    // Put `Nonland Top` on top of library -> `RevealTop` hits `otherwise` (`+1` -> `30`)
    // and leaves `Nonland Top` on top of library!
    let nonland = the(&engine, ZoneLocation::Hand(me()), SPELL_MISS_1);
    engine
        .state
        .move_object(
            nonland,
            ZoneLocation::Library(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    activate(&mut engine, me(), probe, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 30);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(me()))
            .last(),
        Some(&nonland)
    );
}
