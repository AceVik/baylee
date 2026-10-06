//! `cards/creatures/mv_1/soul_warden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Benevolent Geist, Malevolent Hermit's disturb back: "Disturb {2}{U}",
/// "Flying", "Noncreature spells you control can't be countered." and "If
/// Benevolent Geist would be put into a graveyard from anywhere, exile it
/// instead." The Hermit is cast from the graveyard transformed for three
/// mana and arrives as a 2/2 flying Geist. The shield is for spells: Soul
/// Warden's trigger on the Geist's arrival is an ability of the same
/// controller and stays counterable. A sorcery its controller casts then
/// survives a Counterspell (which may still target it and is still spent),
/// and the Geist that dies goes to exile, so it cannot be disturbed again.
#[test]
fn benevolent_geist_is_disturbed_shields_noncreature_spells_and_is_exiled() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(98, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                soul_warden(),
            ],
        )
        .hand(0, &[malevolent_hermit(), counsel_of_the_soratami()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Disturb: the harness puts the Hermit in the graveyard; how it got
    // there is not what this test reads.
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
    // Six blue float: three for the disturb cost, three for the sorcery.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("disturb is offered from the graveyard");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, malevolent_hermit()).is_some()
    });
    let warden_trigger = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("Soul Warden saw the Geist arrive");
    // The Geist's static reaches the stack, so while it stands every
    // refresh projects every object, the trigger included; the next spell
    // moving would start one. The harness starts it here instead, with the
    // trigger still waiting.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may refresh the projection");
    state.invalidate_projections();
    state.refresh_characteristics();
    assert!(
        engine
            .state()
            .object(warden_trigger)
            .is_some_and(crate::object::GameObject::can_be_countered),
        "an ability is no noncreature spell"
    );
    pass_until(&mut engine, stack_is_empty);
    let geist = on_battlefield(&engine, p0, malevolent_hermit()).expect("the Geist is out");
    assert_eq!(pt(&engine, geist), (2, 2), "Benevolent Geist is a 2/2");
    assert!(
        engine
            .state()
            .object(geist)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "with flying"
    );

    // The sorcery is countered in name only: Counterspell resolves, the
    // Counsel stays, and then it draws its two cards.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_with_floating(&mut engine, p0, counsel_of_the_soratami());
    let counsel = on_stack(&engine, counsel_of_the_soratami()).expect("the Counsel is cast");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, counterspell());
    aim_at(&mut engine, p1, counsel);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        card_in(&engine, ZoneLocation::Graveyard(p1), counterspell()).is_some(),
        "the Counterspell was cast and resolved"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 2,
        "the Counsel was not countered: it drew two"
    );

    kill(&mut engine, geist);
    assert!(
        card_in(&engine, ZoneLocation::Graveyard(p0), malevolent_hermit()).is_none(),
        "the Geist never reached the graveyard"
    );
    assert!(
        card_in(&engine, ZoneLocation::Exile(p0), malevolent_hermit()).is_some(),
        "it was exiled instead"
    );
}

/// Soul Warden is a 1/1 for {W} with exactly one printed line: "Whenever
/// **another** creature enters, you gain 1 life" — every creature,
/// not just your own. The scenario is built on exactly that: the Warden
/// itself enters first and the life buffer does not move, because it is
/// not "another" to itself; the Goblin next to it is the +1 that proves
/// the trigger is running at all; and the opponent's Goblin in their own
/// turn is the second +1 that no reading of "another creature you
/// control" would produce. Both life totals are read, so that the life
/// lands with the controller of the Warden and not with the one whose
/// creature entered.
#[test]
fn soul_warden_gains_life_for_another_creature_and_not_for_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(151, forest())
        .battlefield(0, &[plains(), swamp()])
        .hand(0, &[soul_warden(), festering_goblin()])
        .battlefield(1, &[swamp()])
        .hand(1, &[festering_goblin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W} and {B} are in the pool, and both casts below pay from it: the
    // Warden recognizes "another creature" by life total, not by mana.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, soul_warden());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, soul_warden()).is_some(),
        "die Warden ist angekommen"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"whenever *another* creature enters\" — the Warden is not another \
         creature to itself, so it does not trigger its own entry"
    );

    cast_with_floating(&mut engine, p0, festering_goblin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "der Goblin ist neben ihr angekommen"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "a second creature entered, and the Warden's controller gained 1 life"
    );

    // An opponent's creature during their own turn: "another creature"
    // names no controller, so the Warden watches over the whole table,
    // and the life continues to its controller.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, festering_goblin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "der Goblin des Gegners ist angekommen"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "the Warden also saw this creature enter"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and whoever controls the entering creature gets nothing"
    );
}
