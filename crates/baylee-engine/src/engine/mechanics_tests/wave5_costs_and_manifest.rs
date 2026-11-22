//! Wave 5 engine rule tests:
//! - `Squad` (`CR 702.157a–b`): repeatable optional additional `Cost` (mana and
//!   non-mana `CostPart`s) creating `n` ETB token copies of the permanent via
//!   stack trigger, including Last Known Information (`CR 608.2h`) when the
//!   original leaves the battlefield before the ETB trigger resolves.
//! - `Offspring` (`CR 702.175a–b`): optional additional `Cost` creating a `1/1`
//!   token copy (`CopyMod::SetPT(1, 1)`) via ETB trigger while preserving
//!   other copiable characteristics/abilities, plus spell-copy inheritance
//!   (`CR 707.10`).
//! - `Fuse` (`CR 702.102a–d`, `CR 709.4d`): split instant/sorcery cards with
//!   `CardDef::fuse` cast from hand via `CastModeKind::Fused` at combined cost
//!   (single `SpellAdjustment` discount), combined stack characteristics,
//!   independent left/right targets (including `CR 608.2b` partial fizzle),
//!   ordered left-then-right resolution, and negative test proving `Fused` is
//!   never offered from graveyard/exile (`CR 702.102a`).
//! - `Manifest` & `Manifest Dread` (`CR 701.40a–g`, `CR 701.62a–b`): face-down
//!   2/2 colorless nameless creatures without Ward `{2}`, `TURN_FACE_UP`
//!   special action for front-face creatures with a mana cost (`CR 701.40b`)
//!   and morph/disguise interaction (`CR 701.40c–d`), noncreature rejection,
//!   instant/sorcery reveal-and-stay-face-down guard (`CR 701.40g`), and
//!   `Effect::ManifestDread` over 2+, 1, and 0 library cards.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, NumberPrompt, TURN_FACE_UP, YesNoPrompt};
use crate::event::GameEvent;
use crate::object::Status;
use baylee_cards_dsl::prelude::{static_ability, triggered};
use baylee_cards_dsl::{Amount, CostPart, Filter, Modifier, PlayerRel, TargetSpec, Trigger};
use baylee_core::color::{Color, ColorSet};
use baylee_core::mana::{ManaColor, ManaPool};

const SQUAD_TROOPER: u32 = 8500;
const SQUAD_ZEALOT: u32 = 8501;
const OFFSPRING_LIZARD: u32 = 8502;
const COPY_SPELL_INSTANT: u32 = 8503;
const KILL_INSTANT: u32 = 8504;
const FUSE_WEAR_TEAR: u32 = 8505;
const ELECTROMANCER: u32 = 8506;
const FLASHBACK_GRANTER: u32 = 8507;
const VANILLA_BEAR: u32 = 8508;
const VANILLA_BEAST: u32 = 8509;
const MANIFEST_PROBE: u32 = 8510;
const DREAD_PROBE: u32 = 8511;
const EXPOSE_PROBE: u32 = 8512;
const GREEN_BALOTH: u32 = 8513;
const WHITE_SEAL: u32 = 8514;
const RED_SPARK: u32 = 8515;
const MORPH_SPHINX: u32 = 8516;
const FACE_UP_WATCHER: u32 = 8517;

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

fn put_on_top(engine: &mut Bench, owner: PlayerId, card_id: ObjectId) {
    engine
        .state
        .move_object(
            card_id,
            ZoneLocation::Library(owner),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move to library top");
}

fn empty_library(engine: &mut Bench, owner: PlayerId) {
    let cards = engine
        .state
        .zones
        .list(ZoneLocation::Library(owner))
        .clone();
    for id in cards {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Exile(owner),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("exile library filler");
    }
}

fn resolve_one(engine: &mut Bench) {
    let top_before = *engine
        .state
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("stack has an object to resolve");
    for _ in 0..32 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
                if !engine
                    .state
                    .zones
                    .list(ZoneLocation::Stack)
                    .contains(&top_before)
                {
                    return;
                }
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            other => panic!("unexpected pending while resolving one stack object: {other:?}"),
        }
    }
    panic!("stack object never resolved");
}

fn drain_stack(engine: &mut Bench) {
    while !engine.state.zones.stack_is_empty()
        || matches!(engine.pending(), Pending::Arrange { .. })
    {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            other => panic!("unexpected pending while draining stack: {other:?}"),
        }
    }
}

fn split_card(index: u32, left: FaceDef, right: FaceDef, fuse: bool) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test-split",
        scryfall_id: "test-split",
        faces: Box::leak(Box::new([left, right])),
        color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        fuse,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

fn wave5_cards() -> Vec<&'static CardDef> {
    static TROOPER_ETB: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]
    )];
    static LIFE_SQUAD_PARTS: [CostPart; 1] = [CostPart::PayLife(2)];
    static COPY_SPELL_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::CopySpellOrAbility {
            target: TargetSpec::Spell(&Filter::Any),
            controller: PlayerRel::You,
            amount: Amount::Fixed(1),
            mods: &[],
            may_choose_new_targets: false,
        }],
        targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static KILL_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static LEFT_BURN_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static RIGHT_TAP_DRAW_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::TapTarget, Effect::draw(1)],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static ELECTROMANCER_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::INSTANT_OR_SORCERY,
        Modifier::SpellsCostLess(1)
    )];
    static FLASHBACK_GRANTER_ABILS: [AbilityDef; 1] = [free(
        &[Effect::GrantFlashback],
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))),
    )];
    static MANIFEST_ABILS: [AbilityDef; 1] = [free(
        &[Effect::ManifestTop {
            who: PlayerRel::You,
            count: Amount::Fixed(3),
        }],
        None,
    )];
    static DREAD_ABILS: [AbilityDef; 1] = [free(&[Effect::ManifestDread], None)];
    static EXPOSE_ABILS: [AbilityDef; 1] = [free(
        &[Effect::TurnFaceUp {
            target: TargetSpec::Object(&Filter::Any),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )];
    static WATCHER_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::TurnedFaceUp,
        &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }]
    )];

    vec![
        card(
            SQUAD_TROOPER,
            FaceDef {
                squad: Some(Cost {
                    mana: ManaCost::parse("{1}{W}"),
                    parts: &[],
                }),
                abilities: &TROOPER_ETB,
                ..creature_face("Squad Trooper", "{1}{W}", 2, 2)
            },
            KeywordSet::VIGILANCE,
            &TROOPER_ETB,
        ),
        card(
            SQUAD_ZEALOT,
            FaceDef {
                squad: Some(Cost {
                    mana: ManaCost::ZERO,
                    parts: &LIFE_SQUAD_PARTS,
                }),
                ..creature_face("Squad Zealot", "{1}{B}", 3, 1)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            OFFSPRING_LIZARD,
            FaceDef {
                offspring: Some(Cost {
                    mana: ManaCost::parse("{2}"),
                    parts: &[],
                }),
                keywords: KeywordSet::TRAMPLE,
                ..creature_face("Offspring Lizard", "{1}{G}", 3, 4)
            },
            KeywordSet::TRAMPLE,
            &[],
        ),
        card(
            COPY_SPELL_INSTANT,
            FaceDef {
                abilities: &COPY_SPELL_ABILS,
                ..face("Fork Spell", "{U}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &COPY_SPELL_ABILS,
        ),
        card(
            KILL_INSTANT,
            FaceDef {
                abilities: &KILL_ABILS,
                ..face("Instant Doom", "{B}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &KILL_ABILS,
        ),
        split_card(
            FUSE_WEAR_TEAR,
            FaceDef {
                abilities: &LEFT_BURN_ABILS,
                ..face("Scorch Half", "{1}{R}", TypeSet::INSTANT)
            },
            FaceDef {
                abilities: &RIGHT_TAP_DRAW_ABILS,
                ..face("Freeze Half", "{1}{U}", TypeSet::INSTANT)
            },
            true,
        ),
        card(
            ELECTROMANCER,
            FaceDef {
                abilities: &ELECTROMANCER_ABILS,
                ..creature_face("Spell Discounter", "{1}{U}", 2, 2)
            },
            KeywordSet::EMPTY,
            &ELECTROMANCER_ABILS,
        ),
        card(
            FLASHBACK_GRANTER,
            FaceDef {
                abilities: &FLASHBACK_GRANTER_ABILS,
                ..face("Past Archive", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &FLASHBACK_GRANTER_ABILS,
        ),
        card(
            VANILLA_BEAR,
            creature_face("Target Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            VANILLA_BEAST,
            creature_face("Target Beast", "{2}{G}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MANIFEST_PROBE,
            FaceDef {
                abilities: &MANIFEST_ABILS,
                ..face("Manifest Relic", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &MANIFEST_ABILS,
        ),
        card(
            DREAD_PROBE,
            FaceDef {
                abilities: &DREAD_ABILS,
                ..face("Dread Relic", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &DREAD_ABILS,
        ),
        card(
            EXPOSE_PROBE,
            FaceDef {
                abilities: &EXPOSE_ABILS,
                ..face("Expose Relic", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &EXPOSE_ABILS,
        ),
        card(
            GREEN_BALOTH,
            FaceDef {
                abilities: &WATCHER_ABILS,
                ..creature_face("Manifested Baloth", "{1}{G}", 3, 3)
            },
            KeywordSet::TRAMPLE,
            &WATCHER_ABILS,
        ),
        card(
            WHITE_SEAL,
            FaceDef {
                abilities: &WATCHER_ABILS,
                ..face("Manifested Seal", "{1}{W}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            &WATCHER_ABILS,
        ),
        card(
            RED_SPARK,
            FaceDef {
                abilities: &LEFT_BURN_ABILS,
                ..face("Manifested Spark", "{R}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &LEFT_BURN_ABILS,
        ),
        card(
            MORPH_SPHINX,
            FaceDef {
                morph: Some(ManaCost::parse("{2}{U}")),
                ..creature_face("Morphing Sphinx", "{4}{U}{U}", 4, 5)
            },
            KeywordSet::FLYING,
            &[],
        ),
        card(
            FACE_UP_WATCHER,
            FaceDef {
                abilities: &WATCHER_ABILS,
                ..face("Face-Up Watcher", "{1}{G}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            &WATCHER_ABILS,
        ),
    ]
}

/// `Squad` (`CR 702.157a–b`, `CR 608.2h`):
/// - Paying `0` times creates no token copies.
/// - Paying `2` times (`NumberPrompt::Squad`) creates `2` 2/2 Vigilance token
///   copies via an ETB stack trigger (firing each copy's own ETB life-gain
///   trigger too!).
/// - Non-mana `Squad` (`PayLife(2)`) paid `2` times costs `4` life and, even if
///   the original creature is destroyed in response to its Squad ETB trigger,
///   still creates `2` 3/1 token copies using Last Known Information.
#[test]
fn wave5_squad_pays_zero_or_multiple_times_and_copies_via_etb_including_lki() {
    let mut engine = bench(
        85_001,
        wave5_cards(),
        [
            Seat::with(&[])
                .holding(&[SQUAD_TROOPER, SQUAD_TROOPER, SQUAD_ZEALOT])
                .at(10),
            Seat::with(&[]).holding(&[KILL_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    // 1) Cast first SQUAD_TROOPER with exact {1}{W} -> max Squad = 0, no prompt, 1 Trooper enters!
    let trooper_0 = objects(&engine, ZoneLocation::Hand(me()), SQUAD_TROOPER)[0];
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: trooper_0 })
        .expect("cast squad trooper with 0 squad mana");
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.players[0].life, 11,
        "1 Trooper entered and gained 1 life"
    );
    let troopers_after_zero: Vec<_> = engine
        .state
        .battlefield_view()
        .iter()
        .copied()
        .filter(|&id| {
            engine.state.object(id).is_some_and(|o| {
                engine.state.names.get(o.characteristics().name) == "Squad Trooper"
            })
        })
        .collect();
    assert_eq!(
        troopers_after_zero.len(),
        1,
        "paying Squad 0 times creates 0 tokens"
    );

    // 2) Cast second SQUAD_TROOPER with {3}{W}{W}{W} floating -> max = 2, choose 2!
    let trooper_2 = the(&engine, ZoneLocation::Hand(me()), SQUAD_TROOPER);
    add_pool(&mut engine, me(), ManaColor::White, 3);
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: trooper_2 })
        .expect("cast squad trooper with mana for 2 squad payments");
    match engine.pending() {
        Pending::ChooseNumber {
            player,
            min: 0,
            max: 2,
            reason: NumberPrompt::Squad { .. },
        } if *player == me() => {}
        other => panic!("expected NumberPrompt::Squad (0..=2), got {other:?}"),
    }
    engine
        .apply(me(), PlayerAction::ChooseNumber(2))
        .expect("pay squad twice");
    drain_stack(&mut engine);
    let all_troopers: Vec<_> = engine
        .state
        .battlefield_view()
        .iter()
        .copied()
        .filter(|&id| {
            engine.state.object(id).is_some_and(|o| {
                engine.state.names.get(o.characteristics().name) == "Squad Trooper"
            })
        })
        .collect();
    assert_eq!(
        all_troopers.len(),
        4,
        "1 earlier + 1 original + 2 squad token copies = 4 Squad Troopers"
    );
    let token_troopers = all_troopers
        .iter()
        .filter(|&&id| {
            let o = engine.state.object(id).unwrap();
            o.card.is_none()
                && o.characteristics().power == Some(2)
                && o.characteristics().toughness == Some(2)
                && o.characteristics().keywords.contains(KeywordSet::VIGILANCE)
        })
        .count();
    assert_eq!(
        token_troopers, 2,
        "both Squad copies are 2/2 Vigilance tokens"
    );
    assert_eq!(
        engine.state.players[0].life, 14,
        "original + 2 token copies each fired printed ETB GainLife(1)"
    );

    // 3) Non-mana Squad (`PayLife(2)`) paid 2 times + original killed in response (LKI, CR 608.2h)!
    let zealot = the(&engine, ZoneLocation::Hand(me()), SQUAD_ZEALOT);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: zealot })
        .expect("cast squad zealot");
    match engine.pending() {
        Pending::ChooseNumber {
            player,
            min: 0,
            max,
            reason: NumberPrompt::Squad { .. },
        } if *player == me() && *max >= 2 => {}
        other => panic!("expected NumberPrompt::Squad for life cost, got {other:?}"),
    }
    engine
        .apply(me(), PlayerAction::ChooseNumber(2))
        .expect("pay life squad twice");
    assert_eq!(
        engine.state.players[0].life, 10,
        "paid 4 life for 2 squad payments"
    );
    resolve_one(&mut engine); // resolve SQUAD_ZEALOT -> Squad ETB trigger is now on the stack!
    assert!(
        !engine.state.zones.stack_is_empty(),
        "Squad ETB trigger must use the stack"
    );
    // Pass me() priority so them() kills the original SQUAD_ZEALOT in response!
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    let kill = the(&engine, ZoneLocation::Hand(them()), KILL_INSTANT);
    add_pool(&mut engine, them(), ManaColor::Black, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: kill })
        .expect("cast kill in response to squad ETB");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![zealot],
                players: vec![],
            },
        )
        .expect("target original zealot");
    resolve_one(&mut engine); // resolve KILL_INSTANT -> zealot is in graveyard!
    assert_eq!(
        engine.state.object(zealot).unwrap().zone,
        Zone::Graveyard,
        "original zealot died before Squad ETB resolved"
    );
    drain_stack(&mut engine); // resolve Squad ETB trigger via LKI!
    let zealot_tokens: Vec<_> = engine
        .state
        .battlefield_view()
        .iter()
        .copied()
        .filter(|&id| {
            engine.state.object(id).is_some_and(|o| {
                o.card.is_none()
                    && engine.state.names.get(o.characteristics().name) == "Squad Zealot"
                    && o.characteristics().power == Some(3)
                    && o.characteristics().toughness == Some(1)
            })
        })
        .collect();
    assert_eq!(
        zealot_tokens.len(),
        2,
        "Squad ETB trigger creates 2 3/1 token copies via LKI even after original died"
    );
}

/// `Offspring` (`CR 702.175a–b`, `CR 707.10`):
/// - Declining `YesNoPrompt::Offspring` creates no token copy.
/// - Accepting `YesNoPrompt::Offspring` creates a `1/1` token copy that retains
///   `KeywordSet::TRAMPLE`, green color, and mana value `2`.
/// - Copying an Offspring-paid permanent spell on the stack (`CR 707.10`) copies
///   the Offspring payment decision, so the token permanent the spell-copy
///   becomes ALSO triggers Offspring and creates its own `1/1` token copy!
#[test]
fn wave5_offspring_creates_one_one_token_copy_and_propagates_to_spell_copies() {
    let mut engine = bench(
        85_002,
        wave5_cards(),
        [
            Seat::with(&[]).holding(&[OFFSPRING_LIZARD, OFFSPRING_LIZARD, COPY_SPELL_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // 1) Decline Offspring -> 3/4 Lizard enters alone!
    let liz_0 = objects(&engine, ZoneLocation::Hand(me()), OFFSPRING_LIZARD)[0];
    add_pool(&mut engine, me(), ManaColor::Green, 4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: liz_0 })
        .expect("cast lizard 1");
    match engine.pending() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::Offspring { .. },
            ..
        } if *player == me() => {}
        other => panic!("expected YesNoPrompt::Offspring, got {other:?}"),
    }
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("decline offspring");
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.battlefield_view().len(),
        1,
        "declining Offspring leaves only the 3/4 original"
    );

    // 2) Accept Offspring AND copy the creature spell on the stack!
    let liz_1 = the(&engine, ZoneLocation::Hand(me()), OFFSPRING_LIZARD);
    let fork = the(&engine, ZoneLocation::Hand(me()), COPY_SPELL_INSTANT);
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: liz_1 })
        .expect("cast lizard 2");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept offspring");
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: fork })
        .expect("cast copy spell");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![liz_1],
                players: vec![],
            },
        )
        .expect("target offspring lizard spell");
    drain_stack(&mut engine);
    let lizards: Vec<_> = engine
        .state
        .battlefield_view()
        .iter()
        .copied()
        .filter(|&id| {
            engine.state.object(id).is_some_and(|o| {
                engine.state.names.get(o.characteristics().name) == "Offspring Lizard"
            })
        })
        .collect();
    assert_eq!(
        lizards.len(),
        5,
        "expected three 3/4 Lizards and two 1/1 Offspring tokens"
    );
    let one_ones: Vec<_> = lizards
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state.object(id).unwrap();
            o.card.is_none()
                && o.characteristics().power == Some(1)
                && o.characteristics().toughness == Some(1)
                && o.characteristics().keywords.contains(KeywordSet::TRAMPLE)
                && o.characteristics().colors.contains(Color::Green)
                && o.characteristics().mana_value() == 2
        })
        .collect();
    assert_eq!(
        one_ones.len(),
        2,
        "both the original and the spell-copy produced 1/1 green Trample Offspring tokens"
    );
}

/// `Fuse` (`CR 702.102a–d`, `CR 709.4d`, `CR 608.2b`):
/// - Offered from hand as `CastModeKind::Fused` at combined cost (`{1}{R}` +
///   `{1}{U}` = `{2}{R}{U}`, reduced once by `SpellsCostLess(1)` to `{1}{R}{U}`).
/// - Carries combined Red + Blue colors and MV `4` on the stack (`CR 702.102b`).
/// - Prompts left target then right target, resolves left (2 damage) then right
///   (tap + draw 1), and survives partial target removal (`CR 608.2b`).
/// - Negative (`CR 702.102a`): when granted flashback in the graveyard, `Fused`
///   is NOT offered!
#[test]
fn wave5_fuse_casts_combined_halves_from_hand_only_and_resolves_left_then_right() {
    let mut engine = bench(
        85_003,
        wave5_cards(),
        [
            Seat::with(&[ELECTROMANCER, FLASHBACK_GRANTER])
                .holding(&[FUSE_WEAR_TEAR, FUSE_WEAR_TEAR]),
            Seat::with(&[VANILLA_BEAR, VANILLA_BEAST, VANILLA_BEAR, VANILLA_BEAST])
                .holding(&[KILL_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let bears = objects(&engine, ZoneLocation::Battlefield, VANILLA_BEAR);
    let beasts = objects(&engine, ZoneLocation::Battlefield, VANILLA_BEAST);
    let split_1 = objects(&engine, ZoneLocation::Hand(me()), FUSE_WEAR_TEAR)[0];

    // 1) With ELECTROMANCER (-{1}), Fused costs {1}{R}{U} (combined {2}{R}{U} - {1} once)!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: split_1 })
        .expect("begin casting fuse split card");
    let fused_idx = match engine.pending() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| o.kind == CastModeKind::Fused)
                .unwrap_or_else(|| panic!("expected CastModeKind::Fused in {options:?}"))
                .index
        }
        other => panic!("expected ChooseCastMode, got {other:?}"),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(fused_idx as usize))
        .expect("choose Fused mode");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bears[0]],
                players: vec![],
            },
        )
        .expect("choose left half target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beasts[0]],
                players: vec![],
            },
        )
        .expect("choose right half target");

    let on_stack = engine.state.object(split_1).unwrap();
    assert_eq!(on_stack.zone, Zone::Stack);
    assert!(on_stack.characteristics().colors.contains(Color::Red));
    assert!(on_stack.characteristics().colors.contains(Color::Blue));
    assert_eq!(on_stack.characteristics().mana_value(), 4);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        0,
        "spent exact {{1}}{{R}}{{U}}"
    );

    let hand_before = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.object(bears[0]).unwrap().zone,
        Zone::Graveyard,
        "left half dealt 2 lethal damage to Bear 0"
    );
    assert!(
        engine
            .state
            .object(beasts[0])
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "right half tapped Beast 0"
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "right half drew 1 card"
    );
    assert_eq!(engine.state.object(split_1).unwrap().zone, Zone::Graveyard);

    // 2) Negative (`CR 702.102a`): grant flashback to split_1 in graveyard ->
    //    `CastModeKind::Fused` MUST NOT be offered from graveyard!
    let granter = the(&engine, ZoneLocation::Battlefield, FLASHBACK_GRANTER);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: granter,
                ability_index: 0,
            },
        )
        .expect("activate flashback granter");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![split_1],
                players: vec![],
            },
        )
        .expect("target split_1 in graveyard");
    drain_stack(&mut engine);
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let (_, legal_gy) = offer(&engine);
    assert!(
        !legal_gy
            .cast_ways
            .iter()
            .any(|(id, w)| *id == split_1 && w.kind == CastModeKind::Fused),
        "CastModeKind::Fused must never be offered from graveyard (CR 702.102a)"
    );

    // 3) Partial fizzle (`CR 608.2b`): cast second FUSE_WEAR_TEAR from hand targeting
    //    bears[1] and beasts[1]; opponent kills bears[1] in response -> right half STILL resolves!
    let split_2 = the(&engine, ZoneLocation::Hand(me()), FUSE_WEAR_TEAR);
    engine
        .apply(me(), PlayerAction::CastSpell { card: split_2 })
        .expect("cast second fuse card");
    let fused_idx_2 = match engine.pending() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| o.kind == CastModeKind::Fused)
                .expect("Fused offered from hand")
                .index
        }
        other => panic!("expected ChooseCastMode, got {other:?}"),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(fused_idx_2 as usize))
        .expect("choose Fused");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bears[1]],
                players: vec![],
            },
        )
        .expect("left target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beasts[1]],
                players: vec![],
            },
        )
        .expect("right target");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    let kill = the(&engine, ZoneLocation::Hand(them()), KILL_INSTANT);
    add_pool(&mut engine, them(), ManaColor::Black, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: kill })
        .expect("kill left target in response");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bears[1]],
                players: vec![],
            },
        )
        .expect("target bears[1]");
    resolve_one(&mut engine); // resolve KILL_INSTANT -> bears[1] dead
    let hand_before_2 = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    drain_stack(&mut engine); // resolve split_2 -> right half still taps beasts[1] and draws!
    assert!(
        engine
            .state
            .object(beasts[1])
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "right half resolved despite left half's target becoming illegal (CR 608.2b)"
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_2 + 1,
        "right half drew a card"
    );
}

/// `Manifest` (`CR 701.40a–g`):
/// - Manifests top 3 cards as face-down 2/2 colorless nameless creatures WITH NO Ward.
/// - `TURN_FACE_UP` is offered for the front-face creature (`GREEN_BALOTH`, `{1}{G}`),
///   and NOT offered for `WHITE_SEAL` (Enchantment) or `RED_SPARK` (Instant).
/// - `Effect::TurnFaceUp` on `RED_SPARK` reveals it and leaves it face down without
///   firing `Trigger::TurnedFaceUp` (`CR 701.40g`), while `Effect::TurnFaceUp` on
///   `WHITE_SEAL` turns it face up into the Enchantment and fires `Trigger::TurnedFaceUp`!
/// - `MORPH_SPHINX` manifested can turn face up for its `{2}{U}` Morph cost (`CR 701.40c`).
#[test]
fn wave5_manifest_creates_wardless_facedown_creature_turns_creatures_up_and_guards_instants() {
    let mut engine = bench(
        85_004,
        wave5_cards(),
        [
            Seat::with(&[MANIFEST_PROBE, EXPOSE_PROBE, FACE_UP_WATCHER]).holding(&[
                GREEN_BALOTH,
                WHITE_SEAL,
                RED_SPARK,
                MORPH_SPHINX,
            ]),
            Seat::with(&[]).holding(&[KILL_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let baloth = the(&engine, ZoneLocation::Hand(me()), GREEN_BALOTH);
    let seal = the(&engine, ZoneLocation::Hand(me()), WHITE_SEAL);
    let spark = the(&engine, ZoneLocation::Hand(me()), RED_SPARK);
    let sphinx = the(&engine, ZoneLocation::Hand(me()), MORPH_SPHINX);
    put_on_top(&mut engine, me(), sphinx);
    put_on_top(&mut engine, me(), spark);
    put_on_top(&mut engine, me(), seal);
    put_on_top(&mut engine, me(), baloth);
    refresh_offer(&mut engine);

    let probe = the(&engine, ZoneLocation::Battlefield, MANIFEST_PROBE);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate ManifestTop(3)");
    drain_stack(&mut engine);

    for id in [baloth, seal, spark] {
        let o = engine.state.object(id).unwrap();
        assert_eq!(o.zone, Zone::Battlefield);
        assert!(o.status.contains(Status::FACE_DOWN));
        assert_eq!(o.characteristics().power, Some(2));
        assert_eq!(o.characteristics().toughness, Some(2));
        assert!(o.characteristics().colors.is_empty());
        assert_eq!(engine.state.names.get(o.characteristics().name), "");
        assert!(
            o.abilities(&engine.lookup).is_empty(),
            "manifested permanent must have NO Ward or printed abilities while face down"
        );
    }

    // Only baloth (creature card with mana cost) offers TURN_FACE_UP (`CR 701.40b`)!
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    add_pool(&mut engine, me(), ManaColor::White, 2);
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(baloth, TURN_FACE_UP)),
        "manifested creature offers TURN_FACE_UP"
    );
    assert!(
        !legal.abilities.contains(&(seal, TURN_FACE_UP)),
        "manifested enchantment must NOT offer TURN_FACE_UP"
    );
    assert!(
        !legal.abilities.contains(&(spark, TURN_FACE_UP)),
        "manifested instant must NOT offer TURN_FACE_UP"
    );

    // Turn baloth face up via special action -> immediate 3/3 Trample, fires TurnedFaceUp (+5 life)!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: baloth,
                ability_index: TURN_FACE_UP,
            },
        )
        .expect("turn manifested baloth face up");
    assert!(
        !engine
            .state
            .object(baloth)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine.state.object(baloth).unwrap().characteristics().power,
        Some(3)
    );
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 25);

    // CR 701.40g: Effect::TurnFaceUp on manifested Instant (`spark`) reveals it and leaves it face down!
    let expose = the(&engine, ZoneLocation::Battlefield, EXPOSE_PROBE);
    let seq_before = engine.state.journal.last_seq();
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: expose,
                ability_index: 0,
            },
        )
        .expect("activate expose on manifested instant");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![spark],
                players: vec![],
            },
        )
        .expect("target manifested instant");
    drain_stack(&mut engine);
    assert!(
        engine
            .state
            .object(spark)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "manifested instant must stay face down (CR 701.40g)"
    );
    assert_eq!(
        engine.state.players[0].life, 25,
        "TurnedFaceUp trigger must NOT fire for manifested instant (CR 701.40g)"
    );
    let revealed_spark = engine.state.journal.entries().iter().any(|e| {
        e.seq > seq_before
            && matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&spark))
    });
    assert!(revealed_spark, "CR 701.40g reveals the manifested instant");
    let turned_spark = engine.state.journal.entries().iter().any(|e| {
        e.seq > seq_before
            && matches!(e.event, GameEvent::TurnedFaceUp { object } if object == spark)
    });
    assert!(
        !turned_spark,
        "CR 701.40g must NOT record GameEvent::TurnedFaceUp for a manifested instant"
    );

    // Effect::TurnFaceUp on manifested Enchantment (`seal`) DOES turn it face up!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: expose,
                ability_index: 0,
            },
        )
        .expect("activate expose on manifested enchantment");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![seal],
                players: vec![],
            },
        )
        .expect("target manifested enchantment");
    drain_stack(&mut engine);
    assert!(
        !engine
            .state
            .object(seal)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state
            .object(seal)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert_eq!(engine.state.players[0].life, 30);

    // CR 701.40c: Manifest MORPH_SPHINX (mana cost {4}{U}{U}, morph {2}{U}) and turn face up using {2}{U}!
    empty_library(&mut engine, me());
    put_on_top(&mut engine, me(), sphinx);
    refresh_offer(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("manifest morph sphinx");
    drain_stack(&mut engine);
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: sphinx,
                ability_index: TURN_FACE_UP,
            },
        )
        .expect("turn manifested sphinx face up via Morph cost (CR 701.40c)");
    assert!(
        !engine
            .state
            .object(sphinx)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine.state.object(sphinx).unwrap().characteristics().power,
        Some(4)
    );
}

/// `Manifest Dread` (`CR 701.62a–b`):
/// - Over 2+ cards: prompts `ChoicePrompt::ManifestDread` over top 2, manifests
///   chosen card face down, puts unchosen card into graveyard.
/// - Over 1 card: manifests the lone card, puts 0 into graveyard.
/// - Over 0 cards: resolves cleanly as a no-op.
#[test]
fn wave5_manifest_dread_picks_one_graveyards_other_and_handles_short_libraries() {
    let mut engine = bench(
        85_005,
        wave5_cards(),
        [
            Seat::with(&[DREAD_PROBE]).holding(&[GREEN_BALOTH, RED_SPARK, VANILLA_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let baloth = the(&engine, ZoneLocation::Hand(me()), GREEN_BALOTH);
    let spark = the(&engine, ZoneLocation::Hand(me()), RED_SPARK);
    let bear = the(&engine, ZoneLocation::Hand(me()), VANILLA_BEAR);
    empty_library(&mut engine, me());
    put_on_top(&mut engine, me(), bear);
    put_on_top(&mut engine, me(), spark);
    put_on_top(&mut engine, me(), baloth);
    refresh_offer(&mut engine);

    let dread = the(&engine, ZoneLocation::Battlefield, DREAD_PROBE);

    // 1) 3 cards in library (top 2: baloth, spark) -> choose baloth, spark goes to graveyard!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: dread,
                ability_index: 0,
            },
        )
        .expect("activate ManifestDread (2+ cards)");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::ManifestDread,
            ..
        } if *player == me() => {
            assert_eq!(options.len(), 2);
            assert!(options.contains(&baloth) && options.contains(&spark));
        }
        other => panic!("expected ChoicePrompt::ManifestDread over top 2, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![baloth],
            },
        )
        .expect("choose baloth to manifest");
    assert_eq!(engine.state.object(baloth).unwrap().zone, Zone::Battlefield);
    assert!(
        engine
            .state
            .object(baloth)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine.state.object(spark).unwrap().zone,
        Zone::Graveyard,
        "unchosen card put into graveyard (CR 701.62a)"
    );

    // 2) 1 card left in library (`bear`) -> manifests `bear`, 0 new cards in graveyard!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: dread,
                ability_index: 0,
            },
        )
        .expect("activate ManifestDread (1 card)");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
    if let Pending::ChooseCards {
        prompt: ChoicePrompt::ManifestDread,
        options,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(options, vec![bear]);
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![bear],
                },
            )
            .expect("choose lone card");
    }
    assert_eq!(engine.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert!(
        engine
            .state
            .object(bear)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Graveyard(me())).len(),
        1,
        "only spark from step 1 is in graveyard"
    );

    // 3) 0 cards in library -> clean no-op!
    assert!(
        engine
            .state
            .zones
            .list(ZoneLocation::Library(me()))
            .is_empty()
    );
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: dread,
                ability_index: 0,
            },
        )
        .expect("activate ManifestDread (0 cards)");
    drain_stack(&mut engine);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}
