//! Gloom's two taxes, through offered actions and paid casts/activations.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};

fn gloom() -> CardIndex {
    card_index("4d022f53-b1fb-4071-afcc-0af3214fe604")
}

fn lions() -> CardIndex {
    baylee_core::generated::index::SAVANNAH_LIONS
}

fn add_mana(e: &mut Engine<RegistryLookup>, p: PlayerId, color: ManaColor, n: u16) {
    e.dev_state_mut(p).unwrap().players[p.get() as usize]
        .mana_pool
        .add(color, u32::from(n));
    e.refresh_offer();
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, card: CardIndex) {
    let object = in_hand(e, p, card).unwrap();
    assert!(priority_offer(e).castable.contains(&object));
    e.apply(p, PlayerAction::CastSpell { card: object })
        .unwrap();
}

#[test]
fn gloom_taxes_both_players_and_multiple_copies_add() {
    for copies in 1..=2 {
        for seat in 0..=1 {
            let p = PlayerId::new(seat);
            let mut e = Duel::new(1050, forest())
                .battlefield(0, &vec![gloom(); copies])
                .hand(seat as usize, &[lions()])
                .start();
            keep_mulligans(&mut e);
            reach_their_main_phase(&mut e, p);
            let card = in_hand(&e, p, lions()).unwrap();
            assert_eq!(
                priority_offer(&e).spell_increase(card, CastModeKind::Normal),
                (copies * 3) as u32
            );
            assert!(
                !e.compute_legal(PlayerId::new(1 - seat))
                    .spell_increases
                    .iter()
                    .any(|(id, _, _)| *id == card)
            );
            add_mana(&mut e, p, ManaColor::White, 1);
            add_mana(&mut e, p, ManaColor::Colorless, (copies * 3 - 1) as u16);
            assert!(!priority_offer(&e).castable.contains(&card));
            add_mana(&mut e, p, ManaColor::Colorless, 1);
            cast(&mut e, p, lions());
            assert_eq!(e.state().players[seat as usize].mana_pool.total(), 0);
            pass_until(&mut e, stack_is_empty);
            assert!(on_battlefield(&e, p, lions()).is_some());
        }
    }
}

#[test]
fn gloom_uses_spell_color_not_color_identity_or_alternative_cost_color() {
    let p = PlayerId::new(0);
    // Damn is black even in its {2}{W}{W} overload mode.
    let damn = baylee_core::generated::index::DAMN;
    let twins = baylee_core::generated::index::TWINING_TWINS;
    let mut e = Duel::new(1051, forest())
        .battlefield(0, &[gloom()])
        .battlefield(1, &[lions()])
        .hand(0, &[damn, twins])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    add_mana(&mut e, p, ManaColor::White, 2);
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    cast(&mut e, p, damn);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, PlayerId::new(1), lions()).is_none());
    add_mana(&mut e, p, ManaColor::Blue, 2);
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    cast(&mut e, p, twins);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p, twins).is_some());
}

#[test]
fn gloom_prices_a_white_adventure_from_its_own_face() {
    let p = PlayerId::new(0);
    let twins = baylee_core::generated::index::TWINING_TWINS;
    let mut e = Duel::new(1052, forest())
        .battlefield(0, &[gloom(), lions()])
        .hand(0, &[twins])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let card = in_hand(&e, p, twins).unwrap();
    add_mana(&mut e, p, ManaColor::White, 1);
    add_mana(&mut e, p, ManaColor::Colorless, 3);
    assert!(!priority_offer(&e).castable.contains(&card));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    cast(&mut e, p, twins);
    let target = on_battlefield(&e, p, lions()).unwrap();
    e.apply(
        p,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    let spell = e.state().object(card).unwrap();
    assert_eq!(spell.face_index, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(target).unwrap().zone, Zone::Exile);
}

#[test]
fn gloom_waived_mana_cost_still_pays_the_tax() {
    let p = PlayerId::new(0);
    let mut e = Duel::new(1053, forest())
        .battlefield(0, &[gloom()])
        .hand(0, &[lions()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let card = in_hand(&e, p, lions()).unwrap();
    e.state
        .move_object(
            card,
            ZoneLocation::Exile(p),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    assert!(!e.free_cast_possible(p, card));
    assert!(e.start_free_cast(p, card).is_err());
    assert_eq!(e.state().players[0].mana_pool.total(), 2);
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    assert!(e.free_cast_possible(p, card));
    e.start_free_cast(p, card).unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Stack);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p, lions()).is_some());
}

#[test]
fn gloom_taxes_white_enchantment_activations_and_stops_when_it_leaves() {
    let p = PlayerId::new(0);
    let circle = circle_of_protection_red();
    let mut e = Duel::new(1054, forest())
        .battlefield(0, &[gloom(), circle])
        .battlefield(1, &[orcish_artillery()])
        .hand(0, &[baylee_core::generated::index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let source = on_battlefield(&e, p, circle).unwrap();
    add_mana(&mut e, p, ManaColor::Colorless, 3);
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    activate(&mut e, p, circle, 0);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = e.pending().clone()
    else {
        unreachable!("source prompt")
    };
    let red = on_battlefield(&e, PlayerId::new(1), orcish_artillery()).unwrap();
    let selected = baylee_core::ids::DamageSourceRef {
        object: red,
        version: e.state().object(red).expect("source exists").version,
    };
    assert!(options.contains(&selected));
    e.apply(
        p,
        PlayerAction::ChooseDamageSource {
            choice,
            source: selected,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| at_rest(e, p));
    let taxer = on_battlefield(&e, p, gloom()).unwrap();
    add_mana(&mut e, p, ManaColor::White, 1);
    add_mana(&mut e, p, ManaColor::Colorless, 4);
    cast(&mut e, p, baylee_core::generated::index::DISENCHANT);
    e.apply(
        p,
        PlayerAction::ChooseObjects {
            objects: vec![taxer],
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| at_rest(e, p));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    assert!(priority_offer(&e).abilities.contains(&(source, 0)));
    activate(&mut e, p, circle, 0);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

/// Construct the rare type/color combination without depending on another
/// unimplemented card: a white enchantment land still has its land abilities.
fn white_enchantment(e: &mut Engine<RegistryLookup>, p: PlayerId, source: ObjectId) {
    let obj = e.dev_state_mut(p).unwrap().object_mut(source).unwrap();
    obj.base_mut().colors = ColorSet::of(Color::White);
    obj.base_mut().types = obj.base.types.union(TypeSet::ENCHANTMENT);
    e.state.refresh_characteristics();
    e.refresh_offer();
}

#[test]
fn gloom_intrinsic_mana_ability_pays_before_producing_mana() {
    let p = PlayerId::new(0);
    let mut e = Duel::new(1055, forest())
        .battlefield(0, &[gloom(), plains()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let land = on_battlefield(&e, p, plains()).unwrap();
    white_enchantment(&mut e, p, land);
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    assert!(!priority_offer(&e).mana_abilities.contains(&land));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    assert!(priority_offer(&e).mana_abilities.contains(&land));
    assert_eq!(priority_offer(&e).activation_increase(land), 3);
    e.apply(p, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::White),
        1
    );
    assert!(
        e.state()
            .object(land)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn gloom_granted_mana_ability_is_taxed_at_offer_and_payment() {
    let p = PlayerId::new(0);
    let mut e = Duel::new(1056, forest())
        .battlefield(
            0,
            &[
                gloom(),
                plains(),
                baylee_core::generated::index::CHROMATIC_LANTERN,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let land = on_battlefield(&e, p, plains()).unwrap();
    white_enchantment(&mut e, p, land);
    let ability_index = crate::choice::GRANTED_ABILITY;
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    assert!(
        !priority_offer(&e)
            .abilities
            .contains(&(land, ability_index))
    );
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    assert!(
        priority_offer(&e)
            .abilities
            .contains(&(land, ability_index))
    );
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index,
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    e.apply(p, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 1);
}

#[test]
fn gloom_lattice_color_change_removes_the_spell_tax() {
    let p = PlayerId::new(0);
    let mut e = Duel::new(1057, forest())
        .battlefield(
            0,
            &[gloom(), baylee_core::generated::index::MYCOSYNTH_LATTICE],
        )
        .hand(0, &[lions()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    cast(&mut e, p, lions());
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p, lions()).is_some());
}

#[test]
fn gloom_conditional_alternative_cost_pays_three_generic() {
    let p = PlayerId::new(0);
    let spell = baylee_core::generated::index::FLAWLESS_MANEUVER;
    let mut e = Duel::new(1058, forest())
        .battlefield(0, &[gloom()])
        .commander(0, &[lions()])
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let commander = e.state.commanders[0][0].object;
    e.state
        .move_object(
            commander,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    let card = in_hand(&e, p, spell).unwrap();
    assert!(!priority_offer(&e).castable.contains(&card));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    cast(&mut e, p, spell);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(keywords(&e, commander).contains(KeywordSet::INDESTRUCTIBLE));
}

#[test]
fn gloom_play_permission_and_paid_effect_cast_each_charge_once() {
    for free in [false, true] {
        let p = PlayerId::new(0);
        let mut e = Duel::new(1059, forest())
            .battlefield(0, &[gloom()])
            .hand(0, &[lions()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let card = in_hand(&e, p, lions()).unwrap();
        e.state
            .move_object(
                card,
                ZoneLocation::Exile(p),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let version = e.state.object(card).unwrap().version;
        if free {
            e.state
                .per_turn
                .playable
                .push(crate::state::PlayPermission {
                    player: p,
                    card,
                    version,
                    free: true,
                    cast_only: true,
                });
            add_mana(&mut e, p, ManaColor::Colorless, 2);
            assert!(!priority_offer(&e).castable.contains(&card));
            add_mana(&mut e, p, ManaColor::Colorless, 1);
            assert!(priority_offer(&e).castable.contains(&card));
            e.apply(p, PlayerAction::CastSpell { card }).unwrap();
        } else {
            add_mana(&mut e, p, ManaColor::White, 1);
            add_mana(&mut e, p, ManaColor::Colorless, 3);
            e.start_paid_cast(p, card, version, false).unwrap();
        }
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Stack);
        pass_until(&mut e, stack_is_empty);
        assert!(on_battlefield(&e, p, lions()).is_some());
    }
}

#[test]
fn gloom_loyalty_ability_of_a_white_enchantment_pays_mana_and_loyalty() {
    let p = PlayerId::new(0);
    let elspeth = baylee_core::generated::index::ELSPETH_STORM_SLAYER;
    let mut e = Duel::new(1060, forest())
        .battlefield(0, &[gloom(), elspeth])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let source = on_battlefield(&e, p, elspeth).unwrap();
    white_enchantment(&mut e, p, source);
    add_mana(&mut e, p, ManaColor::Colorless, 2);
    assert!(!priority_offer(&e).abilities.contains(&(source, 1)));
    assert_eq!(
        e.state()
            .object(source)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        5
    );
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    activate(&mut e, p, elspeth, 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        e.state()
            .object(source)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        6
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(tokens_of(&e, p).len(), 2);
}

#[test]
fn gloom_multicolor_and_flashback_costs_are_taxed() {
    let p = PlayerId::new(0);
    let finks = baylee_core::generated::index::KITCHEN_FINKS;
    let reclamation = baylee_core::generated::index::SEVINNE_S_RECLAMATION;
    let mut e = Duel::new(1061, forest())
        .battlefield(0, &[gloom()])
        .hand(0, &[finks, reclamation, lions()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    add_mana(&mut e, p, ManaColor::Green, 2);
    add_mana(&mut e, p, ManaColor::Colorless, 3);
    let card = in_hand(&e, p, finks).unwrap();
    assert!(!priority_offer(&e).castable.contains(&card));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    cast(&mut e, p, finks);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| at_rest(e, p));
    let reclaim = in_hand(&e, p, reclamation).unwrap();
    let target = in_hand(&e, p, lions()).unwrap();
    for card in [reclaim, target] {
        e.state
            .move_object(
                card,
                ZoneLocation::Graveyard(p),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    add_mana(&mut e, p, ManaColor::White, 1);
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: p,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp: 999,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::object(&e.state, reclaim),
        modifier: baylee_cards_dsl::Modifier::GrantsFlashback,
    });
    add_mana(&mut e, p, ManaColor::Colorless, 4);
    assert!(!priority_offer(&e).castable.contains(&reclaim));
    add_mana(&mut e, p, ManaColor::Colorless, 1);
    assert!(priority_offer(&e).castable.contains(&reclaim));
    e.apply(p, PlayerAction::CastSpell { card: reclaim })
        .unwrap();
    e.apply(
        p,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn gloom_cost_increases_are_applied_before_activation_reductions() {
    let p = PlayerId::new(0);
    let circle = circle_of_protection_red();
    let mut e = Duel::new(1062, forest())
        .battlefield(0, &[gloom(), circle])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let source = on_battlefield(&e, p, circle).unwrap();
    let cost = baylee_cards_dsl::Cost {
        mana: baylee_core::mana!("{1}"),
        parts: &[],
    };
    let reduction = baylee_cards_dsl::CostReduction::PerCount {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        each: 2,
    };
    let price = e.activation_price(p, source, &cost, Some(reduction));
    assert_eq!(
        price.mana,
        baylee_core::mana!("{2}"),
        "1 + 3 - 2, not max(1 - 2, 0) + 3"
    );
}

#[test]
fn gloom_casting_forms_use_the_announced_color() {
    use crate::casting::SpellForm;
    let p = PlayerId::new(0);
    let mut e = Duel::new(1063, forest())
        .battlefield(0, &[gloom()])
        .hand(0, &[lions(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let white = e.state.object(in_hand(&e, p, lions()).unwrap()).unwrap();
    assert_eq!(casting::spell_increase(&e.state, p, white), 3);
    assert_eq!(
        casting::spell_increase(&e.state, p, &SpellForm::Disguise.project(white)),
        0
    );
    let artifact = e.state.object(in_hand(&e, p, sol_ring()).unwrap()).unwrap();
    let prototype = SpellForm::Prototype(baylee_cards_dsl::Prototype {
        cost: baylee_core::mana!("{1}{W}"),
        power: 2,
        toughness: 2,
    });
    assert_eq!(casting::spell_increase(&e.state, p, artifact), 0);
    assert_eq!(
        casting::spell_increase(&e.state, p, &prototype.project(artifact)),
        3
    );
    assert_eq!(white.characteristics().colors, ColorSet::of(Color::White));
    assert!(artifact.characteristics().colors.is_empty());
}

#[test]
fn gloom_does_not_tax_an_enchantment_card_in_hand_or_a_nonwhite_permanent() {
    let p = PlayerId::new(0);
    let circle = circle_of_protection_red();
    let mut e = Duel::new(1064, forest())
        .battlefield(0, &[gloom(), circle])
        .hand(0, &[circle])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let hand = in_hand(&e, p, circle).unwrap();
    let board = on_battlefield(&e, p, circle).unwrap();
    assert_eq!(casting::activation_increase(&e.state, hand), 0);
    assert_eq!(casting::activation_increase(&e.state, board), 3);
    e.state.object_mut(board).unwrap().base_mut().colors = ColorSet::of(Color::Blue);
    e.state.refresh_characteristics();
    assert_eq!(casting::activation_increase(&e.state, board), 0);
}
