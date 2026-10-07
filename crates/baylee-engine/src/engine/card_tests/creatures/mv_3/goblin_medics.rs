//! `cards/creatures/mv_3/goblin_medics.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Medics — {2}{R}, a 1/1 Goblin Shaman — prints one line: "Whenever
/// this creature becomes tapped, it deals 1 damage to any target."
///
/// So the scenario has to produce a tap, and attacking is the tap the rules
/// hand it: declaring the Medics as an attacker taps it and puts the trigger
/// on the stack (CR 508.2), before any combat damage exists to be mistaken
/// for it. That mistake is why the damage is aimed at a 1/1 Elf across the
/// table rather than at the player — one life lost by a player would have
/// been explained just as well by the Medics' own combat damage a step later,
/// while a dead Elf with the opponent still at 20 life at that moment can only
/// be the trigger. The card is cast for its printed {2}{R} off three
/// Mountains, and the turn cycle before the attack is what summoning sickness
/// costs (CR 302.6).
#[test]
fn goblin_medics_deals_its_damage_when_it_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[goblin_medics()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{R} off the three Mountains, so the card arrives the way the card
    // arrives instead of merely being believed in.
    cast_from_hand(&mut engine, p0, goblin_medics());
    pass_until(&mut engine, stack_is_empty);
    let medics = on_battlefield(&engine, p0, goblin_medics()).expect("the Medics resolved");
    assert_eq!(pt(&engine, medics), (1, 1), "the body the card prints");

    // A creature cast this turn is summoning sick, so the attack is a whole
    // turn cycle away — and the walk is where the Medics untaps again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1, which is one damage from death"
    );
    assert!(
        !is_tapped(&engine, medics),
        "and the Medics untapped on the way back"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&medics),
        "an untapped, unsick Medics may be declared: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(medics, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, medics),
        "attacking is the tap the card is about"
    );

    // The trigger is on the stack and asks for its target while the Medics is
    // still unblocked and no combat damage has been dealt by anybody.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the target question")
    };
    assert_eq!(player, p0, "the controller of the tapped creature chooses");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&elf),
        "\"any target\" reaches a creature across the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: and players are in the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the trigger is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the Elf died in the declare attackers step, before the Medics \
         could have dealt combat damage: the untouched life total is what \
         pins the kill to the trigger"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage belongs to the target that was named and to nobody else"
    );
}
