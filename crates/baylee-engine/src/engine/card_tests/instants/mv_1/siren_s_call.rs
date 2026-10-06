//! `cards/instants/mv_1/siren_s_call.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Siren's Call: "Cast this spell only during an opponent's turn, before
/// attackers are declared." Offered to p1 through p0's beginning of
/// combat, refused from the declare attackers step on, and never offered
/// at all on p1's own turn, however early. An Island is tapped fresh at
/// every checkpoint, open or shut, so a "not castable" reading is never
/// merely "no floating mana".
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn siren_s_call_is_castable_before_p0_declares_attackers_and_never_on_p1_s_own_turn() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            1,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(1, &[siren_s_call()])
        .start();
    keep_mulligans(&mut engine);

    let castable = |e: &Engine<RegistryLookup>| {
        let spell = in_hand(e, p1, siren_s_call()).expect("Siren's Call is still in hand");
        priority_offer(e).castable.contains(&spell)
    };

    // Turn 1 is p0's — an opponent's turn for p1 — and open until attackers
    // are declared.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        castable(&engine),
        "p0's upkeep, before attackers are declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        castable(&engine),
        "p0's main phase, before attackers are declared"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        castable(&engine),
        "beginning of combat, still before attackers"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        !castable(&engine),
        "the declare attackers step has begun — CR 506.7a reads that as \
         the window closing regardless of whether p0 (with nothing to \
         attack with here) actually declared anyone — and the mana just \
         floated proves this is not about affording it"
    );
    let refused = in_hand(&engine, p1, siren_s_call()).expect("Siren's Call is still in hand");
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: refused })
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(!castable(&engine), "well past the declare attackers step");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(!castable(&engine), "the end step, later still");

    // Turn 2 is p1's own — never a legal window regardless of step.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(!castable(&engine), "p1's own upkeep");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(!castable(&engine), "p1's own main phase");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        !castable(&engine),
        "p1's own beginning of combat: still before attackers, but not an opponent's turn"
    );
}

/// Siren's Call: "Creatures the active player controls attack this turn
/// if able" forces a declaration that cannot leave out an able creature.
/// "Destroy all non-Wall creatures that player controls that didn't
/// attack this turn. Ignore this effect for each creature the player
/// didn't control continuously since the beginning of the turn": an
/// ordinary creature unable to obey the forced attack is claimed, but a
/// Wall and a creature cast this same turn are spared even though neither
/// attacked either; the creature that did attack is spared for that; and
/// p1's own board is never touched.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn siren_s_call_forces_the_attack_and_destroys_only_who_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                llanowar_elves(),
                wild_elephant(),
                wall_of_swords(),
            ],
        )
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[island(), island(), island(), grizzly_bears()])
        .hand(1, &[siren_s_call()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    let wall = on_battlefield(&engine, p0, wall_of_swords()).expect("the Wall is seated");

    engine
        .dev_state_mut(p0)
        .expect("a test seat has dev commands")
        .set_tapped(elephant, true);
    engine.refresh_offer();

    // Not `cast_from_hand`: the Elf prints its own "{T}: Add {G}" and would
    // otherwise pay for the Bear, tapping itself out of the very
    // declaration this test is about.
    tap_mana_except(&mut engine, p0, elf);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let fresh_bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the fresh Bear resolved");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    cast_from_hand(&mut engine, p1, siren_s_call());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert!(attackers.contains(&elf));
    assert_eq!(
        required,
        vec![elf],
        "only the Elf is able to attack: the Elephant is tapped, the Wall \
         has defender, and the Bear cast this turn is summoning sick"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "\"attack this turn if able\": leaving out the one creature that \
         could obeys nothing"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .expect("attacking with the required Elf obeys the requirement");

    // `pass_until` declares empty blockers on the way for us (p1's own Bear
    // could block but the point here is combat's aftermath, not blocking).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_some(),
        "combat is over and the end step has not begun yet: the delayed \
         destruction has not fired"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "same reading for the creature the trigger will end up sparing"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "it attacked this turn — the effect's own exception"
    );
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_none(),
        "tapped and unable to obey the forced attack, a non-Wall the \
         active player held since the turn began: destroyed"
    );
    assert_eq!(
        on_battlefield(&engine, p0, wall_of_swords()),
        Some(wall),
        "a Wall is never touched, attacked or not"
    );
    assert_eq!(
        on_battlefield(&engine, p0, grizzly_bears()),
        Some(fresh_bear),
        "cast this turn: not controlled since the turn began, so the \
         delayed effect ignores it even though it too did not attack"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "p1's own creature was never the active player's to touch"
    );
}

/// "Ignore this effect for each creature the player didn't control
/// continuously since the beginning of the turn" scopes only the destroy
/// sentence — the force-attack sentence before it carries no such clause
/// of its own. Proven on turn 3, not turn 1, so what is read is the
/// ordinary per-turn mechanism and not turn 1's own "no seat has had a
/// turn yet" dispensation (`Engine::new`): p0 steals a Bear from p1 with
/// Control Magic in the very main phase Siren's Call is about to reach —
/// too fresh, this turn, to be reached by the destroy sentence, and
/// (for an ordinary reason: summoning sickness) not by the force-attack
/// sentence either — beside two creatures already on the battlefield since
/// long before turn 3 began, one left free to attack and survive, one
/// tapped just before the declare-attackers turn-based action and
/// destroyed for not attacking despite its age.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn siren_s_call_ignores_a_creature_taken_this_turn_but_reaches_one_held_since_an_earlier_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
                wild_elephant(),
                rib_cage_spider(),
            ],
        )
        .hand(0, &[control_magic()])
        .battlefield(1, &[island(), grizzly_bears()])
        .hand(1, &[siren_s_call()])
        .start();
    keep_mulligans(&mut engine);

    let old_attacker =
        on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    let old_stay_home =
        on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("p1's Bear is seated");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    cast_from_hand(&mut engine, p0, control_magic());
    aim_at(&mut engine, p0, their_bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "Control Magic's static ability moved the Bear to p0's side"
    );

    // Tapped now, in the same priority window, before the declare-attackers
    // turn-based action computes who is required — not after.
    engine
        .dev_state_mut(p0)
        .expect("a test seat has dev commands")
        .set_tapped(old_stay_home, true);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    cast_from_hand(&mut engine, p1, siren_s_call());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, required, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert!(
        required.contains(&old_attacker),
        "held since long before turn 3 began, able, and the active \
         player's: \"attacks this turn if able\" reaches it"
    );
    assert!(
        !required.contains(&old_stay_home),
        "tapped moments ago: not able, so \"if able\" cannot reach it \
         either, regardless of age"
    );
    assert!(
        !required.contains(&their_bear),
        "taken this very turn: summoning sick for its new controller \
         (CR 302.6), so \"if able\" cannot reach it — an ordinary \
         summoning-sickness exclusion, since the force-attack sentence \
         itself carries no \"since the turn began\" clause of its own; \
         only the destroy sentence below does"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "leaving out the one able, required creature obeys nothing"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(old_attacker, Defender::Player(p1))],
            },
        )
        .expect("attacking with it obeys the requirement");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_some(),
        "combat is over and the end step has not begun yet: not yet claimed"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some(),
        "same reading for the creature the trigger will end up destroying"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_some(),
        "it attacked this turn — the effect's own exception"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_none(),
        "held since long before turn 3 began, non-Wall, and didn't attack: \
         destroyed exactly as it would have been on turn 1"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "taken this turn: not controlled continuously since the turn \
         began, so the destroy sentence's own exception spares it even \
         though it too never attacked"
    );
}

/// Siren's Call's force-attack sentence reaches a creature the active
/// player took this turn, as soon as it is able to attack. The Gatherer
/// ruling (2004-10-04): "It will require creatures with Haste to attack
/// since they are able, but it won't destroy them if they don't for some
/// reason." Only the destroy sentence carries "Ignore this effect for each
/// creature the player didn't control continuously since the beginning of
/// the turn", so the requirement's filter stays without
/// `ControlledSinceTurnBegan`: p0 steals p1's Bear with Control Magic,
/// equips it with Lightning Greaves (haste), and must attack with it.
#[test]
fn siren_s_call_requires_a_creature_taken_this_turn_that_has_haste_to_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), island(), island(), lightning_greaves()],
        )
        .hand(0, &[control_magic()])
        .battlefield(1, &[island(), grizzly_bears()])
        .hand(1, &[siren_s_call()])
        .start();
    keep_mulligans(&mut engine);
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("p1's Bear is seated");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are seated");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, control_magic());
    aim_at(&mut engine, p0, their_bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "Control Magic moved the Bear to p0's side this turn"
    );
    // Ability 1 is Equip {0}.
    activate(&mut engine, p0, lightning_greaves(), 1);
    aim_at(&mut engine, p0, their_bear);
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(their_bear))
    });
    assert!(
        keywords(&engine, their_bear).contains(KeywordSet::HASTE),
        "equipped, so able to attack this turn"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    cast_from_hand(&mut engine, p1, siren_s_call());
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        required.contains(&their_bear),
        "the active player's and able: \"attack this turn if able\" reaches it"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "a declaration without the hasty Bear obeys nothing"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(their_bear, Defender::Player(p1))],
            },
        )
        .expect("attacking with it obeys the requirement");
}
