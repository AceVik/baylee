//! `cards/artifacts/mv_4/jade_monolith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jade Monolith: "{1}: The next time a source of your choice would deal
/// damage to target creature this turn, that source deals that damage to
/// you instead." Activated after blocks, aimed at the blocking Elf and
/// naming the attacking Giant as the source: the Giant's combat damage
/// reaches the Monolith's controller instead of the Elf, whose own damage
/// to the Giant is untouched, and the shield — "the next time" — is gone
/// after moving that one instance.
#[test]
fn jade_monolith_sends_a_blocked_attackers_combat_damage_to_its_controller_instead_of_the_blocker()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[jade_monolith(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;
    let before_journal = engine.journal().entries().len();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .expect("the Giant attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, giant)],
            },
        )
        .expect("the Elf blocks");
    pass_until(
        &mut engine,
        priority_in(crate::turn::Step::DeclareBlockers, p1),
    );

    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the blocking Elf is a legal target");
    let options = choose_monolith_source(&mut engine, p1, giant);
    assert!(
        options.contains(&giant),
        "the attacker is a source to choose: {options:?}"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        before_life - 3,
        "the Giant's power, redirected onto its controller"
    );
    assert_eq!(
        engine.state().object(elf).unwrap().damage,
        0,
        "the Elf was dealt nothing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and survives"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        1,
        "the Elf's own 1 damage to the Giant: untouched"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        3,
        "counted for the player, since it landed on them"
    );
    // "The next time": spent after the one instance it moved. Nothing in
    // this fixture gives the Giant a second instance of damage to the Elf
    // this same turn to show the shield declining it directly, so the
    // shield list itself is the check.
    assert!(
        engine.state().shields.is_empty(),
        "the shield is gone after moving its one instance"
    );
    assert_eq!(
        damage_events(&engine, before_journal),
        vec![
            (giant, crate::event::DamageTarget::Player(p1), 3, true),
            (elf, crate::event::DamageTarget::Object(giant), 1, true),
        ],
        "the Giant's redirected hop to the player, and the Elf's own untouched hit back"
    );
}

/// Jade Monolith — "The next time a source of your choice would deal
/// damage to target creature this turn, that source deals that damage
/// to you instead": the chosen source need not be a permanent. With a
/// Lightning Bolt still on the stack, aimed at the same creature,
/// choosing the Bolt itself as the source (CR 609.7a) sends its damage
/// to the Monolith's controller instead.
#[test]
fn jade_monolith_sends_a_targeted_lightning_bolts_damage_to_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[jade_monolith(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, lightning_bolt());
    let menu = aim_at(&mut engine, p0, elf);
    assert!(menu.contains(&elf));
    let bolt_id = on_stack(&engine, lightning_bolt()).expect("the Bolt is on the stack");

    pass_until(&mut engine, priority_in(crate::turn::Step::Main, p1));
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    let options = choose_monolith_source(&mut engine, p1, bolt_id);
    assert!(
        options.contains(&bolt_id),
        "a spell on the stack is a source to choose: {options:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        before_life - 3,
        "the Bolt's damage, redirected onto the Monolith's controller"
    );
    assert_eq!(
        engine.state().object(elf).unwrap().damage,
        0,
        "the Elf was dealt nothing"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "the Bolt still resolved and went to its own graveyard"
    );
}

/// Jade Monolith counter-check, on the same Lightning-Bolt-on-the-stack
/// board: naming a different source — a land, never the Bolt — leaves the
/// shield unmatched this turn (CR 609.7b: it prevents or replaces nothing,
/// so it isn't used up either), and the Bolt's 3 damage reaches the Elf
/// exactly as it would without the Monolith.
#[test]
fn a_shield_for_another_source_leaves_a_targeted_lightning_bolts_damage_on_the_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[jade_monolith(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, elf);

    pass_until(&mut engine, priority_in(crate::turn::Step::Main, p1));
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    // The land itself: any permanent is a legal "source of your choice"
    // (CR 609.7a), tapped or not; it just never deals the Elf any damage.
    choose_monolith_source(&mut engine, p1, land);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Bolt's 3 on a 1-toughness Elf, undisturbed"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "the wrong source: nothing came to the controller"
    );
    // CR 609.7b: "If for any reason the shield prevents no damage or
    // replaces no damage, the shield isn't used up."
    assert_eq!(
        engine.state().shields.len(),
        1,
        "still waiting: it matched nothing, so it is not spent"
    );
}

/// Jade Monolith counter-check: "a source of your choice" — with a
/// different source chosen, the shield never matches. The Giant's combat
/// damage lands on the Elf exactly as it would without the Monolith, and
/// the untouched shield — replacing no damage, so it "isn't used up"
/// (CR 609.7b) — is still waiting mid-turn, not yet gone.
#[test]
fn a_shield_for_another_source_leaves_the_blocked_attackers_damage_on_the_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[jade_monolith(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let monolith = on_battlefield(&engine, p1, jade_monolith()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .expect("the Giant attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, giant)],
            },
        )
        .expect("the Elf blocks");
    pass_until(
        &mut engine,
        priority_in(crate::turn::Step::DeclareBlockers, p1),
    );

    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the blocking Elf is a legal target");
    // The Monolith itself: any permanent is a legal "source of your
    // choice" (CR 609.7a); it just never deals the Elf any damage.
    choose_monolith_source(&mut engine, p1, monolith);
    assert_eq!(engine.state().shields.len(), 1, "made, and waiting");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Giant's 3 on a 1-toughness Elf, undisturbed"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "the wrong source: nothing came to the controller"
    );
    // CR 609.7b: "If for any reason the shield prevents no damage or
    // replaces no damage, the shield isn't used up."
    assert_eq!(
        engine.state().shields.len(),
        1,
        "still waiting: it matched nothing, so it is not spent"
    );
}

/// Jade Monolith counter-check: "this turn" — a shield that never moves
/// anything is gone at cleanup regardless (CR 514.2), and the same
/// source's damage to the same creature next turn lands on it normally.
#[test]
fn an_unfired_jade_monolith_shield_expires_at_cleanup_and_the_same_source_hits_normally_next_turn()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[jade_monolith(), earth_elemental(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;

    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("p0 passes without attacking");
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elemental],
                players: vec![],
            },
        )
        .expect("their own Elemental is a legal target too");
    choose_monolith_source(&mut engine, p1, giant);
    assert_eq!(engine.state().shields.len(), 1, "made, and waiting");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && e.state().turn.number == 1
    });
    assert_eq!(
        engine.state().shields.len(),
        1,
        "still there in turn 1's ending phase: gone only at cleanup, not before"
    );

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert!(
        engine.state().shields.is_empty(),
        "\"this turn\": gone at cleanup, whether or not it ever moved anything"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .expect("the Giant attacks again");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elemental, giant)],
            },
        )
        .expect("the Elemental blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(elemental).unwrap().damage,
        3,
        "the same source's damage, on the creature: the expired shield caught nothing"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "no redirection happened"
    );
    assert!(engine.state().shields.is_empty(), "never recreated");
}

/// Jade Monolith: "you" is the activating player (CR 109.5), not the
/// target creature's controller. p1's Monolith targets p0's blocker and
/// names p1's own attacker as the source: the attacker's combat damage
/// goes to p1 — the Monolith's controller — instead of marking p0's
/// creature, whose own combat damage back at the attacker is untouched,
/// and p0's life never moves (the damage was headed for a blocker, never
/// for p0, either way).
#[test]
fn jade_monoliths_you_is_its_controller_not_the_targeted_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[jade_monolith(), earth_elemental(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life0 = engine.state().players[0].life;
    let before_life1 = engine.state().players[1].life;

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elemental, Defender::Player(p0))],
            },
        )
        .expect("p1's Elemental attacks p0");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, elemental)],
            },
        )
        .expect("p0's Giant blocks");
    pass_until(
        &mut engine,
        priority_in(crate::turn::Step::DeclareBlockers, p1),
    );

    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .expect("p0's Giant, the opponent's creature, is a legal target too");
    choose_monolith_source(&mut engine, p1, elemental);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        before_life1 - 4,
        "the Elemental's power lands on its own controller: the Monolith's activator"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        0,
        "p0's Giant, the target creature, never marked: its controller is not \"you\""
    );
    assert_eq!(
        engine.state().players[0].life,
        before_life0,
        "p0 was never in this damage's path, blocking or not"
    );
    assert_eq!(
        engine.state().object(elemental).unwrap().damage,
        3,
        "the Giant's own 3 back at the blocked Elemental: untouched"
    );
}

/// Both cards at once: an untapped Veteran Bodyguard takes an unblocked
/// attacker's damage off its controller (CR 614.9), and a Jade Monolith
/// aimed at the Bodyguard, naming that same attacker, catches it there
/// and sends it on to the controller after all — the Bodyguard's static
/// applied once to this damage and does not take it back (CR 614.5).
#[test]
fn veteran_bodyguard_and_jade_monolith_together_the_bodyguard_does_not_take_back_what_the_monolith_sent_away()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[veteran_bodyguard(), jade_monolith(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    let land = on_battlefield(&engine, p1, forest()).expect("seated");
    let before_life = engine.state().players[1].life;
    let before_journal = engine.journal().entries().len();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1))],
            },
        )
        .expect("the Ogre attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks: the Ogre is unblocked, so the Bodyguard would take it");
    pass_until(
        &mut engine,
        priority_in(crate::turn::Step::DeclareBlockers, p1),
    );

    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![guard],
                players: vec![],
            },
        )
        .expect("the Bodyguard is a legal target");
    choose_monolith_source(&mut engine, p1, ogre);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        before_life - 2,
        "the Ogre's power: the Bodyguard's controller loses it after all"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        0,
        "the Bodyguard does not take it a second time"
    );
    assert_eq!(engine.state().per_turn.damage_dealt_to[1], 2);
    assert!(
        engine.state().shields.is_empty(),
        "the Monolith's shield is spent"
    );
    assert_eq!(
        damage_events(&engine, before_journal),
        vec![(ogre, crate::event::DamageTarget::Player(p1), 2, true)],
        "one hop: from the Ogre straight to the player, never marked on the Bodyguard along the way"
    );
}
