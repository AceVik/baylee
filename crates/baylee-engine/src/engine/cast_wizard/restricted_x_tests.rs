use super::*;
use crate::engine::PlayerAction;
use crate::engine::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{Amount, CardDef, CostReduction, Effect, FaceDef};
use baylee_core::color::Color;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::mana::ManaColor;
use baylee_core::preset::DeckEntry;
use baylee_core::types::TypeSet;

fn probe_spell(reduction: u32, restricted: bool, damage: bool) -> &'static CardDef {
    let template = synthetic::land(98_001, "Restricted X probe", &[]);
    Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            name: "Restricted X probe",
            mana_cost: ManaCost::parse("{X}{1}{B}"),
            types: TypeSet::INSTANT,
            x_mana_color: restricted.then_some(Color::Black),
            cost_reduction: Some(CostReduction::PerCount {
                amount: Amount::Fixed(reduction),
                each: 1,
            }),
            ..FaceDef::DEFAULT
        }])),
        abilities: Box::leak(Box::new([AbilityDef::Spell {
            effects: if damage {
                &[Effect::DealDamageWithCappedLifeGain { amount: Amount::X }]
            } else {
                &[Effect::GainLife { amount: Amount::X }]
            },
            targets: damage.then_some(TargetReq::one(TargetSpec::AnyTarget)),
            second_targets: None,
            condition: None,
        }])),
        index: template.index,
        oracle_id: template.oracle_id,
        scryfall_id: template.scryfall_id,
        color_identity: template.color_identity,
        keywords: template.keywords,
        commander: template.commander,
        partner: template.partner,
        coverage: template.coverage,
    }))
}

#[test]
fn restricted_x_reductions_reduce_the_total_and_the_color_requirement_once() {
    for (reduction, black, expected_x) in [(0, 3, 2), (2, 3, 4), (5, 3, 7), (0, 62, 61)] {
        let definition = probe_spell(reduction, true, false);
        let mut preset = synthetic::preset(51, &[]);
        preset.seats[0].starting_hand = Some(vec![DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: CardIndex::new(98_001),
            print: PrintRef::new(0),
        }]);
        let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
        synthetic::keep_mulligans(&mut engine);
        let player = PlayerId::new(0);
        for _ in 0..30 {
            if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        engine.state.players[0]
            .mana_pool
            .add(ManaColor::Black, black);
        engine.state.players[0].mana_pool.add(ManaColor::Red, 1);
        engine.refresh_offer();
        let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
        engine
            .apply(player, PlayerAction::CastSpell { card })
            .unwrap();
        assert!(
            matches!(engine.pending(), Pending::ChooseNumber { min: 0, max, .. } if *max == expected_x)
        );
        engine
            .apply(player, PlayerAction::ChooseNumber(expected_x))
            .unwrap();
        assert_eq!(engine.state.zones.list(ZoneLocation::Stack), &[card]);
        assert_eq!(
            engine.state.players[0].mana_pool.total(),
            0,
            "the chosen X spends the complete available pool"
        );
        for _ in 0..10 {
            if engine.state.zones.list(ZoneLocation::Stack).is_empty() {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        assert_eq!(
            engine.state.players[0].life,
            20 + i32::try_from(expected_x).unwrap()
        );
    }
}

#[test]
fn an_exact_x_target_spell_selects_and_pays_for_more_than_255_targets() {
    let template = probe_spell(0, false, false);
    let definition = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            mana_cost: ManaCost::parse("{X}"),
            ..template.faces[0]
        }])),
        abilities: Box::leak(Box::new([AbilityDef::Spell {
            effects: &[Effect::GainLife { amount: Amount::X }],
            targets: Some(TargetReq::x_targets(TargetSpec::Object(
                &baylee_cards_dsl::Filter::LAND,
            ))),
            second_targets: None,
            condition: None,
        }])),
        ..*template
    }));
    let land = synthetic::land(98_011, "Target-count probe", &[]);
    let mut preset = synthetic::preset(74, &[98_011; 256]);
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: definition.index,
        print: PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition, land])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, 256);
    engine.refresh_offer();
    let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    engine
        .apply(player, PlayerAction::ChooseNumber(256))
        .unwrap();
    let Pending::ChooseTargets {
        min: 256,
        max: 256,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("exact X retains its full count: {:?}", engine.pending());
    };
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: options[..255].to_vec()
                }
            )
            .is_err()
    );
    assert_eq!(
        engine.fingerprint(),
        before,
        "one fewer target is an atomic refusal"
    );
    engine
        .apply(player, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    for _ in 0..10 {
        if engine.state.zones.stack_is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].life, 276);
}

#[test]
fn target_cost_increases_join_x_before_unused_generic_reductions() {
    let template = probe_spell(3, false, false);
    let definition = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            extra_target_cost: 1,
            ..template.faces[0]
        }])),
        abilities: Box::leak(Box::new([AbilityDef::Spell {
            effects: &[Effect::GainLife { amount: Amount::X }],
            targets: Some(TargetReq::up_to(TargetSpec::AnyTarget, 2)),
            second_targets: None,
            condition: None,
        }])),
        ..*template
    }));
    let mut preset = synthetic::preset(73, &[]);
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: definition.index,
        print: PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.state.players[0].mana_pool.add(ManaColor::Black, 5);
    engine.refresh_offer();
    let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    engine.apply(player, PlayerAction::ChooseNumber(5)).unwrap();
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![player, PlayerId::new(1)],
            },
        )
        .unwrap();
    assert_eq!(engine.state.zones.list(ZoneLocation::Stack), &[card]);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        0,
        "X5 plus fixed1 plus second-target1 minus reduction3, and B"
    );
    for _ in 0..10 {
        if engine.state.zones.stack_is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].life, 25);
}

#[test]
fn a_mana_x_activation_can_pay_and_resolve_above_fifty() {
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!("{X}", TapSelf),
        &[Effect::GainLife { amount: Amount::X }]
    )];
    let definition = synthetic::land(98_002, "Variable-cost probe", ABILITIES);
    let preset = synthetic::preset(52, &[98_002]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, 80);
    engine.refresh_offer();
    let source = synthetic::permanents(&engine, 98_002)[0];
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 80,
            ..
        }
    ));
    engine
        .apply(player, PlayerAction::ChooseNumber(80))
        .unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    assert!(synthetic::tapped(&engine, source));
    for _ in 0..10 {
        if engine.state.zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].life, 100);
}

#[test]
fn affordability_search_is_logarithmic_even_at_the_largest_bound() {
    let mut probes = 0;
    let result = casting::greatest_affordable(u32::MAX, |x| {
        probes += 1;
        x <= 100_000
    });
    assert_eq!(result, 100_000);
    assert!(probes <= 32);
    assert_eq!(casting::greatest_affordable(u32::MAX, |_| false), 0);
}

#[test]
fn deferred_x_payment_can_generate_and_spend_more_than_fifty_after_announcement() {
    static MANA: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(
        baylee_cards_dsl::cost!("", SacrificeSelf),
        &[Effect::mana(ManaColor::Black, 63)]
    )];
    let land = synthetic::land(98_003, "Deferred payment source", MANA);
    let template = probe_spell(0, true, false);
    let definition = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            miracle: Some(ManaCost::parse("{X}{1}{B}")),
            ..template.faces[0]
        }])),
        ..*template
    }));
    let mut preset = synthetic::preset(54, &[98_003]);
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: definition.index,
        print: PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition, land])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
    engine.start_miracle_cast(player, card).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseNumber { max, .. } if *max >= 61));
    engine
        .apply(player, PlayerAction::ChooseNumber(61))
        .unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            player,
            baylee_core::mana::ManaPayment::Fixed(
                ManaCost::parse("{1}").with_symbol(baylee_core::mana::ManaSymbol::Black, 62),
            ),
        )),
        "payment_window materializes restricted X into colored symbols so view.owed and manaplan do not plan non-black mana for X"
    );
    let source = synthetic::permanents(&engine, 98_003)[0];
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 63);
    assert_eq!(
        engine.state.object(source).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    for _ in 0..10 {
        if engine.state.zones.stack_is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].life, 81);
}

#[test]
fn paid_damage_preserves_large_x_through_damage_history_and_capped_life_gain() {
    for (restricted, x) in [
        (true, 32_767),
        (true, 32_768),
        (true, 65_534),
        (false, 393_208),
    ] {
        let definition = probe_spell(0, restricted, true);
        let mut preset = synthetic::preset(53, &[]);
        preset.seats[0].starting_hand = Some(vec![DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: definition.index,
            print: PrintRef::new(0),
        }]);
        preset.seats[1].starting_life = Some(1_000_000);
        let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
        synthetic::keep_mulligans(&mut engine);
        let player = PlayerId::new(0);
        let victim = PlayerId::new(1);
        for _ in 0..30 {
            if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        let pool = &mut engine.state.players[0].mana_pool;
        if restricted {
            pool.add(ManaColor::Black, x + 1);
            pool.add(ManaColor::Red, 1);
        } else {
            for color in ManaColor::ALL {
                pool.add(color, u32::from(u16::MAX));
            }
        }
        engine.refresh_offer();
        let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
        engine
            .apply(player, PlayerAction::CastSpell { card })
            .unwrap();
        assert!(matches!(engine.pending(), Pending::ChooseNumber { max, .. } if *max >= x));
        engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![victim],
                },
            )
            .unwrap();
        assert_eq!(
            engine.state.players[0].mana_pool.total(),
            0,
            "all announced mana was paid"
        );
        for _ in 0..10 {
            if engine.state.zones.list(ZoneLocation::Stack).is_empty() {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        assert_eq!(
            engine.state.players[1].life,
            1_000_000 - i32::try_from(x).unwrap()
        );
        assert_eq!(engine.state.players[0].life, 20 + i32::try_from(x).unwrap());
        let damage: Vec<_> = engine
            .state
            .journal
            .entries()
            .iter()
            .filter_map(|entry| match entry.event {
                GameEvent::DamageDealt {
                    source: Some(source),
                    target,
                    amount,
                    ..
                } if source == card => Some((target, amount)),
                _ => None,
            })
            .collect();
        assert_eq!(
            damage,
            [(crate::event::DamageTarget::Player(victim), x)],
            "one full damage event, never truncated or split"
        );
    }
}

#[test]
fn replicate_uses_unspent_generic_reduction_in_the_offer_and_the_payment() {
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1)
    }])];
    let template = synthetic::land(98_003, "Replicate discount probe", &[]);
    let definition = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            name: "Replicate discount probe",
            mana_cost: ManaCost::parse("{U}"),
            types: TypeSet::INSTANT,
            replicate: Some(ManaCost::parse("{1}")),
            cost_reduction: Some(CostReduction::PerCount {
                amount: Amount::Fixed(3),
                each: 1,
            }),
            ..FaceDef::DEFAULT
        }])),
        abilities: ABILITIES,
        ..*template
    }));
    let player = PlayerId::new(0);
    let mut preset = synthetic::preset(51, &[]);
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: definition.index,
        print: PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..30 {
        if matches!(engine.pending(),Pending::Priority{player:p,..} if *p==player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    engine.refresh_offer();
    let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 3,
            reason: NumberPrompt::Replicate { .. },
            ..
        }
    ));
    engine.apply(player, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    for _ in 0..40 {
        if engine.state.zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state.players[0].life, 24);
}

#[test]
fn a_multicolor_x_records_actual_units_separately_from_fixed_pips() {
    static RECEIPT_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::GainLife {
        amount: Amount::ManaTypeSpentOnX(ManaColor::Black),
    }])];
    use baylee_core::color::ColorSet;
    for (black, red, blue, expected_x, black_x) in [
        (1, 4, 0, 3, 0),
        (4, 1, 0, 3, 3),
        (2, 3, 0, 3, 1),
        (1, 2, 2, 2, 0),
    ] {
        let template = probe_spell(0, false, false);
        let definition = Box::leak(Box::new(CardDef {
            faces: Box::leak(Box::new([FaceDef {
                x_mana_colors: Some(ColorSet::from_slice(&[Color::Black, Color::Red])),
                ..template.faces[0]
            }])),
            abilities: RECEIPT_ABILITIES,
            ..*template
        }));
        let player = PlayerId::new(0);
        let mut preset = synthetic::preset(61, &[]);
        preset.seats[0].starting_hand = Some(vec![DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: definition.index,
            print: PrintRef::new(0),
        }]);
        let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
        synthetic::keep_mulligans(&mut engine);
        for _ in 0..30 {
            if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        for (color, amount) in [
            (ManaColor::Black, black),
            (ManaColor::Red, red),
            (ManaColor::Blue, blue),
        ] {
            engine.state.players[0].mana_pool.add(color, amount);
        }
        engine.refresh_offer();
        let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
        engine
            .apply(player, PlayerAction::CastSpell { card })
            .unwrap();
        assert!(
            matches!(engine.pending(), Pending::ChooseNumber { max, .. } if *max == expected_x)
        );
        engine
            .apply(player, PlayerAction::ChooseNumber(expected_x))
            .unwrap();
        let paid = engine.state.object(card).unwrap().paid.as_ref().unwrap();
        assert_eq!(
            paid.mana_types_spent_on_x[ManaColor::Black.index()],
            black_x
        );
        assert_eq!(
            paid.mana_types_spent_on_x[ManaColor::Red.index()],
            expected_x - black_x
        );
        assert_eq!(paid.mana_types_spent_on_x[ManaColor::Blue.index()], 0);
        assert_eq!(paid.mana_types_spent[ManaColor::Black.index()], black_x + 1);
        for _ in 0..10 {
            if engine.state.zones.stack_is_empty() {
                break;
            }
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending));
        }
        assert_eq!(
            engine.state.players[0].life,
            20 + i32::try_from(black_x).unwrap()
        );
    }
}

#[test]
fn accepting_a_composite_price_revalidates_life_before_spending_any_mana() {
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(""),
        &[Effect::PlayerMayPayManaAndLifeEachThenElse {
            player: baylee_cards_dsl::PlayerRel::You,
            mana: ManaCost::parse("{B}"),
            life: 2,
            times: Amount::Fixed(2),
            paid: &[],
            unpaid: &[Effect::GainLife {
                amount: Amount::Fixed(7)
            }],
        }]
    )];
    let source_def = synthetic::land(98_081, "Composite price probe", ABILITIES);
    let mut engine = Engine::new(
        &synthetic::preset(62, &[98_081]),
        SyntheticLookup::new(vec![source_def]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.state.players[0].life = 3;
    engine.state.players[0].mana_pool.add(ManaColor::Black, 2);
    engine.refresh_offer();
    let source = synthetic::permanents(&engine, 98_081)[0];
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    for _ in 0..10 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayManaAndLife { .. },
                ..
            }
        ) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::PayManaAndLife { life: 4, .. },
            ..
        }
    ));
    engine.apply(player, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state.players[0].life, 10);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        2,
        "unpayable life cannot consume the mana component"
    );
}

#[test]
fn deferred_payment_window_enforces_color_restriction_on_x_and_rolls_back_off_color_mana() {
    static ADD_B: &[Effect] = &[Effect::mana(ManaColor::Black, 1)];
    static ADD_R: &[Effect] = &[Effect::mana(ManaColor::Red, 1)];
    static SWAMP_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(ADD_B)];
    static MOUNTAIN_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(ADD_R)];
    let definition = probe_spell(0, true, false);
    let swamp_def = synthetic::land(98_090, "Swamp probe", SWAMP_ABILITIES);
    let mountain_def = synthetic::land(98_091, "Mountain probe", MOUNTAIN_ABILITIES);
    let mut preset = synthetic::preset(77, &[98_090, 98_090, 98_090, 98_090, 98_091, 98_091]);
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(98_001),
        print: PrintRef::new(0),
    }]);
    let mut engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![definition, swamp_def, mountain_def]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    let player = PlayerId::new(0);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }

    let card = engine.state.zones.list(ZoneLocation::Hand(player))[0];
    let swamps = synthetic::permanents(&engine, 98_090);
    let mountains = synthetic::permanents(&engine, 98_091);

    // 1. Cast with empty pool (deferred payment window) and choose X = 2.
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    engine.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            player,
            baylee_core::mana::ManaPayment::Fixed(ManaCost::parse("{1}{B}{B}{B}"))
        )),
        "X = 2 with x_mana_color = Black must materialize {{B}}{{B}} in the deferred payment window"
    );

    // Tap 2 Swamps and 2 Mountains ({B}{B}{R}{R} = 4 mana, but only 2 black for {1}{B}{B}{B}).
    for &land in &[swamps[0], swamps[1], mountains[0], mountains[1]] {
        engine
            .apply(
                player,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    // Passing priority in the payment window fails to pay {1}{B}{B}{B} and reverses cast + mana (CR 734.1).
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    assert!(engine.state.zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(engine.state.zones.list(ZoneLocation::Hand(player)), &[card]);
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    for &land in &[swamps[0], swamps[1], mountains[0], mountains[1]] {
        assert!(
            !engine
                .state
                .object(land)
                .unwrap()
                .status
                .contains(crate::object::Status::TAPPED)
        );
    }

    // 2. Cast again with X = 1 and pay {B}{B}{R} (satisfying {1}{B}{B}).
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    engine.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            player,
            baylee_core::mana::ManaPayment::Fixed(ManaCost::parse("{1}{B}{B}"))
        ))
    );
    for &land in &[swamps[2], swamps[3], mountains[0]] {
        engine
            .apply(
                player,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state.zones.list(ZoneLocation::Stack), &[card]);
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
}

#[test]
fn unpaid_special_and_offboard_activations_partition_cleanly_between_unpaid_and_paid() {
    use crate::choice::{TURN_FACE_UP, unlock_door};
    use crate::engine::testkit::{
        Duel, cast_with_floating, in_hand, keep_mulligans, on_battlefield, pass_until,
        reach_main_phase, stack_is_empty,
    };
    use baylee_core::generated::index;

    let p0 = PlayerId::new(0);
    let walk_in_closet = baylee_cards::by_oracle_id("52e77cc3-f8e9-4a20-811b-fe1e46a96ad7")
        .expect("Walk-In Closet // Forgotten Cellar")
        .index;
    let mut engine = Duel::new(42, index::FOREST)
        .battlefield(0, &[walk_in_closet])
        .hand(0, &[index::ZOETIC_CAVERN, index::ASH_BARRENS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Zoetic Cavern face down via morph ({3}), leaving the mana pool empty.
    engine.state.players[0].mana_pool.add(ManaColor::Green, 3);
    engine.refresh_offer();
    cast_with_floating(&mut engine, p0, index::ZOETIC_CAVERN);
    pass_until(&mut engine, stack_is_empty);

    let cavern = on_battlefield(&engine, p0, index::ZOETIC_CAVERN).expect("face-down Cavern");
    let room = on_battlefield(&engine, p0, walk_in_closet).expect("Room on battlefield");
    let barrens = in_hand(&engine, p0, index::ASH_BARRENS).expect("Ash Barrens in hand");

    // 1. With an empty mana pool at sorcery speed: all four appear in `unpaid_abilities` only.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    for (src, idx, expected_cost) in [
        (cavern, TURN_FACE_UP, ManaCost::parse("{2}")),
        (room, unlock_door(0), ManaCost::parse("{2}{G}")),
        (room, unlock_door(1), ManaCost::parse("{3}{G}{G}")),
        (barrens, 1, ManaCost::parse("{1}")),
    ] {
        assert!(legal.unpaid_abilities.contains(&(src, idx, expected_cost)));
        assert!(!legal.abilities.contains(&(src, idx)));
    }

    // 2. Add {3}{G}{G} to the mana pool: all four move from `unpaid_abilities` into `abilities`.
    engine.state.players[0].mana_pool.add(ManaColor::Green, 5);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    for (src, idx) in [
        (cavern, TURN_FACE_UP),
        (room, unlock_door(0)),
        (room, unlock_door(1)),
        (barrens, 1),
    ] {
        assert!(legal.abilities.contains(&(src, idx)));
        assert!(
            !legal
                .unpaid_abilities
                .iter()
                .any(|&(s, i, _)| s == src && i == idx)
        );
    }

    // 3. Unlock door 0 ({2}{G}), then clear the remaining pool: door 0 is gone; door 1 is unpaid again.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: room,
                ability_index: unlock_door(0),
            },
        )
        .unwrap();
    engine.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    assert!(!legal.abilities.contains(&(room, unlock_door(0))));
    assert!(
        !legal
            .unpaid_abilities
            .iter()
            .any(|&(s, i, _)| s == room && i == unlock_door(0))
    );
    assert!(
        legal
            .unpaid_abilities
            .contains(&(room, unlock_door(1), ManaCost::parse("{3}{G}{G}")))
    );
}
