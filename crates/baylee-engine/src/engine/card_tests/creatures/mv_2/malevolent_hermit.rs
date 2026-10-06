//! `cards/creatures/mv_2/malevolent_hermit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Malevolent Hermit's whole printed text is one line: "{U}, Sacrifice this
/// creature: Counter target noncreature spell unless its controller pays
/// {3}." Two spells of different kinds stand on the stack at once, so the
/// target menu is the evidence for the filter — the Ritual on it and the
/// Elves not. The price then falls on the *target's* controller rather than
/// the Hermit's, which is what `PlayerRel::ControllerOfTarget` gets right or
/// wrong in silence, and declining to pay is what turns "unless" into the
/// countered spell in a graveyard. The Hermit in its owner's graveyard is
/// the sacrifice having been paid as a cost rather than promised.
#[allow(clippy::too_many_lines)] // two spells on the stack, the menu between them and the tax after
#[test]
fn malevolent_hermit_taxes_a_noncreature_spell_and_pays_for_it_with_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, island())
        .battlefield(0, &[malevolent_hermit(), island()])
        .battlefield(1, &[forest(), swamp(), swamp(), swamp(), swamp()])
        .hand(1, &[llanowar_elves(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // One spell of each kind, so the menu has something it must offer and
    // something it must decline.
    cast_from_hand(&mut engine, p1, llanowar_elves());
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    // The Island is tapped first: an ability whose cost nobody can pay is
    // absent from the offer, which would make the assertion below pass for a
    // reason that has nothing to do with the Hermit.
    tap_all_mana(&mut engine, p0);
    let ritual = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == dark_ritual()))
        })
        .expect("the Ritual is on the stack");
    assert!(
        on_battlefield(&engine, p0, malevolent_hermit()).is_some(),
        "the Hermit is standing, and nothing has been sacrificed yet"
    );

    // Ability 0 is the printed line; the offer is part of the assertion.
    activate(&mut engine, p0, malevolent_hermit(), 0);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target noncreature spell\" asks for one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that picks");
    assert_eq!(
        options,
        vec![ritual],
        "the Ritual is a noncreature spell and the Elves are a creature \
         spell, which is the whole of the filter"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the target was one of the options");
    assert!(
        in_graveyard(&engine, p0, malevolent_hermit()).is_some(),
        "the creature is the cost, paid as the ability is activated \
         (CR 601.2h), so it is in its owner's graveyard before anybody is \
         asked for {{3}}"
    );

    // The ability resolves: the price lands on the Ritual's controller.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the yes/no it was waiting for")
    };
    assert_eq!(
        player, p1,
        "`PlayerRel::ControllerOfTarget`: the price falls on the spell's \
         controller and not on the Hermit's"
    );
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayTax { mana: 3 },
        "the printed {{3}}"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "and the pool could pay it, so declining is a choice and not an \
         inability"
    );

    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining answers out of the question's own enumeration");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the unpaid Ritual was countered, and a countered spell goes to its \
         owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "and the {{3}} stayed in the pool: the spell was countered, not \
         bought off"
    );
    assert!(
        on_battlefield(&engine, p0, malevolent_hermit()).is_none(),
        "the Hermit is still the payment for an ability already spent"
    );
}

/// Benevolent Geist's mana value is Malevolent Hermit's {1}{U}, 2, not its
/// disturb cost's 3: on the stack, because it was cast transformed (CR
/// 712.8c), and on the battlefield with its back face up (CR 712.8e).
#[test]
fn a_disturbed_geist_has_the_hermits_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(98, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[malevolent_hermit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = in_hand(&engine, p0, malevolent_hermit()).expect("the Hermit is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            card,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("into the graveyard");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("disturb is offered from the graveyard");
    let mana_value = |engine: &Engine<RegistryLookup>, id: ObjectId| {
        engine
            .state()
            .object(id)
            .map(|o| o.characteristics().mana_value())
    };
    let spell = on_stack(&engine, malevolent_hermit()).expect("the Geist is cast");
    assert_eq!(
        mana_value(&engine, spell),
        Some(2),
        "the Hermit's {{1}}{{U}}"
    );
    pass_until(&mut engine, stack_is_empty);
    let geist = on_battlefield(&engine, p0, malevolent_hermit()).expect("the Geist is out");
    assert_eq!(face_shown(&engine, geist), 1, "Benevolent Geist");
    assert_eq!(
        mana_value(&engine, geist),
        Some(2),
        "and on the battlefield"
    );
}
