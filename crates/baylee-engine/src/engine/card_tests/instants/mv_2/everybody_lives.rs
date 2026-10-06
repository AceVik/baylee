//! `cards/instants/mv_2/everybody_lives.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "39213de3-6a4a-4879-a7f9-70f45013765e"

/// Everybody Lives! — {1}{W} Instant: "All creatures gain hexproof and
/// indestructible until end of turn. Players gain hexproof until end of turn.
/// Players can't lose life this turn and players can't lose the game or win
/// the game this turn."
///
/// A grant that lasts until end of turn is only worth playing in the turn it
/// refuses something, so the instant is cast in the **opponent's** main phase,
/// where their sorcery is waiting to be aimed: the Vindicate's target menu then
/// holds my artifacts and none of my creatures, while the Elf across the table
/// carries the same two keywords ("all creatures" and not my half of it). The
/// indestructible half is read off a destroy that *does* name my creature — my
/// own Despotic Scepter, which hexproof never shielded from its controller's
/// own ability — and the same ability aimed at a Plains takes the land down, so
/// the Elf left standing is the keyword's doing and not a destroy that quietly
/// never worked.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn everybody_lives_hides_and_arms_every_creature_through_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                despotic_scepter(),
                despotic_scepter(),
                llanowar_elves(),
                forest(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[everybody_lives()])
        .battlefield(1, &[llanowar_elves(), plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HEXPROOF),
        "nothing has granted anything yet"
    );

    // Into the opponent's main phase: the spell is an instant, and the turn it
    // has to refuse is the one whose sorcery is about to be aimed at my board.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the active seat opens its own main phase");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the instant is cast in the opponent's turn, so the seat holding it \
         needs priority: got {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p0, everybody_lives());
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });

    let granted = keywords(&engine, mine);
    assert!(
        granted.contains(KeywordSet::HEXPROOF) && granted.contains(KeywordSet::INDESTRUCTIBLE),
        "the creature under my own control has both printed keywords: {granted:?}"
    );
    let across = keywords(&engine, theirs);
    assert!(
        across.contains(KeywordSet::HEXPROOF) && across.contains(KeywordSet::INDESTRUCTIBLE),
        "\"All creatures\" is the whole table and not my half of it: {across:?}"
    );

    let scepters = all_on_battlefield(&engine, p0, despotic_scepter());
    assert_eq!(
        scepters.len(),
        2,
        "two Scepters, one for each destroy this scenario needs"
    );

    // The opponent's sorcery, aimed while the grant is live.
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        !options.contains(&mine),
        "hexproof is the opponents' word (CR 702.11b): the spell cast across the \
         table has no room for the Elf it was brought for: {options:?}"
    );
    assert!(
        options.contains(&scepters[0]) && options.contains(&scepters[1]),
        "and the board it declines to name is not an empty one — everything I \
         control that is no creature is still on that menu: {options:?}"
    );

    let doomed = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert!(
        options.contains(&doomed),
        "a land of mine is a permanent the menu really offers: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land was one of the options the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the Vindicate resolved against the one permanent it was allowed to name"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "and the Forest left the battlefield, so the cast was a real one"
    );

    // The indestructible half, read off a destroy hexproof never blocked: the
    // Scepter is mine, so my own creature is a permanent its controller owns.
    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Scepter destroys a permanent its controller owns, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own ability");
    assert!(
        options.contains(&mine),
        "hexproof shields a creature from its controller's *opponents* only, so \
         my own destroy may still name my own Elf: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"indestructible\": the destroy resolved and the Elf is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "and it was not put into a graveyard instead"
    );

    // The same ability aimed at something with no keyword at all: the destroy
    // really works, so the Elf above stood on indestructible.
    let lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(lands.len(), 2, "both Plains are still standing");
    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the second Scepter asks the same question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that paid the tap names the target");
    assert!(
        options.contains(&lands[0]),
        "a Plains is a permanent this seat owns: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lands[0]],
            },
        )
        .expect("the land the question offered is destroyed");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        1,
        "the destroy takes a land down, so nothing was wrong with the ability"
    );
    assert!(
        in_graveyard(&engine, p0, plains()).is_some(),
        "and the land is where a destroyed permanent goes"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf is still there beside it, which only the keyword can account for"
    );
}

/// Everybody Lives!' "Players can't lose life this turn", with the loss
/// coming from the other seat: the opponent casts both spells after the
/// instant has resolved, and both hit the opponent.
///
/// Vampiric Tutor's "You lose 2 life" is an effect whose `you` is not the
/// instant's controller. The check used to be keyed on whoever resolved
/// the loss rather than on who would lose it, so that loss went through
/// (#244). "Players" is the whole table. The Lightning Bolt is damage, and
/// damage makes a player lose life (CR 120.3a). The damage is still dealt,
/// but the loss is what the player can't have; damage was never checked at
/// all. The Bolt is aimed at its own caster because the instant also gave
/// every player hexproof, and that shuts out the other target.
#[test]
fn everybody_lives_keeps_every_life_total_whoever_resolves_the_loss() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[everybody_lives()])
        .battlefield(1, &[swamp(), mountain()])
        .hand(1, &[vampiric_tutor(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, everybody_lives());
    pass_until(&mut engine, stack_is_empty);

    // Priority passes across an empty stack in p0's main, to the seat
    // holding both instants.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, vampiric_tutor());
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, lightning_bolt()).is_some()
            && in_graveyard(&engine, p1, vampiric_tutor()).is_some(),
        "both spells resolved"
    );
    assert!(
        engine.state().journal.entries().iter().any(|e| matches!(
            e.event,
            crate::event::GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Player(hit),
                amount: 3,
                ..
            } if hit == p1
        )),
        "the Bolt's damage was dealt: only the loss is refused"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "three damage and the Tutor's two life, and not a point of either lost"
    );
    assert_eq!(
        engine.state().per_turn.life_lost,
        [false, false],
        "and nobody lost life this turn"
    );
}

/// The other half of "Players can't lose life": a player who can't lose
/// life can't pay it either (CR 119.8, "a cost that involves having that
/// player pay life can't be paid"). Three pay-life doors, one board, all
/// after the instant resolves:
///
/// - Mana Confluence's "{T}, Pay 1 life" is no longer offered. It is
///   offered before the instant, which is the control.
/// - The Black Gate's "pay 3 life or enter tapped" does not ask, because
///   the payment is not possible. It enters tapped.
/// - Toxic Deluge's "pay X life" offers X = 0 and nothing above it,
///   because paying 0 life is always possible (CR 119.4b).
#[test]
#[allow(clippy::too_many_lines)] // three payment doors on one board
fn everybody_lives_leaves_no_life_to_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mana_confluence(),
            ],
        )
        .hand(0, &[everybody_lives(), the_black_gate(), toxic_deluge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let confluence = on_battlefield(&engine, p0, mana_confluence()).expect("the Confluence is out");
    let offered = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal
            .abilities
            .iter()
            .any(|(source, _)| *source == confluence)
    };
    assert!(offered(&engine), "twenty life pays for the Confluence");

    tap_mana_except(&mut engine, p0, confluence);
    cast_with_floating(&mut engine, p0, everybody_lives());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !offered(&engine),
        "a player who can't lose life can't pay it, so the ability can't be activated"
    );

    let gate = in_hand(&engine, p0, the_black_gate()).expect("the Gate is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: gate })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no pay-or-tap question: there is nothing to pay with, got {:?}",
        engine.pending()
    );
    assert!(
        engine
            .state()
            .object(gate)
            .expect("the Gate is on the battlefield")
            .status
            .contains(Status::TAPPED),
        "so it enters tapped"
    );

    cast_with_floating(&mut engine, p0, toxic_deluge());
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 0),
        "no life to pay, and paying none is legal"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, toxic_deluge()).is_some(),
        "the Deluge was cast for X = 0 and resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and no life was paid anywhere"
    );
}

/// "Players can't lose the game" (Everybody Lives!) stops a pact's "you lose
/// the game" just as it stops a life total at zero: both are the game saying
/// a player loses, and an effect saying so (CR 104.3e) is one of them (#237).
///
/// Cast in the upkeep the debt comes due in, it keeps the seat in the game on
/// both roads the pact has. With the mana there the question is put and
/// declined; with a pool that cannot cover it the question never comes. Either
/// way the debt is simply gone, because the trigger that demanded it has
/// resolved.
#[test]
fn a_pact_left_unpaid_under_everybody_lives_costs_nothing() {
    let p0 = PlayerId::new(0);
    for has_the_mana in [true, false] {
        let board = if has_the_mana {
            let mut board = vec![island(); 8];
            board.extend([forest(), plains()]);
            board
        } else {
            vec![island(), forest(), plains()]
        };
        let mut engine = a_pact_owed_by_p0(&board, &[everybody_lives()]);
        let cast_turn = engine.state().turn.number;
        let (mut protected, mut asked) = (false, false);
        for _ in 0..600 {
            if protected && engine.state().turn.step != crate::turn::Step::Upkeep {
                break;
            }
            match engine.pending().clone() {
                Pending::GameOver(_) => break,
                Pending::YesNo { player, .. } => {
                    assert_eq!(player, p0, "the debt is the caster's");
                    asked = true;
                    engine
                        .apply(p0, PlayerAction::YesNo(false))
                        .expect("declining is an answer");
                }
                Pending::Priority { player, .. } => {
                    let turn = &engine.state().turn;
                    let the_upkeep_it_is_due = player == p0
                        && turn.active == p0
                        && turn.number > cast_turn
                        && turn.step == crate::turn::Step::Upkeep;
                    if the_upkeep_it_is_due && !protected {
                        tap_all_mana(&mut engine, p0);
                        cast_with_floating(&mut engine, p0, everybody_lives());
                        pass_until(&mut engine, stack_is_empty);
                        protected = true;
                        continue;
                    }
                    engine
                        .apply(player, PlayerAction::PassPriority)
                        .expect("passing priority is always legal");
                }
                Pending::ChooseAttackers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                        .unwrap();
                }
                other => panic!("unexpected on the way to the deferred upkeep cost: {other:?}"),
            }
        }
        assert!(protected, "the upkeep the debt is due in was never reached");
        assert_eq!(
            asked, !has_the_mana,
            "available mana pays the mandatory debt; otherwise ask before losing"
        );
        assert_eq!(
            engine.state().players[0].loss,
            None,
            "players can't lose the game this turn (has the mana: {has_the_mana})"
        );
        assert!(
            !matches!(engine.pending(), Pending::GameOver(_)),
            "and the game goes on"
        );
    }
}
