//! `cards/creatures/mv_4/wild_elephant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "The active player has controlled continuously since the beginning of
/// the turn" is only ever proven against a creature seated at game start
/// versus one cast the same turn. Proven here instead on turn 3, not turn
/// 1, so what is read is the ordinary per-turn mechanism and not turn 1's
/// own "no seat has had a turn yet" dispensation (`Engine::new`): an
/// Elephant that has sat on p0's battlefield since long before turn 3
/// began is on the Imp's menu, targetable, required and — attacked —
/// spared; a Bear p0 takes from p1 with Control Magic in the very main
/// phase the Imp is about to reach is not, however the destruction turns
/// out, since it is never even offered as a target.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn nettling_imp_reaches_a_creature_held_since_an_earlier_turn_but_not_one_taken_this_turn() {
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
                wild_elephant(),
            ],
        )
        .hand(0, &[control_magic()])
        .battlefield(1, &[nettling_imp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let old_creature =
        on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    let imp = on_battlefield(&engine, p1, nettling_imp()).expect("the Imp is seated");
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("p1's own Bear is seated");

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

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: imp,
                ability_index: 0,
            },
        )
        .expect("offered in p0's beginning of combat, before attackers are declared");
    let options = aim_at(&mut engine, p1, old_creature);
    assert!(
        options.contains(&old_creature),
        "held since long before turn 3 began: on the menu"
    );
    assert!(
        !options.contains(&their_bear),
        "taken this very turn: not controlled continuously since turn 3 \
         began, so never offered — whatever the destruction would later \
         make of it"
    );
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
    assert_eq!(
        required,
        vec![old_creature],
        "\"attacks this turn if able\""
    );
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "leaving out the Elephant disobeys the Imp's requirement"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(old_creature, Defender::Player(p1))],
            },
        )
        .expect("attacking with it obeys the requirement");

    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_some(),
        "it attacked this turn, so the delayed destruction spares it"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the Imp's ability was never used on the stolen Bear at all"
    );
}

/// The same "controlled continuously since the beginning of the turn"
/// read, on the destroying branch: an Elephant that has sat on p0's
/// battlefield since long before turn 3 began is targeted, then tapped
/// before the declare-attackers turn-based action, and destroyed for not
/// attacking exactly as a creature held since turn 1 would be — this is
/// not the trivial turn-1 case (`Engine::new`'s dispensation for a seat
/// that has not had a turn yet), because it is checked on turn 3.
#[test]
fn nettling_imp_destroys_a_creature_held_since_an_earlier_turn_when_it_could_not_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wild_elephant()])
        .battlefield(1, &[nettling_imp()])
        .start();
    keep_mulligans(&mut engine);

    let old_creature =
        on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    let imp = on_battlefield(&engine, p1, nettling_imp()).expect("the Imp is seated");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: imp,
                ability_index: 0,
            },
        )
        .expect("offered in p0's main phase, before attackers are declared");
    aim_at(&mut engine, p1, old_creature);
    pass_until(&mut engine, stack_is_empty);

    // Tapped after the forced-attack effect is already in place, the
    // Elephant cannot obey it.
    engine
        .dev_state_mut(p0)
        .expect("a test seat has dev commands")
        .set_tapped(old_creature, true);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        required.is_empty(),
        "tapped, the Elephant is not able to attack, so nothing is \
         required of it any more"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        on_battlefield(&engine, p0, wild_elephant()),
        Some(old_creature),
        "combat is over and the end step has not begun yet: not yet claimed"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_none(),
        "held since long before turn 3 began, and it did not attack this \
         turn: the Imp's delayed destruction claims it exactly as it \
         would have on turn 1"
    );
}
