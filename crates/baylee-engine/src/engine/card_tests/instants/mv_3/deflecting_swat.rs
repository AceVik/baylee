//! `cards/instants/mv_3/deflecting_swat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deflecting Swat asks `{2}{R}` and none of it while its caster *controls* a
/// commander, and it hands them the target spell to aim somewhere else. Both
/// printed clauses are one scenario: p1 points Swords to Plowshares at p0's
/// Llanowar Elves, and p0 — four spent Swamps, no mana in the pool and a
/// commander standing on the battlefield — casts the Swat for free and turns
/// the Swords onto p1's own Umara Raptor. A card that charged `{2}{R}` would
/// be refused outright here, so the cast itself is the free-cost proof; a Swat
/// that resolved without asking for new targets would exile the Elves, so the
/// two creatures' zones say whether the redirection happened at all.
///
/// The commander is *played* rather than seated, because "you control a
/// commander" is a battlefield sentence (`casting::controls_a_commander`) and
/// one waiting in the command zone is not controlled — which is what the
/// first draft of this test assumed, and the empty `castable` list it got
/// back is exactly what that mistake looks like.
#[allow(clippy::too_many_lines)] // a commander cast, an opponent's spell, and the redirection of it
#[test]
fn deflecting_swat_redirects_a_spell_for_free_while_a_commander_stands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(977, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[plains(), umara_raptor()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The four Swamps pay the commander's {2}{B}{B} and nothing else: the
    // pool p0 answers p1's spell out of is empty.
    tap_all_mana(&mut engine, p0);
    let commander = engine
        .state()
        .zones
        .list(ZoneLocation::Command(p0))
        .first()
        .copied()
        .expect("Sheoldred starts in the command zone");
    engine
        .apply(p0, PlayerAction::CastSpell { card: commander })
        .expect("four Swamps pay {2}{B}{B}");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, sheoldred_the_apocalypse()).is_some() && stack_is_empty(e)
    });

    // p1 aims the Swords at the Elves — the play the Swat exists to undo.
    reach_their_main_phase(&mut engine, p1);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("the Raptor is out");
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares asks what it is aimed at, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "the creature across the table is a legal target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    // The caster takes priority back; hand it over so the seat with the Swat
    // gets to answer with the Swords standing on the stack.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!(
            "a spell on the stack hands priority back, got {:?}",
            engine.pending()
        )
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let swords = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swords_to_plowshares()))
        })
        .expect("the Swords is on the stack");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that is not casting gets to respond");
    let swat = in_hand(&engine, p0, deflecting_swat()).expect("the Swat is in hand");
    assert!(
        legal.castable.contains(&swat),
        "the commander on the battlefield is what offers the free cast, and \
         p0's pool is empty: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: swat })
        .expect("the Swat is cast for nothing");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let free = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
            .expect("the free alternative is one of the modes offered");
        engine.apply(p0, PlayerAction::ChooseMode(free)).unwrap();
    }

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Swat asks for a spell or ability, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&swords),
        "the Swords on the stack is what there is to turn: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords],
            },
        )
        .unwrap();

    // The Swat resolves, and the question it asks next is where the Swords
    // points now.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "\"you may choose new targets\": the Swat's caster"
    );
    assert!(
        options.contains(&raptor) && !options.contains(&elves),
        "the other creature is legal for the spell being aimed, and the Elves \
         stay by naming nothing, not by being offered again: {options:?}"
    );
    assert_eq!(min, 0, "\"you **may** choose new targets\" (CR 115.7d)");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raptor],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Swords was aimed away, so the Elves it was cast at are still there"
    );
    assert!(
        on_battlefield(&engine, p1, umara_raptor()).is_none(),
        "and the Raptor took the exile instead"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == umara_raptor()))),
        "\"exile target creature\": the redirected spell resolved, so the \
         redirection was a real change of target and not a fizzle"
    );
}

/// "You **may** choose new targets" (CR 115.7d): the Swat's caster may leave
/// the target where it is, and says so by naming nothing (#247). The question
/// used to ask for one target at least, so the only way to keep the Elves was
/// to be offered them again.
#[test]
fn deflecting_swat_may_leave_the_target_where_it_is() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[plains(), umara_raptor()])
        .hand(1, &[swords_to_plowshares()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("the Raptor is out");

    let swords = aimed(&mut engine, p1, swords_to_plowshares(), &[elves], &[]);
    let Some(Pending::ChooseTargets { options, min, .. }) = swatted(&mut engine, p0, swords) else {
        panic!("the Raptor is another legal target, so the Swat asks")
    };
    assert_eq!(
        (options, min),
        (vec![raptor], 0),
        "the other creature is offered, and naming nothing is an answer"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("\"may\": naming nothing leaves the target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && on_battlefield(&engine, p1, umara_raptor()).is_some(),
        "the Swords exiles the Elves it was cast at, and the Raptor stays"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "and the Elves' controller gains 1: the Swords resolved, unchanged"
    );
}

/// A limitation, pinned: Deflecting Swat on an **ability** leaves its target
/// alone (#249). A non-synthetic ability carries no `target_req`, so the
/// resolver cannot read what it may target without a card lookup, and asks
/// nothing. That is a legal answer under CR 115.7d, but the only one: Rod of
/// Ruin's shot at p0 could have gone to p1 or to either creature.
#[test]
fn deflecting_swat_leaves_an_abilitys_target_alone_until_249() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[forest(), forest(), forest(), rod_of_ruin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, rod_of_ruin(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("\"any target\": a player");
    let shot = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .copied()
        .expect("the Rod's ability is on the stack");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let asked = swatted(&mut engine, p0, shot);
    assert!(
        asked.is_none(),
        "#249: the Swat cannot read what the Rod's ability may target, so it \
         asks nothing; when #249 lands this asks, and this test moves: {asked:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (19, 20),
        "#249: the shot still lands where it was aimed"
    );
}

/// Deflecting Swat on a Curse of the Swine holding two targets (#247): each
/// target is asked about on its own, and either may stay (CR 115.7d). The
/// first moves to the Raptor, and the Elves the second names do not become
/// a choice for the first, since two instances of one target would break CR
/// 115.3. The second stays.
#[test]
fn deflecting_swat_moves_one_of_two_targets_and_leaves_the_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[mountain(), mountain(), mountain(), elf, elf])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[island(), island(), island(), island(), umara_raptor()])
        .hand(1, &[curse_of_the_swine()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let elves = all_on_battlefield(&engine, p0, elf);
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("the Raptor is out");
    cast_from_hand(&mut engine, p1, curse_of_the_swine());
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: elves.clone(),
            },
        )
        .expect("X = 2 names both Elves");
    let curse = on_stack(&engine, curse_of_the_swine()).expect("it is on the stack");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Some(Pending::ChooseTargets { options, min, .. }) = swatted(&mut engine, p0, curse) else {
        panic!("the Raptor is another legal target, so the Swat asks")
    };
    assert_eq!(min, 0);
    assert!(
        options.contains(&raptor) && options.contains(&elves[1]),
        "later targets are offered so a final legal swap is possible"
    );
    assert!(
        !options.contains(&elves[0]),
        "an empty answer keeps this slot's original target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raptor],
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        panic!(
            "the second target is asked about, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        (options, min),
        (vec![elves[0]], 0),
        "the second target: the Elves the first let go, and nothing the first \
         now names"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("the second stays");
    assert_eq!(
        engine
            .state()
            .object(curse)
            .expect("the Curse is on the stack")
            .targets
            .to_vec(),
        vec![raptor, elves[1]],
        "the first moved to the Raptor in its place, and the second stayed"
    );
}
