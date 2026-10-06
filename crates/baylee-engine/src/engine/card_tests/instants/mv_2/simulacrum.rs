//! `cards/instants/mv_2/simulacrum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Simulacrum's damage half — "deals damage to target creature you control"
/// — is a legal-target question of its own, answerable before any damage has
/// ever been dealt: the menu it offers is a fact about who controls what, not
/// about the amount involved.
#[test]
fn simulacrum_only_offers_a_creature_you_control_as_its_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .hand(0, &[simulacrum()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are seated");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bear is seated");

    cast_from_hand(&mut engine, p0, simulacrum());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Simulacrum asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![mine],
        "only a creature I control is a legal target, never one I don't ({theirs:?})"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![mine],
                players: vec![],
            },
        )
        .expect("my own creature was offered");
    pass_until(&mut engine, stack_is_empty);
}

/// "Target creature you control" with no creature on the caster's side of
/// the table: there is nothing the second half of the spell could ever
/// point at, so Simulacrum has no legal target at all and is refused
/// (CR 601.2 — unable to name a required target, the casting is illegal).
#[test]
fn simulacrum_needs_a_creature_you_control_to_be_cast() {
    let p0 = PlayerId::new(0);
    let simulacrum_card = simulacrum();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[simulacrum_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        2,
        "both Swamps stand ready: the withholding below is the missing \
         target, not an empty pool"
    );

    let card = in_hand(&engine, p0, simulacrum_card).expect("Simulacrum is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "with no creature to point its damage at, Simulacrum has no legal \
         target and is not offered"
    );
    assert!(
        matches!(
            engine.apply(p0, PlayerAction::CastSpell { card }),
            Err(EngineError::IllegalAction("not among the options"))
        ),
        "and naming it anyway is refused, not quietly allowed"
    );
}

/// "You gain life equal to the damage dealt to you this turn": the Bear (2)
/// and the Elephant (3) go through unblocked for 5, a Lightning Bolt later
/// the same turn adds 3 for 8 total, and a life payment and a life gain
/// injected between
/// the damage and the casting — neither of them damage — must not move the
/// number Simulacrum reads. Only the *increment* Simulacrum's own resolution
/// makes is asserted, which is exactly what the noise is there to insulate:
/// a version that read raw life lost this turn instead of the damage tally
/// would answer differently once life had been nudged by hand.
#[test]
fn simulacrum_gains_life_equal_to_all_the_damage_dealt_this_turn_no_matter_what_else_moved_the_life_total()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), tyrranax_rex()])
        .hand(0, &[simulacrum()])
        .battlefield(1, &[grizzly_bears(), wild_elephant(), mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);

    // p1's turn 2: the Bear (2) and the Elephant (3), unblocked, hit p0 for 5.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bear is seated");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("the Elephant is seated");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(p0)),
                    (elephant, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        5,
        "the Bear's 2 and the Elephant's 3, unblocked"
    );

    // Still p1's turn, in the combat damage step: a Lightning Bolt at p0 for
    // 3 more, off the Mountain kept back from the attack.
    aimed(&mut engine, p1, lightning_bolt(), &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        8,
        "5 from combat plus the Bolt's 3"
    );

    // Noise that is not damage, injected by hand: a life payment and a life
    // gain, neither of which may touch the tally Simulacrum reads.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    state.change_life(p0, -2, crate::event::Cause::DevCommand);
    state.change_life(p0, 5, crate::event::Cause::DevCommand);
    engine.refresh_offer();
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        8,
        "life lost or gained by hand does not touch the damage tally"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let target = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");
    let life_before_the_spell = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell + 8,
        "exactly the 5 combat and 3 burn damage dealt this turn, unmoved by \
         the life paid and the life gained that came between"
    );
    // Tyrranax Rex's own toughness is 8 (see the survives/destroyed test
    // below): the 8 Simulacrum just marked it with is exactly lethal, so it
    // dies here too — incidental to this test, which is about the life
    // total, not the target, but a claim worth checking rather than only
    // stating.
    assert!(
        in_graveyard(&engine, p0, tyrranax_rex()).is_some(),
        "8 marked on an 8-toughness creature is lethal (CR 704.5g)"
    );
}

/// The turn's tally starts over every turn (`per_turn.damage_dealt_to`): a
/// hit taken last turn buys nothing this turn, so Simulacrum cast in a
/// later, otherwise quiet turn gains no life and marks its target with no
/// damage either.
#[test]
fn simulacrum_does_not_count_damage_from_an_earlier_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), tyrranax_rex()])
        .hand(0, &[simulacrum()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);

    // p1's turn 2: the Elephant hits p0 for 3, unblocked.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("the Elephant is seated");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elephant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        3,
        "the Elephant's 3, unblocked"
    );
    let life_after_the_hit = engine.state().players[0].life;

    // Turn 3 opens on p0's own main phase, untouched by anything this turn.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().turn.number, 3, "a later turn than the hit");
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "a new turn's tally starts from nothing"
    );

    let target = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");
    cast_from_hand(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_after_the_hit,
        "no damage this turn: nothing gained, even though the Elephant hit \
         for 3 the turn before"
    );
    assert_eq!(
        engine.state().object(target).map(|o| o.damage),
        Some(0),
        "and none was dealt to the named creature either"
    );
}

/// "Simulacrum deals damage to target creature you control equal to the
/// damage dealt to you this turn": marked with exactly the 3 a single Bolt
/// puts on the books this turn, whichever of the caster's creatures is
/// named — enough to destroy a 1-toughness one (CR 704.5g) and not enough to
/// trouble an 8-toughness one, which is what tells "marked with that much"
/// apart from "however much it takes to kill it". The Rex iteration also
/// seats a bystander (Llanowar Elves) beside it: a reading of "each creature
/// you control" instead of "target creature you control" would mark that
/// one too, and Simulacrum itself is asserted as the damage's own source in
/// the journal.
#[test]
fn simulacrum_marks_its_target_with_the_same_damage_and_kills_it_only_if_thats_lethal() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (target_card, survives) in [(llanowar_elves(), false), (tyrranax_rex(), true)] {
        let mut board0 = vec![swamp(), swamp(), target_card];
        if survives {
            board0.push(llanowar_elves());
        }
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board0)
            .hand(0, &[simulacrum()])
            .battlefield(1, &[mountain()])
            .hand(1, &[lightning_bolt()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        engine.apply(p0, PlayerAction::PassPriority).unwrap();
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected p1 priority, got {:?}", engine.pending())
        };
        assert_eq!(player, p1);
        aimed(&mut engine, p1, lightning_bolt(), &[], &[p0]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().per_turn.damage_dealt_to[0],
            3,
            "the Bolt's 3, and nothing else has happened yet this turn"
        );

        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        );
        let target =
            on_battlefield(&engine, p0, target_card).expect("the target creature is seated");
        let spell = aimed(&mut engine, p0, simulacrum(), &[target], &[]);
        pass_until(&mut engine, stack_is_empty);

        assert!(
            engine.state().journal.entries().iter().any(|e| matches!(
                e.event,
                crate::event::GameEvent::DamageDealt {
                    source: Some(source),
                    target: crate::event::DamageTarget::Object(hit),
                    amount: 3,
                    is_combat: false,
                } if source == spell && hit == target
            )),
            "Simulacrum itself, not the Bolt, is this damage's source"
        );

        if survives {
            assert_eq!(
                engine.state().object(target).map(|o| o.damage),
                Some(3),
                "survives=true: marked with exactly the 3 dealt this turn"
            );
            assert!(
                on_battlefield(&engine, p0, target_card).is_some(),
                "3 is under an 8-toughness creature's toughness: it stands"
            );
            let bystander = on_battlefield(&engine, p0, llanowar_elves())
                .expect("the bystander Elves is seated too");
            assert_eq!(
                engine.state().object(bystander).map(|o| o.damage),
                Some(0),
                "\"target creature you control\", singular: the bystander \
                 took none of it"
            );
        } else {
            assert!(
                in_graveyard(&engine, p0, target_card).is_some(),
                "survives=false: 3 is at least a 1-toughness creature's \
                 toughness, so it is destroyed (CR 704.5g)"
            );
        }
    }
}

/// If Simulacrum's only target is gone by the time it would resolve, the
/// whole spell is removed from the stack instead — not even the untargeted
/// "you gain life" half happens (CR 608.2b), even though the turn's damage
/// tally is real and nonzero the whole time.
#[test]
fn simulacrum_does_nothing_if_its_only_target_is_gone_before_it_resolves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .hand(0, &[simulacrum()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    aimed(&mut engine, p1, lightning_bolt(), &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        3,
        "the Bolt's 3, on the books before Simulacrum is even cast"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let life_before_the_spell = engine.state().players[0].life;
    let spell = aimed(&mut engine, p0, simulacrum(), &[elf], &[]);

    // The Elves leave in response, before Simulacrum resolves: its only
    // target is now illegal.
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elf,
            ZoneLocation::Exile(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("the harness moves a card");
    engine.refresh_offer();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, simulacrum()).is_some(),
        "608.2b's own remedy for an all-illegal target: removed from the \
         stack and put in its owner's graveyard, same as a spell that \
         resolved normally — the graveyard alone does not say which \
         happened"
    );
    assert!(
        engine.state().journal.entries().iter().any(
            |e| matches!(e.event, GameEvent::StackObjectDidNotResolve { object } if object == spell)
        ),
        "the graveyard alone cannot tell CR 608.2b from a normal \
         resolution; the journal can"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell,
        "no legal target left to point the damage at means nothing \
         resolved at all, and the untargeted life gain never happened \
         either"
    );
}

/// Damage dealt to one of the caster's own creatures is not damage dealt to
/// *its controller*: a Bolt aimed at a Rootbreaker Wurm instead of at p0
/// leaves the turn's tally at 0, and Simulacrum, named at that same Wurm
/// afterward, gains nothing for it. (Not Tyrranax Rex here: its ward would
/// tax the very Bolt this test needs to land.)
#[test]
fn simulacrum_gains_nothing_when_a_bolt_hits_its_own_creature_instead_of_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), rootbreaker_wurm()])
        .hand(0, &[simulacrum()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let target = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is seated");
    aimed(&mut engine, p1, lightning_bolt(), &[target], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(target).map(|o| o.damage),
        Some(3),
        "the Bolt still marked the creature"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "damage dealt to a creature is not damage dealt to its controller"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let life_before_the_spell = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell,
        "the tally p0 was never dealt damage this turn stays 0: gains nothing"
    );
}

/// Only the opponent was dealt damage this turn: Simulacrum, cast by p0,
/// reads only p0's own tally — a stray 0, never the 3 sitting on p1's side
/// of the ledger.
#[test]
fn simulacrum_gains_nothing_when_only_the_opponent_was_dealt_damage_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), mountain(), tyrranax_rex()])
        .hand(0, &[simulacrum(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        3,
        "p1 took the Bolt"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "p0's own tally is untouched by damage dealt to the opponent"
    );

    let target = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");
    let life_before_the_spell = engine.state().players[0].life;
    cast_with_floating(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell,
        "0, even though the opponent took 3 this same turn"
    );
}

/// Simulacrum is black: White Knight's protection from black (CR 702.16b)
/// keeps it off the target menu even though its own controller is the one
/// casting the spell — Llanowar Elves, with no such protection, is the only
/// creature offered.
#[test]
fn simulacrum_never_offers_a_creature_protected_from_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), white_knight(), llanowar_elves()])
        .hand(0, &[simulacrum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    on_battlefield(&engine, p0, white_knight()).expect("White Knight is seated");

    cast_from_hand(&mut engine, p0, simulacrum());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Simulacrum asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![elf],
        "protection from black excludes White Knight even from its own \
         controller's black spell (CR 702.16b)"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elves, unprotected, is a legal target");
    pass_until(&mut engine, stack_is_empty);
}
