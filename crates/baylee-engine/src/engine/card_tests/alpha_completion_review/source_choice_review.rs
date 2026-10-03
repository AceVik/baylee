//! Independent real-card source decisions, checked against current CR 609.7a–b.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;
use baylee_core::ids::{DamageSourceRef, SourceChoiceId};

const DEFENDER: PlayerId = PlayerId::new(0);
const ATTACKER: PlayerId = PlayerId::new(1);

fn set_color(engine: &mut Engine<RegistryLookup>, source: ObjectId, color: Color) {
    engine.state.object_mut(source).unwrap().base_mut().colors = ColorSet::of(color);
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();
}

fn incarnation(engine: &Engine<RegistryLookup>, object: ObjectId) -> DamageSourceRef {
    DamageSourceRef {
        object,
        version: engine.state().object(object).unwrap().version,
    }
}

fn blink(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    for destination in [ZoneLocation::Hand(ATTACKER), ZoneLocation::Battlefield] {
        engine
            .state
            .move_object(
                object,
                destination,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.state.refresh_characteristics();
}

fn setup(shield: CardIndex) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                shield,
                plains(),
                plains(),
                plains(),
                plains(),
                index::WALL_OF_SWORDS,
            ],
        )
        .hand(0, &[index::REVERSE_DAMAGE])
        .battlefield(1, &[index::ORCISH_ARTILLERY, index::ORCISH_ARTILLERY])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, ATTACKER);
    engine
}

fn shoot(engine: &mut Engine<RegistryLookup>, target: Option<ObjectId>) -> DamageSourceRef {
    let source = on_battlefield(engine, ATTACKER, index::ORCISH_ARTILLERY).unwrap();
    shoot_object(engine, source, target)
}

fn shoot_object(
    engine: &mut Engine<RegistryLookup>,
    source: ObjectId,
    target: Option<ObjectId>,
) -> DamageSourceRef {
    let reference = incarnation(engine, source);
    engine
        .apply(
            ATTACKER,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    if let Some(target) = target {
        aim(engine, vec![target], vec![]);
    } else {
        aim(engine, vec![], vec![DEFENDER]);
    }
    reference
}

fn defender_priority(engine: &mut Engine<RegistryLookup>) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == DEFENDER),
    );
}

fn source_question(
    engine: &mut Engine<RegistryLookup>,
    shield: CardIndex,
    target: Option<ObjectId>,
) {
    defender_priority(engine);
    if shield == index::REVERSE_DAMAGE {
        cast_from_hand(engine, DEFENDER, shield);
    } else {
        tap_all_mana(engine, DEFENDER);
        activate(engine, DEFENDER, shield, 0);
        if let Some(target) = target {
            aim(engine, vec![target], vec![]);
        }
    }
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
}

fn choose(engine: &mut Engine<RegistryLookup>, source: DamageSourceRef) -> PlayerAction {
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
    } = engine.pending().clone()
    else {
        panic!("a source decision is required: {:?}", engine.pending());
    };
    assert_eq!(player, DEFENDER);
    assert!(
        options.contains(&source),
        "the exact source must be offered: {options:?}"
    );
    let action = PlayerAction::ChooseDamageSource { choice, source };
    engine.apply(DEFENDER, action.clone()).unwrap();
    action
}

fn reject(engine: &mut Engine<RegistryLookup>, player: PlayerId, action: PlayerAction) {
    let fingerprint = engine.fingerprint();
    let pending = serde_json::to_value(engine.pending()).unwrap();
    let journal = engine.state().journal.len();
    assert!(engine.apply(player, action).is_err());
    assert_eq!(engine.fingerprint(), fingerprint);
    assert_eq!(serde_json::to_value(engine.pending()).unwrap(), pending);
    assert_eq!(engine.state().journal.len(), journal);
}

#[test]
fn every_circle_distinguishes_the_departed_damage_source_from_its_returned_card() {
    for (circle, color) in [
        (index::CIRCLE_OF_PROTECTION_WHITE, Color::White),
        (index::CIRCLE_OF_PROTECTION_BLUE, Color::Blue),
        (index::CIRCLE_OF_PROTECTION_BLACK, Color::Black),
        (index::CIRCLE_OF_PROTECTION_RED, Color::Red),
        (index::CIRCLE_OF_PROTECTION_GREEN, Color::Green),
    ] {
        for choose_old in [true, false] {
            let mut engine = setup(circle);
            let object = on_battlefield(&engine, ATTACKER, index::ORCISH_ARTILLERY).unwrap();
            set_color(&mut engine, object, color);
            let old = shoot(&mut engine, None);
            blink(&mut engine, old.object);
            set_color(&mut engine, old.object, color);
            let current = incarnation(&engine, old.object);
            assert_ne!(old, current);
            source_question(&mut engine, circle, None);
            let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
                unreachable!()
            };
            assert!(options.contains(&old) && options.contains(&current));
            choose(&mut engine, if choose_old { old } else { current });
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                engine.state().players[0].life,
                if choose_old { 20 } else { 18 },
                "{circle:?}"
            );
            assert_eq!(engine.state().players[1].life, 17);
        }
    }
}

#[test]
fn reverse_damage_gains_life_only_for_the_chosen_incarnation() {
    for choose_old in [true, false] {
        let mut engine = setup(index::CIRCLE_OF_PROTECTION_RED);
        let old = shoot(&mut engine, None);
        blink(&mut engine, old.object);
        let current = incarnation(&engine, old.object);
        source_question(&mut engine, index::REVERSE_DAMAGE, None);
        choose(&mut engine, if choose_old { old } else { current });
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].life,
            if choose_old { 22 } else { 18 }
        );
        assert_eq!(engine.state().players[1].life, 17);
    }
}

#[test]
fn jade_monolith_redirects_only_the_chosen_incarnation() {
    for choose_old in [true, false] {
        let mut engine = setup(index::JADE_MONOLITH);
        let body = on_battlefield(&engine, DEFENDER, index::WALL_OF_SWORDS).unwrap();
        let old = shoot(&mut engine, Some(body));
        blink(&mut engine, old.object);
        let current = incarnation(&engine, old.object);
        source_question(&mut engine, index::JADE_MONOLITH, Some(body));
        choose(&mut engine, if choose_old { old } else { current });
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].life,
            if choose_old { 18 } else { 20 }
        );
        assert_eq!(
            engine.state().object(body).unwrap().damage,
            if choose_old { 0 } else { 2 }
        );
        assert_eq!(engine.state().players[1].life, 17);
    }
}

#[test]
fn circle_rechecks_live_color_but_departed_damage_uses_departure_color() {
    for departed in [true, false] {
        let mut engine = setup(index::CIRCLE_OF_PROTECTION_RED);
        let old = shoot(&mut engine, None);
        source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
        choose(&mut engine, old);
        if departed {
            blink(&mut engine, old.object);
        }
        set_color(&mut engine, old.object, Color::Blue);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].life,
            if departed { 20 } else { 18 }
        );
        assert_eq!(engine.state().players[1].life, 17);
    }
}

#[test]
fn circle_excludes_an_unreferenced_graveyard_and_rejects_forged_or_stale_answers() {
    let mut engine = setup(index::CIRCLE_OF_PROTECTION_RED);
    let sources = all_on_battlefield(&engine, ATTACKER, index::ORCISH_ARTILLERY);
    let irrelevant = incarnation(&engine, sources[1]);
    bury(&mut engine, &[irrelevant.object]);
    let old = shoot(&mut engine, None);
    blink(&mut engine, old.object);
    source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(
        !options
            .iter()
            .any(|source| source.object == irrelevant.object)
    );
    let valid = PlayerAction::ChooseDamageSource {
        choice,
        source: old,
    };
    reject(&mut engine, ATTACKER, valid);
    reject(
        &mut engine,
        DEFENDER,
        PlayerAction::ChooseDamageSource {
            choice,
            source: irrelevant,
        },
    );
    reject(
        &mut engine,
        DEFENDER,
        PlayerAction::ChooseDamageSource {
            choice,
            source: DamageSourceRef {
                object: old.object,
                version: old.version.wrapping_add(1),
            },
        },
    );
    reject(
        &mut engine,
        DEFENDER,
        PlayerAction::ChooseDamageSource {
            choice: SourceChoiceId::new(choice.get().wrapping_add(1)),
            source: old,
        },
    );
    reject(
        &mut engine,
        DEFENDER,
        PlayerAction::ChooseObjects {
            objects: vec![old.object],
        },
    );
    let answer = choose(&mut engine, old);
    reject(&mut engine, DEFENDER, answer);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn forcefield_chooses_the_exact_unblocked_attacker_and_prevents_all_but_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::FORCEFIELD, plains()])
        .battlefield(1, &[index::SERRA_ANGEL, index::ORCISH_ARTILLERY])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, ATTACKER);
    let angel = on_battlefield(&engine, ATTACKER, index::SERRA_ANGEL).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            ATTACKER,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(DEFENDER))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(DEFENDER, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    source_question(&mut engine, index::FORCEFIELD, None);
    let exact = incarnation(&engine, angel);
    let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(
        options,
        &[exact],
        "a nonattacking creature is not unblocked"
    );
    choose(&mut engine, exact);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].life, 19);
}

#[test]
fn circle_three_incarnations_offer_two_waiting_abilities_and_shield_only_one_exact_source() {
    for protect in 0..3 {
        let mut engine = setup(index::CIRCLE_OF_PROTECTION_RED);
        let object = on_battlefield(&engine, ATTACKER, index::ORCISH_ARTILLERY).unwrap();
        // Lifelink makes the first incarnation's outcome distinguishable from
        // the second one's damage, without changing either printed ability.
        let base = engine.state.object_mut(object).unwrap().base_mut();
        base.keywords = base.keywords.union(KeywordSet::LIFELINK);
        engine.state.invalidate_projections();
        engine.state.refresh_characteristics();
        let first = shoot(&mut engine, None);
        blink(&mut engine, object);
        let returned = engine.state.object_mut(object).unwrap();
        let base = returned.base_mut();
        base.keywords = base.keywords.difference(KeywordSet::LIFELINK);
        returned.controlled_since = 0;
        engine.state.invalidate_projections();
        engine.state.refresh_characteristics();
        engine.refresh_offer();
        let second = shoot_object(&mut engine, object, None);
        blink(&mut engine, object);
        let third = incarnation(&engine, object);
        source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
        let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
            unreachable!()
        };
        for exact in [first, second, third] {
            assert!(options.contains(&exact));
        }
        choose(&mut engine, [first, second, third][protect]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].life,
            if protect == 2 { 16 } else { 18 }
        );
        assert_eq!(
            engine.state().players[1].life,
            if protect == 0 { 17 } else { 19 },
            "only the first incarnation gains life from its damage"
        );
    }
}

#[test]
fn circle_waiting_shield_keeps_a_departed_source_selectable_until_the_shield_expires() {
    for expire in [true, false] {
        let mut engine = setup(index::CIRCLE_OF_PROTECTION_RED);
        let old = shoot(&mut engine, None);
        blink(&mut engine, old.object);
        let shielded = incarnation(&engine, old.object);
        source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
        choose(&mut engine, shielded);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[0].life, 18);
        bury(&mut engine, &[old.object]);
        if expire {
            // Keep the source in the graveyard through a real cleanup and
            // wait for our next main phase before requesting another source.
            pass_until(&mut engine, |e| {
                e.state().turn.active == DEFENDER && e.state().turn.phase == Phase::FirstMain
            });
        }
        source_question(&mut engine, index::REVERSE_DAMAGE, None);
        let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
            unreachable!()
        };
        assert_eq!(options.contains(&shielded), !expire);
        assert!(
            !options.contains(&old),
            "the resolved artillery ability is no longer a reference"
        );
        if !expire {
            choose(&mut engine, shielded);
        }
    }
}

#[test]
fn jade_selecting_a_cast_fire_imp_spell_applies_to_its_resolved_permanents_etb_damage() {
    let mut engine = Duel::new(1431, forest())
        .battlefield(0, &[index::JADE_MONOLITH, plains(), index::WALL_OF_SWORDS])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[index::FIRE_IMP])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, ATTACKER);
    let body = on_battlefield(&engine, DEFENDER, index::WALL_OF_SWORDS).unwrap();
    let imp = in_hand(&engine, ATTACKER, index::FIRE_IMP).unwrap();
    cast_from_hand(&mut engine, ATTACKER, index::FIRE_IMP);
    let spell = incarnation(&engine, imp);
    source_question(&mut engine, index::JADE_MONOLITH, Some(body));
    choose(&mut engine, spell);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_ne!(
        incarnation(&engine, imp),
        spell,
        "the resolved permanent has a new version"
    );
    aim(&mut engine, vec![body], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().object(body).unwrap().damage, 0);
}

#[test]
fn reverse_damage_offers_the_real_sacrifice_costs_departed_creature() {
    let mut engine = Duel::new(1432, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[index::REVERSE_DAMAGE])
        .battlefield(1, &[swamp(), index::ORCISH_ARTILLERY])
        .hand(1, &[index::SACRIFICE])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, ATTACKER);
    let victim = on_battlefield(&engine, ATTACKER, index::ORCISH_ARTILLERY).unwrap();
    let sacrificed = incarnation(&engine, victim);
    cast_from_hand(&mut engine, ATTACKER, index::SACRIFICE);
    assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
    engine
        .apply(
            ATTACKER,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    assert_eq!(
        in_graveyard(&engine, ATTACKER, index::ORCISH_ARTILLERY),
        Some(victim)
    );
    let graveyard = incarnation(&engine, victim);
    assert_ne!(sacrificed, graveyard);
    source_question(&mut engine, index::REVERSE_DAMAGE, None);
    let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(options.contains(&sacrificed));
    assert!(
        !options.contains(&graveyard),
        "the spell refers to the sacrificed creature, not the new graveyard card"
    );
    choose(&mut engine, sacrificed);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        3
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a source need not be able to deal damage"
    );
}

#[test]
fn jade_offers_only_the_cards_referred_to_by_a_real_linked_return_ability() {
    let mut engine = Duel::new(1433, forest())
        .battlefield(0, &[index::JADE_MONOLITH, plains(), index::WALL_OF_SWORDS])
        .battlefield(
            1,
            &[
                index::ENDLESS_SANDS,
                index::ENDLESS_SANDS,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::FIRE_IMP,
                index::WALL_OF_AIR,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, ATTACKER);
    let sands = all_on_battlefield(&engine, ATTACKER, index::ENDLESS_SANDS);
    let imp = on_battlefield(&engine, ATTACKER, index::FIRE_IMP).unwrap();
    let other = on_battlefield(&engine, ATTACKER, index::WALL_OF_AIR).unwrap();
    let body = on_battlefield(&engine, DEFENDER, index::WALL_OF_SWORDS).unwrap();
    tap_all_mana_but(&mut engine, ATTACKER, Some(index::ENDLESS_SANDS));
    for (source, target) in [(sands[0], imp), (sands[1], other)] {
        engine
            .apply(
                ATTACKER,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1,
                },
            )
            .unwrap();
        aim(&mut engine, vec![target], vec![]);
        pass_until(&mut engine, stack_is_empty);
    }
    let linked = incarnation(&engine, imp);
    let unrelated = incarnation(&engine, other);
    pass_until(&mut engine, |e| {
        e.state().turn.active == ATTACKER
            && e.state().turn.phase == Phase::FirstMain
            && !is_tapped(e, sands[0])
    });
    tap_all_mana_but(&mut engine, ATTACKER, Some(index::ENDLESS_SANDS));
    engine
        .apply(
            ATTACKER,
            PlayerAction::ActivateAbility {
                source: sands[0],
                ability_index: 2,
            },
        )
        .unwrap();
    source_question(&mut engine, index::JADE_MONOLITH, Some(body));
    let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        options.contains(&linked),
        "the waiting linked-return text refers to this exiled card"
    );
    assert!(
        !options.contains(&unrelated),
        "the other land's exiled card is not referred to"
    );
    choose(&mut engine, linked);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_ne!(incarnation(&engine, imp), linked);
    aim(&mut engine, vec![body], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(body).unwrap().damage,
        2,
        "returning an exiled card is not the permanent-spell exception"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

fn omnath_death_source(choose_before_death: bool) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[index::CIRCLE_OF_PROTECTION_RED, swamp(), swamp(), plains()],
        )
        .hand(0, &[index::TERROR])
        .battlefield(1, &[index::OMNATH_LOCUS_OF_RAGE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, DEFENDER);
    let omnath = on_battlefield(&engine, ATTACKER, index::OMNATH_LOCUS_OF_RAGE).unwrap();
    let departed = incarnation(&engine, omnath);
    if choose_before_death {
        source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
        choose(&mut engine, departed);
        pass_until(&mut engine, stack_is_empty);
    }
    cast_from_hand(&mut engine, DEFENDER, index::TERROR);
    aim(&mut engine, vec![omnath], vec![]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == ATTACKER),
    );
    aim(&mut engine, vec![], vec![DEFENDER]);
    let graveyard = incarnation(&engine, omnath);
    assert_ne!(departed, graveyard);
    assert_eq!(
        in_graveyard(&engine, ATTACKER, index::OMNATH_LOCUS_OF_RAGE),
        Some(omnath)
    );
    if !choose_before_death {
        source_question(&mut engine, index::CIRCLE_OF_PROTECTION_RED, None);
        let Pending::ChooseDamageSource { options, .. } = engine.pending() else {
            unreachable!()
        };
        assert!(
            options.contains(&departed),
            "the death ability refers to its battlefield source: {options:?}"
        );
        assert!(
            !options.contains(&graveyard),
            "the graveyard card is a separate incarnation: {options:?}"
        );
        choose(&mut engine, departed);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the preselected battlefield source and its death trigger must match; chose before death: {choose_before_death}"
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert!(
        engine.state().shields.is_empty(),
        "the Circle prevented and consumed the next three damage"
    );
}

#[test]
fn circle_red_preselected_battlefield_source_prevents_omnaths_real_death_trigger() {
    omnath_death_source(true);
}

#[test]
fn circle_red_after_omnath_dies_offers_its_departed_source_and_prevents_damage() {
    omnath_death_source(false);
}
