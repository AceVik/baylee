//! `cards/instants/mv_1/dark_ritual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brain Freeze, {1}{U}: "Target player mills three cards." — and under it,
/// "Storm (When you cast this spell, copy it for each spell cast before it
/// this turn. You may choose new targets for the copies.)"
///
/// The card is `Coverage::Partial`, so both halves are played here. A test
/// that only counted the three cards would be green on a card that prints two
/// sentences and plays one — and the unplayed one is the sentence the card is
/// famous for.
///
/// Dark Ritual is cast first for exactly one reason: it makes the turn's
/// spell count one, which the engine already keeps in `per_turn.spells_cast`
/// and which is asserted below. So the copy that never appears is a copy
/// storm would have been owed, and not one that had nothing to count. The
/// Ritual is asked nothing on the way — its whole text is "Add {B}{B}{B}" —
/// and it is cast off the Swamp alone, leaving both Islands for the spell
/// under test.
///
/// The mill half is aimed across the table, which is what "target player"
/// buys: the three cards have to leave the named seat's library and no other.
#[test]
fn brain_freeze_mills_three_and_the_storm_it_prints_never_copies_it() {
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

    // The storm half, at the one moment a copy would be visible: it is put on
    // the stack as the spell is cast, and is offered new targets of its own.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "storm is not written: the turn's second spell went on the stack alone"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "and nothing is asked to be re-aimed, because there is no copy: {:?}",
        engine.pending()
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 3,
        "three cards off the top of the named seat's library — three, not six"
    );
    assert_eq!(
        graveyard(&engine, p1),
        their_graveyard + 3,
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

/// Flusterstorm: "Counter target instant or sorcery spell unless its
/// controller pays {1}" — and, printed beside it, storm, which is the
/// `Coverage::Partial` this card declares.
///
/// A Partial card owes both halves, so both are here.
///
/// The half it has: the tax is put to the *countered spell's* controller
/// and not to the seat that cast Flusterstorm, and declining it is a
/// counter rather than a fizzle. The mana pool is the whole proof of that
/// last word — a Dark Ritual that resolved and a Dark Ritual that was
/// countered both end in p1's graveyard, so the zone distinguishes nothing
/// and the number does: it would be three higher had the Ritual resolved,
/// and one lower had the tax been paid.
///
/// The half it has not: Dark Ritual was cast this turn before Flusterstorm,
/// so storm would copy it once and put its own "when you cast this spell"
/// trigger on the stack above the two spells. The stack holds the two
/// spells and nothing else, which is the card playing as though the line
/// were not printed — and the assertion that stops passing the day storm
/// is written.
#[test]
fn a_declined_flusterstorm_counters_the_spell_and_never_copies_itself() {
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

    // The gap, measured rather than asserted about the card file: one spell
    // was cast before this one this turn, so storm would have copied it once
    // and its trigger would be sitting on top of both of these.
    let stack = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .clone();
    assert_eq!(
        stack,
        vec![ritual, fluster],
        "the two spells, bottom to top, and nothing above them: storm prints \
         a \"when you cast this spell\" trigger this card does not have"
    );

    // Both pass, Flusterstorm resolves, and the tax is put to somebody.
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
        "and Flusterstorm, having resolved, follows it there"
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
