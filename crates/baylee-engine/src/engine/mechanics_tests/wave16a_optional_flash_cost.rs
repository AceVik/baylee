//! Optional additional costs that grant flash (CR 118.8b, CR 702.8a).
//! The ordinary route keeps its printed timing; only the paid route changes it.

use super::*;
use crate::choice::CastModeKind;
use baylee_cards_dsl::prelude::spell;
use baylee_cards_dsl::{Amount, Filter, Modifier, StaticAbility, TargetSpec, ZoneRef};
use baylee_core::mana::ManaColor;

const FLASH_BOLT: u32 = 99_920;
const X_FLASH_BOLT: u32 = 99_921;
const TAXER: u32 = 99_922;

static BOLT_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::DealDamage {
        target: TargetSpec::AnyTarget,
        amount: Amount::Fixed(2),
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget))
)];

static X_BOLT_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::DealDamage {
        target: TargetSpec::AnyTarget,
        amount: Amount::X,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget))
)];

fn add_pool(engine: &mut Bench, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn opponent_turn(engine: &mut Bench) {
    to_main(engine, them());
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("opponent passes priority");
    assert_eq!(offer(engine).0, me());
}

fn target_opponent(engine: &mut Bench) {
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("opponent is a legal target");
}

#[test]
fn optional_cost_alone_grants_flash_and_charges_the_full_cost() {
    let bolt = card(
        FLASH_BOLT,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::parse("{2}"),
                parts: &[],
            }),
            optional_cost_grants_flash: true,
            ..face("Sorcery With Optional Flash", "{R}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        BOLT_ABILITIES,
    );
    let mut engine = bench(
        18_160,
        vec![bolt],
        [Seat::default().holding(&[FLASH_BOLT]), Seat::default()],
    );
    opponent_turn(&mut engine);
    let spell = the(&engine, ZoneLocation::Hand(me()), FLASH_BOLT);

    add_pool(&mut engine, ManaColor::Red, 1);
    assert!(
        !offer(&engine).1.castable.contains(&spell),
        "the printed sorcery route is not an instant-speed escape hatch"
    );

    add_pool(&mut engine, ManaColor::Colorless, 2);
    assert!(offer(&engine).1.castable.contains(&spell));
    let ways = &offer(&engine).1.cast_ways;
    assert_eq!(
        ways.len(),
        1,
        "the public offer carries the price: {ways:?}"
    );
    assert_eq!(ways[0].0, spell);
    assert_eq!(ways[0].1.kind, CastModeKind::OptionalCost);
    assert_eq!(ways[0].1.cost, ManaCost::parse("{2}{R}"));
    let options = engine
        .cast_options(me(), spell, false)
        .expect("the paid flash route has a cast option");
    assert_eq!(options.len(), 1, "Normal must not be offered: {options:?}");
    assert_eq!(options[0].kind, CastModeKind::OptionalCost);
    assert_eq!(options[0].cost, ManaCost::parse("{2}{R}"));

    engine
        .apply(me(), PlayerAction::CastSpell { card: spell })
        .expect("paid flash route is castable on the opponent's turn");
    target_opponent(&mut engine);
    settle(&mut engine, me());

    assert_eq!(life(&engine, them()), 18, "the spell effect resolved");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn optional_flash_cost_leaves_the_printed_sorcery_route_unchanged() {
    let bolt = card(
        FLASH_BOLT,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::parse("{2}"),
                parts: &[],
            }),
            optional_cost_grants_flash: true,
            ..face("Sorcery With Optional Flash", "{R}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        BOLT_ABILITIES,
    );
    let mut engine = bench(
        18_161,
        vec![bolt],
        [Seat::default().holding(&[FLASH_BOLT]), Seat::default()],
    );
    to_main(&mut engine, me());
    let spell = the(&engine, ZoneLocation::Hand(me()), FLASH_BOLT);
    add_pool(&mut engine, ManaColor::Red, 1);

    engine
        .apply(me(), PlayerAction::CastSpell { card: spell })
        .expect("ordinary sorcery cast needs only the printed cost");
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "the unaffordable optional route adds no mode question: {:?}",
        engine.pending()
    );
    target_opponent(&mut engine);
    settle(&mut engine, me());

    assert_eq!(life(&engine, them()), 18);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn optional_flash_cost_uses_the_ordinary_x_and_cost_increase_pipeline() {
    static TAX_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::ControlledByYou,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::SpellsCostMoreMana(ManaCost::parse("{1}")),
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })];
    let bolt = card(
        X_FLASH_BOLT,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::parse("{2}"),
                parts: &[],
            }),
            optional_cost_grants_flash: true,
            ..face("X Sorcery With Optional Flash", "{X}{R}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        X_BOLT_ABILITIES,
    );
    let taxer = card(
        TAXER,
        face("Generic Spell Tax", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TAX_ABILITIES,
    );
    let mut engine = bench(
        18_162,
        vec![bolt, taxer],
        [
            Seat::with(&[TAXER]).holding(&[X_FLASH_BOLT]),
            Seat::default(),
        ],
    );
    opponent_turn(&mut engine);
    let spell = the(&engine, ZoneLocation::Hand(me()), X_FLASH_BOLT);
    add_pool(&mut engine, ManaColor::Red, 1);
    add_pool(&mut engine, ManaColor::Colorless, 4);

    let options = engine
        .cast_options(me(), spell, false)
        .expect("the paid flash route has a cast option");
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::OptionalCost);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell })
        .expect("X=1 plus {2} optional cost and {1} tax is payable");
    let Pending::ChooseNumber { min: 0, .. } = engine.pending().clone() else {
        panic!("expected the spell's X choice, got {:?}", engine.pending());
    };
    engine
        .apply(me(), PlayerAction::ChooseNumber(1))
        .expect("choose X=1");
    target_opponent(&mut engine);
    settle(&mut engine, me());

    assert_eq!(life(&engine, them()), 19, "X survived the paid flash route");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "X=1, optional {{2}}, printed {{R}} and tax {{1}} were all paid"
    );
}
