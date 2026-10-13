//! `cards/instants/mv_1/dark_ritual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A preceding Ritual gives Brain Freeze one storm copy, so the opponent
/// mills six cards while only the original card enters its owner's graveyard.
#[test]
fn brain_freeze_and_its_storm_copy_each_mill_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[swamp(), island(), island()])
        .hand(0, &[dark_ritual(), brain_freeze()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let graveyard = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state().zones.list(ZoneLocation::Graveyard(seat)).len()
    };

    // The turn's first spell, so that storm would have something to count.
    tap_all_mana_but(&mut engine, p0, Some(island()));
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("one Swamp pays {B}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.spells_cast[p0.get() as usize],
        1,
        "a spell was cast before the next one this turn — the number storm reads"
    );

    let their_library = library_size(&engine, p1);
    let their_graveyard = graveyard(&engine, p1);
    let my_graveyard = graveyard(&engine, p0);

    // Cast off the engine's own offer rather than at it: the two Islands are
    // tapped first, because a spell is castable here only once its mana is
    // already floating.
    let freeze = in_hand(&engine, p0, brain_freeze()).expect("the spell is in hand");
    tap_all_mana_but(&mut engine, p0, None);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.castable.contains(&freeze),
        "the engine offers the spell before the test presses it"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: freeze })
        .expect("two Islands pay {1}{U}");

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is named as the spell is cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their choice");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "either player may be milled, the caster included: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the other seat is one of the options just offered");

    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 2);
    keep_storm_copy_targets(&mut engine, p0);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 6,
        "the original and its storm copy each mill three cards"
    );
    assert_eq!(
        graveyard(&engine, p1),
        their_graveyard + 6,
        "milling puts them into the graveyard, not into exile"
    );
    assert_eq!(
        graveyard(&engine, p0),
        my_graveyard + 1,
        "the caster milled nothing: the one card added is Brain Freeze itself"
    );
    assert!(
        in_graveyard(&engine, p0, brain_freeze()).is_some(),
        "and the instant resolved once and went to its owner's graveyard"
    );
}

/// The storm copy taxes the target's controller. Declining counters the
/// Ritual, and the original Flusterstorm then has no legal target.
#[test]
fn a_declined_flusterstorm_copy_counters_the_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(59, forest())
        .battlefield(0, &[island()])
        .hand(0, &[flusterstorm()])
        // Two Swamps, not one, so the Ritual leaves a mana floating and
        // Flusterstorm's tax is answered straight from the pool. Off a
        // single Swamp the question is still put — CR 605.3a opens a
        // payment window — but the test would then be about the window
        // instead of about Flusterstorm.
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p0 holds; p1 answers with an instant of their own, one Swamp's worth
    // of mana still floating behind it.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // p0 taps the Island and points Flusterstorm at it.
    let fluster = in_hand(&engine, p0, flusterstorm()).expect("Flusterstorm is in hand");
    cast_from_hand(&mut engine, p0, flusterstorm());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ritual],
        "the only instant or sorcery on the stack for it to point at"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    let stack = engine.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 3);
    assert_eq!(&stack[..2], &[ritual, fluster]);
    keep_storm_copy_targets(&mut engine, p0);

    // Both pass, the copy resolves, and the target controller is taxed.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("the walk above stops on the tax question")
    };
    assert_eq!(*mana, 1, "Flusterstorm prints a {{1}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" is the countered spell's controller, \
         not the seat that cast Flusterstorm"
    );

    let pool_before = engine.state().players[1].mana_pool.total();
    assert!(
        pool_before >= 1,
        "the second Swamp is what leaves the tax payable, and so what makes \
         the question askable at all"
    );
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is an answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before,
        "the Ritual was countered, so it added no {{B}}{{B}}{{B}} — and the \
         tax went unpaid, so nothing left the pool for it either"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p0, flusterstorm()).is_some(),
        "the original Flusterstorm has no legal target and goes to its graveyard"
    );
}

/// "Counter target instant or sorcery spell unless its controller pays {1}":
/// the paying branch. Both the storm copy and the original ask; paying each
/// time takes exactly {1} from the pool and the Ritual is not countered.
#[test]
fn a_paid_flusterstorm_tax_takes_one_mana_and_leaves_the_spell_standing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(59, forest())
        .battlefield(0, &[island()])
        .hand(0, &[flusterstorm()])
        // Three Swamps: the Ritual leaves two floating, one for each of the
        // two taxes (the storm copy's and the original's).
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p0, flusterstorm());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();
    keep_storm_copy_targets(&mut engine, p0);

    let tax_is_asked = |e: &Engine<RegistryLookup>| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { .. },
                ..
            }
        )
    };
    pass_until(&mut engine, tax_is_asked);
    let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the walk above stops on the tax question")
    };
    assert_eq!(player, p1, "the countered spell's controller is asked");
    assert_eq!(mana, 1, "Flusterstorm prints a {{1}} tax");
    let pool_before = engine.state().players[1].mana_pool.total();
    assert_eq!(
        pool_before, 2,
        "the three Swamps less the Ritual's {{B}} are floating"
    );

    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("paying is an answer");

    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before - 1,
        "paying the tax takes exactly {{1}} from the pool"
    );
    assert_eq!(
        engine.state().object(ritual).map(|o| o.zone),
        Some(Zone::Stack),
        "the tax was paid, so the copy did not counter the Ritual"
    );

    // The original Flusterstorm asks the same question of the same spell.
    pass_until(&mut engine, |e| tax_is_asked(e) || stack_is_empty(e));
    let mut payments = 1;
    if tax_is_asked(&engine) {
        engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
        payments += 1;
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(
        payments, 2,
        "the copy and the original each taxed the Ritual"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before - 2 + 3,
        "two {{1}} taxes paid, and the uncountered Ritual added its {{B}}{{B}}{{B}}"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the Ritual resolved and went to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, flusterstorm()).is_some(),
        "the original Flusterstorm resolved too, to its graveyard"
    );
}

/// A copy of a **synthetic** triggered ability (CR 707.10). Prowess keeps
/// its effect beside the engine and not on the ability's object, so the
/// copy has to be handed it (`GameState::synthetic_copies`). Dark Ritual
/// makes Pinnacle Monk's prowess trigger; Vantress Visions copies it, and is
/// itself a noncreature spell, so prowess triggers again: three resolutions,
/// and the 2/2 is a 5/5.
#[test]
fn vantress_visions_copies_a_prowess_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pinnacle_monk(), swamp(), island(), island()])
        .hand(0, &[dark_ritual(), virtue_of_knowledge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let monk = on_battlefield(&engine, p0, pinnacle_monk()).expect("the Monk");
    let swamp = on_battlefield(&engine, p0, swamp()).expect("the Swamp");
    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(pt(&engine, monk), (2, 2));

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp })
        .expect("the Swamp taps for {B}");
    cast_with_floating(&mut engine, p0, dark_ritual());
    let prowess = top_of_stack(&engine);
    assert_eq!(
        engine.state().object(prowess).map(|o| o.kind),
        Some(crate::object::ObjectKind::AbilityOnStack),
        "prowess triggered over the Ritual"
    );
    for &island in &islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: island })
            .expect("an Island taps for {U}");
    }
    cast_vantress_visions(&mut engine, p0, prowess);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, monk),
        (5, 5),
        "prowess for the Ritual, its copy, and prowess for Visions"
    );
}

fn keep_storm_copy_targets(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
}
